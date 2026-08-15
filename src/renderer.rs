use std::ops::Range;

use bevy::{
  app::{Plugin, PostUpdate},
  asset::{AsAssetId, Assets, Handle, RenderAssetUsages},
  ecs::{
    component::{Component, Mutable},
    entity::Entity,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::Ref,
  },
  image::{Image, ImageSampler},
  log::{error, warn},
  prelude::AssetChanged,
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
  sprite::Sprite,
  time::{Time, Timer, TimerMode},
  utils::default,
};
use pixquare::utility_type::LayerVisibility;

use crate::{
  data_type::{AnimationDirection, AnimationState},
  loader::PxArtwork,
};

#[derive(Debug, Component)]
pub struct PixquareFile {
  pub artwork: Handle<PxArtwork>,
  pub layer_visibility: LayerVisibility,
}

impl Default for PixquareFile {
  fn default() -> Self {
    Self {
      artwork: Handle::default(),
      layer_visibility: LayerVisibility::Visible,
    }
  }
}

impl AsAssetId for PixquareFile {
  type Asset = PxArtwork;

  fn as_asset_id(&self) -> bevy::asset::AssetId<Self::Asset> {
    self.artwork.id()
  }
}

#[derive(Debug, Component)]
pub struct PxFrameAnimation {
  pub tag: Option<String>,
  pub duration: f32,
  pub direction: AnimationDirection,
  pub loop_count: u16,
  pub play_state: AnimationState,
}

impl Default for PxFrameAnimation {
  fn default() -> Self {
    Self {
      tag: None,
      duration: 0.1,
      direction: AnimationDirection::Forward,
      loop_count: 0,
      play_state: AnimationState::Playing,
    }
  }
}

#[derive(Debug, Component)]
struct PxAnimationStatus {
  pub frame_index: u16,
  pub current_direction: AnimationDirection,
  pub loop_count: u16,
  pub animation_timer: Option<Timer>,
  pub current_tag: Option<String>,
}

impl Default for PxAnimationStatus {
  fn default() -> Self {
    Self {
      frame_index: 0,
      current_direction: AnimationDirection::Forward,
      loop_count: 0,
      animation_timer: None,
      current_tag: None,
    }
  }
}

trait RenderPx {
  type Extra<'e>;

  fn render_px(&mut self, texture: Handle<Image>, _extra: &mut Self::Extra<'_>);
}

impl RenderPx for Sprite {
  type Extra<'e> = ();

  fn render_px(&mut self, texture: Handle<Image>, _extra: &mut Self::Extra<'_>) {
    self.image = texture;
  }
}

fn render<T: RenderPx + Component<Mutability = Mutable>>(
  mut q_px: Query<
    (&mut T, Ref<PixquareFile>, &PxAnimationStatus),
    Or<(
      Changed<PixquareFile>,
      AssetChanged<PixquareFile>,
      Changed<PxAnimationStatus>,
    )>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut images: ResMut<Assets<Image>>,
  mut extra: <T as RenderPx>::Extra<'_>,
) {
  for (mut target, px, animation_status) in q_px.iter_mut() {
    let Some(artwork) = res_pxartworks.get(&px.artwork) else {
      continue;
    };

    match artwork
      .0
      .get_frame_image(animation_status.frame_index as usize, px.layer_visibility)
    {
      Ok(image_buf) => {
        let mut image = Image::new(
          Extent3d {
            width: artwork.0.canvas_size.width,
            height: artwork.0.canvas_size.height,
            depth_or_array_layers: 1,
          },
          TextureDimension::D2,
          image_buf,
          TextureFormat::Rgba8UnormSrgb,
          RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();

        let texture_handle = images.add(image);
        target.render_px(texture_handle, &mut extra);
      }
      Err(err) => {
        error!("{}", err);
      }
    }
  }
}

fn detect_added_animation_component(
  mut commands: Commands,
  q_px: Query<(Entity, &PixquareFile, &PxFrameAnimation), Added<PxFrameAnimation>>,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (entity, px_file, frame_animation) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      warn!("the artwork file is not loaded yet. please preload the artwork file");
      continue;
    };

    let initial_direction = match frame_animation.direction {
      AnimationDirection::Forward => AnimationDirection::Forward,
      AnimationDirection::Backward => AnimationDirection::Backward,
      AnimationDirection::PingPong => AnimationDirection::Forward,
    };
    let frame_index: u16 = if let Some(tag) = &frame_animation.tag {
      let tag_range = artwork.get_tag_range(&tag);

      tag_range.start
    } else {
      0
    };

    let status = PxAnimationStatus {
      current_direction: initial_direction,
      frame_index,
      current_tag: frame_animation.tag.clone(),
      ..default()
    };
    commands.entity(entity).insert(status);
  }
}

