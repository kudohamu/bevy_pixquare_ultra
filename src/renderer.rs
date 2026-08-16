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
  data_type::{AnimationDirection, AnimationPlayState},
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
  pub play_state: AnimationPlayState,
}

impl Default for PxFrameAnimation {
  fn default() -> Self {
    Self {
      tag: None,
      duration: 0.1,
      direction: AnimationDirection::Forward,
      loop_count: 0,
      play_state: AnimationPlayState::Playing,
    }
  }
}

#[derive(Debug, Component)]
struct PxAnimationState {
  pub frame_index: u16,
  pub current_direction: AnimationDirection,
  pub temporary_direction: AnimationDirection,
  pub loop_count: u16,
  pub animation_timer: Option<Timer>,
  pub current_tag: Option<String>,
}

impl Default for PxAnimationState {
  fn default() -> Self {
    Self {
      frame_index: 0,
      current_direction: AnimationDirection::Forward,
      temporary_direction: AnimationDirection::Forward,
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
    (&mut T, Ref<PixquareFile>, &PxAnimationState),
    Or<(
      Changed<PixquareFile>,
      AssetChanged<PixquareFile>,
      Changed<PxAnimationState>,
    )>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut images: ResMut<Assets<Image>>,
  mut extra: <T as RenderPx>::Extra<'_>,
) {
  for (mut target, px, animation_state) in q_px.iter_mut() {
    let Some(artwork) = res_pxartworks.get(&px.artwork) else {
      continue;
    };

    match artwork
      .0
      .get_frame_image(animation_state.frame_index as usize, px.layer_visibility)
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

    let mut state = PxAnimationState {
      current_direction: frame_animation.direction,
      temporary_direction: initial_direction,
      frame_index: artwork.get_initial_frame_index(frame_animation.tag.clone(), initial_direction),
      ..default()
    };

    match &frame_animation.tag {
      Some(tag) => {
        if artwork.is_valid_tag(&tag) {
          state.current_tag = Some(tag.clone());
        }
      }
      None => {
        state.current_tag = None;
      }
    }

    commands.entity(entity).insert(state);
  }
}

fn detect_removed_animation_component(
  mut removed: RemovedComponents<PxFrameAnimation>,
  mut commands: Commands,
) {
  for entity in removed.read() {
    commands.entity(entity).remove::<PxAnimationState>();
  }
}

fn detect_updated_frame_animation_component(
  q_px: Query<(&PixquareFile, &PxFrameAnimation, &mut PxAnimationState), Changed<PxFrameAnimation>>,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, frame_animation, mut animation_state) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let is_tag_changed = animation_state.current_tag != frame_animation.tag
      && frame_animation
        .tag
        .clone()
        .is_none_or(|tag| artwork.is_valid_tag(&tag));

    if is_tag_changed {
      animation_state.frame_index = artwork.get_initial_frame_index(
        frame_animation.tag.clone(),
        animation_state.temporary_direction,
      );
      animation_state.current_tag = frame_animation.tag.clone();
    }

    if animation_state.current_direction != frame_animation.direction || is_tag_changed {
      let initial_direction = match frame_animation.direction {
        AnimationDirection::Forward => AnimationDirection::Forward,
        AnimationDirection::Backward => AnimationDirection::Backward,
        AnimationDirection::PingPong => AnimationDirection::Forward,
      };

      animation_state.current_direction = frame_animation.direction;
      animation_state.temporary_direction = initial_direction;
      animation_state.loop_count = 0;
    }
  }
}