fn detect_removed_animation_component(
  mut removed: RemovedComponents<PxFrameAnimation>,
  mut commands: Commands,
) {
  for entity in removed.read() {
    commands.entity(entity).remove::<PxAnimationStatus>();
  }
}

fn detect_updated_frame_animation_component(
  q_px: Query<
    (&PixquareFile, &PxFrameAnimation, &mut PxAnimationStatus),
    Changed<PxFrameAnimation>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, frame_animation, mut status) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    if status.current_tag != frame_animation.tag {
      let frame_index: u16 = if let Some(tag) = &frame_animation.tag {
        let tag_range = artwork.get_tag_range(&tag);

        tag_range.start
      } else {
        0
      };

      status.frame_index = frame_index;
      status.current_tag = frame_animation.tag.clone();
    }
  }
}

fn update_frame_index(
  q_px: Query<(&PixquareFile, &mut PxFrameAnimation, &mut PxAnimationStatus)>,
  res_pxartworks: Res<Assets<PxArtwork>>,
  time: Res<Time>,
) {
  for (px_file, mut frame_animation, mut animation_status) in q_px {
    if frame_animation.play_state == AnimationState::Paused {
      continue;
    }
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let timer = match animation_status.animation_timer.as_mut() {
      Some(timer) => timer,
      None => {
        let timer = Timer::from_seconds(frame_animation.duration, TimerMode::Once);
        animation_status.animation_timer = Some(timer);

        animation_status.animation_timer.as_mut().unwrap()
      }
    };

    timer.tick(time.delta());

    if timer.just_finished() {
      if frame_animation.loop_count != 0 {
        if animation_status.loop_count >= frame_animation.loop_count {
          frame_animation.play_state = AnimationState::Paused;
          continue;
        }
        if animation_status.frame_index as usize >= artwork.0.frames_len() - 1 {
          animation_status.loop_count += 1;
        }
      }

      let next_frame_index = frame_animation.next_frame(&artwork, &animation_status);
      animation_status.frame_index = next_frame_index;
      animation_status.animation_timer = None;

      if frame_animation.direction == AnimationDirection::PingPong {
        if animation_status.current_direction == AnimationDirection::Forward
          && animation_status.frame_index as usize >= artwork.0.frames_len() - 1
        {
          animation_status.current_direction = AnimationDirection::Backward;
        }

        if animation_status.current_direction == AnimationDirection::Backward
          && animation_status.frame_index == 0
        {
          animation_status.current_direction = AnimationDirection::Forward;
        }
      }
    }
  }
}

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_systems(
      PostUpdate,
      (
        (
          detect_added_animation_component,
          detect_removed_animation_component,
          detect_updated_frame_animation_component,
        ),
        (render::<Sprite>, update_frame_index),
      )
        .chain(),
    );
  }
}

impl PxArtwork {
  fn get_tag_range(&self, tag: &str) -> Range<u16> {
    let Some(tag) = self.0.tags.iter().find(|t| t.name == tag) else {
      warn!("tag: `{}` is not found", tag);
      let frames_len = self.0.frames_len();

      return 0..frames_len as u16;
    };

    tag.start_index..(tag.end_index + 1)
  }
}

impl PxFrameAnimation {
  fn next_frame(&self, artwork: &PxArtwork, status: &PxAnimationStatus) -> u16 {
    let delta: i16 = if status.current_direction == AnimationDirection::Forward {
      1
    } else {
      -1
    };

    let index_range = if let Some(tag) = &self.tag {
      artwork.get_tag_range(&tag)
    } else {
      0..(artwork.0.frames_len() as u16 - 1)
    };

    let min = index_range.start as i16;
    let max = index_range.end as i16;

    min as u16 + (status.frame_index as i16 - min + delta).rem_euclid(max - min) as u16
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use bevy::{
    app::App,
    asset::Assets,
    ecs::entity::Entity,
    image::Image,
    sprite::Sprite,
    time::{TimePlugin, TimeUpdateStrategy},
  };
  use pixquare::model::Artwork;

  use super::*;

  const FRAME_DURATION: Duration = Duration::from_millis(100);
  const TIME_STEP: Duration = Duration::from_millis(101);

  fn create_px_file_app(path: &str) -> (App, Entity) {
    let mut app = App::new();
    app
      .add_plugins((TimePlugin, PixquareRendererPlugin))
      .insert_resource(TimeUpdateStrategy::ManualDuration(TIME_STEP))
      .insert_resource(Assets::<PxArtwork>::default())
      .insert_resource(Assets::<Image>::default());

    app.update();

    let file_data = std::fs::read(path).unwrap();
    let artwork = Artwork::read(&file_data).unwrap();

    let artwork = app
      .world_mut()
      .resource_mut::<Assets<PxArtwork>>()
      .add(PxArtwork(artwork.clone()));

    let entity = app
      .world_mut()
      .spawn((
        PixquareFile {
          artwork,
          ..Default::default()
        },
        Sprite::default(),
      ))
      .id();

    (app, entity)
  }

  #[test]
  fn test_advances_to_next_frame_when_entity_has_px_animation_frame_component() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, 1);
  }

  #[test]
  fn test_wraps_to_first_frame_when_current_frame_is_last() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    let px_file = app.world().entity(entity).get::<PixquareFile>().unwrap();
    let res_pxartwork = app.world().get_resource::<Assets<PxArtwork>>().unwrap();
    let artwork = &res_pxartwork.get(&px_file.artwork).unwrap().0;
    let frames_len = artwork.frames_len() as u16;

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationStatus>()
      .unwrap()
      .frame_index = frames_len - 2;

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, frames_len - 1);
  }

  #[test]
  fn test_advance_to_previous_frame_when_direction_is_backward() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationStatus>()
      .unwrap()
      .frame_index = 1;

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, 0);
  }

  #[test]
  fn test_wraps_to_last_frame_when_direction_is_backward_and_current_frame_is_first() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    let px_file = app.world().entity(entity).get::<PixquareFile>().unwrap();
    let res_pxartwork = app.world().get_resource::<Assets<PxArtwork>>().unwrap();
    let artwork = &res_pxartwork.get(&px_file.artwork).unwrap().0;

    assert_eq!(status.frame_index, artwork.frames_len() as u16 - 1);
  }

  #[test]
  fn test_reverses_to_backward_at_last_frame_when_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    let px_file = app.world().entity(entity).get::<PixquareFile>().unwrap();
    let res_pxartwork = app.world().get_resource::<Assets<PxArtwork>>().unwrap();
    let artwork = &res_pxartwork.get(&px_file.artwork).unwrap().0;
    let frames_len = artwork.frames_len() as u16;

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    let mut entity_mut = app.world_mut().entity_mut(entity);
    let mut status = entity_mut.get_mut::<PxAnimationStatus>().unwrap();
    status.frame_index = frames_len - 2;

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, frames_len - 1);
    assert_eq!(status.current_direction, AnimationDirection::Backward);

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, frames_len - 2);
    assert_eq!(status.current_direction, AnimationDirection::Backward);
  }

  #[test]
  fn test_reverses_to_forward_at_first_frame_when_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    let mut entity_mut = app.world_mut().entity_mut(entity);
    let mut status = entity_mut.get_mut::<PxAnimationStatus>().unwrap();
    status.frame_index = 1;
    status.current_direction = AnimationDirection::Backward;

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, 0);
    assert_eq!(status.current_direction, AnimationDirection::Forward);

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, 1);
    assert_eq!(status.current_direction, AnimationDirection::Forward);
  }

  #[test]
  fn test_stays_on_first_frame_when_ping_pong_artwork_has_one_frame() {
    let (mut app, entity) = create_px_file_app(&"assets/orange.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    let status = app
      .world()
      .entity(entity)
      .get::<PxAnimationStatus>()
      .unwrap();

    assert_eq!(status.frame_index, 0);
  }
}