fn update_frame_index(
  q_px: Query<(&PixquareFile, &mut PxFrameAnimation, &mut PxAnimationState)>,
  res_pxartworks: Res<Assets<PxArtwork>>,
  time: Res<Time>,
) {
  for (px_file, mut frame_animation, mut animation_state) in q_px {
    if frame_animation.play_state == AnimationPlayState::Paused {
      continue;
    }
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let timer = match animation_state.animation_timer.as_mut() {
      Some(timer) => timer,
      None => {
        let timer = Timer::from_seconds(frame_animation.duration, TimerMode::Once);
        animation_state.animation_timer = Some(timer);

        animation_state.animation_timer.as_mut().unwrap()
      }
    };

    timer.tick(time.delta());

    if timer.just_finished() {
      if frame_animation.loop_count != 0 {
        if animation_state.loop_count >= frame_animation.loop_count {
          frame_animation.play_state = AnimationPlayState::Paused;
          continue;
        }
        if animation_state.frame_index as usize >= artwork.0.frames_len() - 1 {
          animation_state.loop_count += 1;
        }
      }

      let next_frame_index = frame_animation.next_frame(&artwork, &animation_state);
      animation_state.frame_index = next_frame_index;
      animation_state.animation_timer = None;

      if frame_animation.direction == AnimationDirection::PingPong {
        let range = artwork.get_tag_range(animation_state.current_tag.clone());

        if animation_state.temporary_direction == AnimationDirection::Forward
          && animation_state.frame_index >= range.end - 1
        {
          animation_state.temporary_direction = AnimationDirection::Backward;
        }

        if animation_state.temporary_direction == AnimationDirection::Backward
          && animation_state.frame_index == range.start
        {
          animation_state.temporary_direction = AnimationDirection::Forward;
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
  fn get_tag_range(&self, tag: Option<String>) -> Range<u16> {
    match tag {
      Some(tag) => {
        let Some(tag) = self.0.tags.iter().find(|t| t.name == tag) else {
          warn!("tag: `{}` is not found", tag);

          return 0..self.0.frames_len() as u16;
        };

        tag.start_index..(tag.end_index + 1)
      }
      None => {
        return 0..self.0.frames_len() as u16;
      }
    }
  }

  fn get_initial_frame_index(&self, tag: Option<String>, direction: AnimationDirection) -> u16 {
    let range = self.get_tag_range(tag);

    match direction {
      AnimationDirection::Forward => range.start,
      AnimationDirection::Backward => range.end - 1,
      AnimationDirection::PingPong => range.start,
    }
  }

  fn is_valid_tag(&self, tag: &str) -> bool {
    self.0.tags.iter().position(|t| t.name == tag).is_some()
  }
}

impl PxFrameAnimation {
  fn next_frame(&self, artwork: &PxArtwork, animation_state: &PxAnimationState) -> u16 {
    let delta: i16 = if animation_state.temporary_direction == AnimationDirection::Forward {
      1
    } else {
      -1
    };

    let index_range = artwork.get_tag_range(animation_state.current_tag.clone());
    let min = index_range.start as i16;
    let max = index_range.end as i16;

    if max == min {
      return min as u16;
    }
    min as u16 + (animation_state.frame_index as i16 - min + delta).rem_euclid(max - min) as u16
  }
}

#[cfg(test)]
mod tests {
  use std::{assert_eq, time::Duration};

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

  fn get_artwork(app: &App, entity: Entity) -> &Artwork {
    let px_file = app.world().entity(entity).get::<PixquareFile>().unwrap();
    let res_pxartwork = app.world().get_resource::<Assets<PxArtwork>>().unwrap();
    let artwork = &res_pxartwork.get(&px_file.artwork).unwrap().0;

    return artwork;
  }

  fn animation_state(app: &App, entity: Entity) -> &PxAnimationState {
    app
      .world()
      .entity(entity)
      .get::<PxAnimationState>()
      .unwrap()
  }

  fn set_play_state(app: &mut App, entity: Entity, play_state: AnimationPlayState) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxFrameAnimation>()
      .unwrap()
      .play_state = play_state;
  }

  fn set_tag(app: &mut App, entity: Entity, tag: Option<String>) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxFrameAnimation>()
      .unwrap()
      .tag = tag;
  }

  fn set_direction(app: &mut App, entity: Entity, direction: AnimationDirection) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxFrameAnimation>()
      .unwrap()
      .direction = direction;
  }

  fn set_frame_index(app: &mut App, entity: Entity, frame_index: u16) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationState>()
      .unwrap()
      .frame_index = frame_index;
  }

  fn set_current_direction(app: &mut App, entity: Entity, direction: AnimationDirection) {
    let initial_direction = match direction {
      AnimationDirection::Forward => AnimationDirection::Forward,
      AnimationDirection::Backward => AnimationDirection::Backward,
      AnimationDirection::PingPong => AnimationDirection::Forward,
    };

    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationState>()
      .unwrap()
      .current_direction = direction;

    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationState>()
      .unwrap()
      .temporary_direction = initial_direction;
  }

  fn set_loop_count(app: &mut App, entity: Entity, loop_count: u16) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAnimationState>()
      .unwrap()
      .loop_count = loop_count;
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

    assert_eq!(animation_state(&app, entity).frame_index, 1);
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
      .get_mut::<PxAnimationState>()
      .unwrap()
      .frame_index = frames_len - 2;

    app.update();

    let state = app
      .world()
      .entity(entity)
      .get::<PxAnimationState>()
      .unwrap();

    assert_eq!(state.frame_index, frames_len - 1);
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
      .get_mut::<PxAnimationState>()
      .unwrap()
      .frame_index = 1;

    app.update();

    let state = app
      .world()
      .entity(entity)
      .get::<PxAnimationState>()
      .unwrap();

    assert_eq!(state.frame_index, 0);
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

    set_frame_index(&mut app, entity, 0);

    app.update();

    let state = animation_state(&app, entity);
    let artwork = get_artwork(&app, entity);
    assert_eq!(state.frame_index, artwork.frames_len() as u16 - 1);
  }

  #[test]
  fn test_reverses_to_backward_at_last_frame_when_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    let artwork = get_artwork(&app, entity);
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

    set_frame_index(&mut app, entity, frames_len - 2);
    app.update();

    let state = app
      .world()
      .entity(entity)
      .get::<PxAnimationState>()
      .unwrap();

    assert_eq!(state.frame_index, frames_len - 1);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
    assert_eq!(state.current_direction, AnimationDirection::PingPong);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, frames_len - 2);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
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

    set_frame_index(&mut app, entity, 1);
    set_current_direction(&mut app, entity, AnimationDirection::Backward);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 0);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 1);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
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

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 0);
  }

  #[test]
  fn test_starts_at_tag_first_frame_when_tag_direction_is_forward() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
    assert_eq!(state.current_tag.clone().unwrap(), "front_move");
  }

  #[test]
  fn test_starts_at_tag_last_frame_when_tag_direction_is_backward() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 5);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
    assert_eq!(state.current_tag.clone().unwrap(), "front_move");
  }

  #[test]
  fn test_starts_at_tag_first_frame_when_tag_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Paused,
        ..default()
      });

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
    assert_eq!(state.current_tag.clone().unwrap(), "front_move");
  }

  #[test]
  fn test_wraps_to_tag_first_frame_when_tag_direction_is_forward() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    app.update();

    set_frame_index(&mut app, entity, 5);
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();

    assert_eq!(animation_state(&app, entity).frame_index, 4);
  }

  #[test]
  fn test_wraps_to_tag_last_frame_when_tag_direction_is_backward() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    app.update();

    set_frame_index(&mut app, entity, 4);
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();

    assert_eq!(animation_state(&app, entity).frame_index, 5);
  }

  #[test]
  fn test_reverses_at_tag_last_frame_when_tag_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Playing,
        ..default()
      });
    app.update();

    set_frame_index(&mut app, entity, 4);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 5);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 5);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
  }

  #[test]
  fn test_stays_on_single_frame_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Playing,
        ..default()
      });

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 3);
    assert_eq!(state.current_tag.clone().unwrap(), "front");
  }

  #[test]
  fn test_resets_animation_status_when_switching_to_valid_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });

    app.update();

    set_frame_index(&mut app, entity, 5);
    set_current_direction(&mut app, entity, AnimationDirection::Backward);
    set_loop_count(&mut app, entity, 7);

    set_tag(&mut app, entity, Some("right_move".into()));
    set_direction(&mut app, entity, AnimationDirection::Forward);

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 8);
    assert_eq!(state.current_tag.clone().unwrap(), "right_move");
    assert_eq!(state.loop_count, 0);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
  }

  #[test]
  fn test_preserves_active_tag_and_status_when_switching_to_missing_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: Some("front_move".into()),
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Playing,
        ..default()
      });
    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 5);

    set_loop_count(&mut app, entity, 7);
    set_tag(&mut app, entity, Some("invalid".into()));
    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.current_tag.clone().unwrap(), "front_move");
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.loop_count, 7);
  }

  #[test]
  fn test_uses_all_frames_when_missing_tag_is_selected_without_active_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        tag: None,
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    app.update();

    set_frame_index(&mut app, entity, 8);
    set_current_direction(&mut app, entity, AnimationDirection::Forward);
    set_loop_count(&mut app, entity, 7);
    set_tag(&mut app, entity, Some("invalid".into()));

    app.update();

    let state = animation_state(&app, entity);
    assert_eq!(state.frame_index, 8);
    assert!(state.current_tag.is_none());
    assert_eq!(state.loop_count, 7);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    set_play_state(&mut app, entity, AnimationPlayState::Playing);
    app.update();

    assert_eq!(animation_state(&app, entity).frame_index, 0);
  }
}
