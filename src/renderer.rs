use std::ops::Range;

use bevy::{
  app::{Plugin, PostUpdate},
  asset::{AsAssetId, Assets, Handle, RenderAssetUsages},
  ecs::{
    component::{Component, Mutable},
    entity::Entity,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Or, With, Without},
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
};
use pixquare::utility_type::LayerVisibility;

use crate::{
  data_type::{AnimationDirection, AnimationPlayState},
  event::PixquareFileInitializedEvent,
  loader::PxArtwork,
};

#[derive(Debug, Component)]
#[require(PxState, PendingPxInitialization)]
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
  pub duration: f32,
  pub direction: AnimationDirection,
  pub loop_count: u16,
  pub play_state: AnimationPlayState,
}

impl Default for PxFrameAnimation {
  fn default() -> Self {
    Self {
      duration: 0.1,
      direction: AnimationDirection::Forward,
      loop_count: 0,
      play_state: AnimationPlayState::Playing,
    }
  }
}

#[derive(Debug, Component)]
pub struct PxTag(pub String);

impl PxTag {
  pub fn new(str: String) -> Self {
    Self(str)
  }
}

#[derive(Debug, Component)]
struct PxState {
  pub artwork_id: Option<bevy::asset::AssetId<PxArtwork>>,
  pub frame_index: u16,
  pub current_direction: AnimationDirection,
  pub temporary_direction: AnimationDirection,
  pub loop_count: u16,
  pub animation_timer: Option<Timer>,
  pub current_tag: Option<String>,
}

impl Default for PxState {
  fn default() -> Self {
    Self {
      artwork_id: None,
      frame_index: 0,
      current_direction: AnimationDirection::Forward,
      temporary_direction: AnimationDirection::Forward,
      loop_count: 0,
      animation_timer: None,
      current_tag: None,
    }
  }
}

impl PxState {
  fn next_frame(&self, artwork: &PxArtwork) -> u16 {
    let delta: i16 = if self.temporary_direction == AnimationDirection::Forward {
      1
    } else {
      -1
    };

    let index_range = artwork.get_tag_range(&self.current_tag);
    let min = index_range.start as i16;
    let max = index_range.end as i16;

    if max == min {
      return min as u16;
    }
    min as u16 + (self.frame_index as i16 - min + delta).rem_euclid(max - min) as u16
  }
}

#[derive(Component, Default)]
struct PendingPxInitialization;

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

impl PxArtwork {
  fn get_tag_range(&self, tag: &Option<String>) -> Range<u16> {
    match tag {
      Some(tag) => {
        let Some(tag) = self.0.tags.iter().find(|t| t.name == *tag) else {
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

  fn get_initial_frame_index(&self, tag: &Option<String>, direction: AnimationDirection) -> u16 {
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

fn render<T: RenderPx + Component<Mutability = Mutable>>(
  mut q_px: Query<
    (&mut T, Ref<PixquareFile>, &PxState),
    (
      Or<(
        Changed<PixquareFile>,
        AssetChanged<PixquareFile>,
        Changed<PxState>,
      )>,
      Without<PendingPxInitialization>,
    ),
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut images: ResMut<Assets<Image>>,
  mut extra: <T as RenderPx>::Extra<'_>,
) {
  for (mut target, px_file, px_state) in q_px.iter_mut() {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    match artwork
      .0
      .get_frame_image(px_state.frame_index as usize, px_file.layer_visibility)
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

fn initialize_pending_px_files(
  mut commands: Commands,
  mut q_px: Query<
    (
      Entity,
      &PixquareFile,
      Option<&PxTag>,
      Option<&PxFrameAnimation>,
      &mut PxState,
    ),
    With<PendingPxInitialization>,
  >,
  artworks: Res<Assets<PxArtwork>>,
) {
  for (entity, px_file, px_tag, frame_animation, mut px_state) in &mut q_px {
    let Some(artwork) = artworks.get(&px_file.artwork) else {
      continue;
    };

    let current_tag = px_tag.map_or(None, |tag| {
      if artwork.is_valid_tag(&tag.0) {
        return Some(tag.0.clone());
      }
      None
    });

    px_state.current_tag = current_tag;

    let current_direction = frame_animation.map_or(AnimationDirection::Forward, |f| f.direction);
    let initial_direction = match current_direction {
      AnimationDirection::Forward => AnimationDirection::Forward,
      AnimationDirection::Backward => AnimationDirection::Backward,
      AnimationDirection::PingPong => AnimationDirection::Forward,
    };

    px_state.current_direction = current_direction;
    px_state.temporary_direction = initial_direction;
    px_state.artwork_id = Some(px_file.artwork.id());
    px_state.frame_index =
      artwork.get_initial_frame_index(&px_state.current_tag, initial_direction);
    px_state.loop_count = 0;
    px_state.animation_timer = None;

    commands.entity(entity).remove::<PendingPxInitialization>();
    commands.trigger(PixquareFileInitializedEvent(entity));
  }
}

fn mark_changed_px_files_as_pending(
  mut commands: Commands,
  query: Query<(Entity, &PixquareFile, &PxState), Changed<PixquareFile>>,
) {
  for (entity, px_file, px_state) in &query {
    if px_state.artwork_id != Some(px_file.artwork.id()) {
      commands.entity(entity).insert(PendingPxInitialization);
    }
  }
}

fn detect_added_animation_component(
  q_px: Query<
    (
      &PixquareFile,
      Option<&PxTag>,
      &PxFrameAnimation,
      &mut PxState,
    ),
    (Without<PendingPxInitialization>, Added<PxFrameAnimation>),
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, px_tag, frame_animation, mut px_state) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let initial_direction = match frame_animation.direction {
      AnimationDirection::Forward => AnimationDirection::Forward,
      AnimationDirection::Backward => AnimationDirection::Backward,
      AnimationDirection::PingPong => AnimationDirection::Forward,
    };

    px_state.current_direction = frame_animation.direction;
    px_state.temporary_direction = initial_direction;
    px_state.frame_index =
      artwork.get_initial_frame_index(&px_tag.map(|t| t.0.clone()), initial_direction);
  }
}

fn detect_removed_animation_component(
  mut removed: RemovedComponents<PxFrameAnimation>,
  mut q_px: Query<(&PixquareFile, Option<&PxTag>, &mut PxState), Without<PendingPxInitialization>>,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for entity in removed.read() {
    let Ok((px_file, px_tag, mut px_state)) = q_px.get_mut(entity) else {
      continue;
    };
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    px_state.frame_index =
      artwork.get_initial_frame_index(&px_tag.map(|t| t.0.clone()), AnimationDirection::Forward);
    px_state.current_direction = AnimationDirection::Forward;
    px_state.temporary_direction = AnimationDirection::Forward;
  }
}

fn detect_updated_frame_animation_component(
  q_px: Query<
    (&PixquareFile, &PxFrameAnimation, &mut PxState),
    (Without<PendingPxInitialization>, Changed<PxFrameAnimation>),
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, frame_animation, mut px_state) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    if px_state.current_direction != frame_animation.direction {
      let initial_direction = match frame_animation.direction {
        AnimationDirection::Forward => AnimationDirection::Forward,
        AnimationDirection::Backward => AnimationDirection::Backward,
        AnimationDirection::PingPong => AnimationDirection::Forward,
      };

      px_state.current_direction = frame_animation.direction;
      px_state.temporary_direction = initial_direction;
      px_state.loop_count = 0;

      px_state.frame_index =
        artwork.get_initial_frame_index(&px_state.current_tag, px_state.temporary_direction);
    }
  }
}

fn detect_added_or_updated_tag_component(
  q_px: Query<
    (
      &PixquareFile,
      &PxTag,
      &mut PxState,
      Option<&PxFrameAnimation>,
    ),
    (
      Without<PendingPxInitialization>,
      Or<(Added<PxTag>, Changed<PxTag>)>,
    ),
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, px_tag, mut px_state, frame_animation) in q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };
    if !artwork.is_valid_tag(&px_tag.0) {
      continue;
    }
    if px_state
      .current_tag
      .as_ref()
      .is_some_and(|tag| px_tag.0 == *tag)
    {
      continue;
    }

    px_state.current_tag = Some(px_tag.0.clone());
    px_state.frame_index =
      artwork.get_initial_frame_index(&px_state.current_tag, px_state.temporary_direction);

    if let Some(frame_animation) = frame_animation {
      let initial_direction = match frame_animation.direction {
        AnimationDirection::Forward => AnimationDirection::Forward,
        AnimationDirection::Backward => AnimationDirection::Backward,
        AnimationDirection::PingPong => AnimationDirection::Forward,
      };

      px_state.current_direction = frame_animation.direction;
      px_state.temporary_direction = initial_direction;
      px_state.loop_count = 0;
    }
  }
}

fn detect_removed_tag_component(
  mut removed: RemovedComponents<PxTag>,
  mut q_px: Query<
    (&PixquareFile, &mut PxState, Option<&PxFrameAnimation>),
    Without<PendingPxInitialization>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for entity in removed.read() {
    let Ok((px_file, mut animation_state, frame_animation)) = q_px.get_mut(entity) else {
      continue;
    };
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    animation_state.frame_index =
      artwork.get_initial_frame_index(&None, animation_state.temporary_direction);
    animation_state.current_tag = None;

    if let Some(frame_animation) = frame_animation {
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
  q_px: Query<
    (&PixquareFile, &mut PxFrameAnimation, &mut PxState),
    Without<PendingPxInitialization>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
  time: Res<Time>,
) {
  for (px_file, mut frame_animation, mut px_state) in q_px {
    if frame_animation.play_state == AnimationPlayState::Paused {
      continue;
    }
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let timer = match px_state.animation_timer.as_mut() {
      Some(timer) => timer,
      None => {
        let timer = Timer::from_seconds(frame_animation.duration, TimerMode::Once);
        px_state.animation_timer = Some(timer);

        px_state.animation_timer.as_mut().unwrap()
      }
    };

    timer.tick(time.delta());

    if timer.just_finished() {
      if frame_animation.loop_count != 0 {
        if px_state.loop_count >= frame_animation.loop_count {
          frame_animation.play_state = AnimationPlayState::Paused;
          continue;
        }
        if px_state.frame_index as usize >= artwork.0.frames_len() - 1 {
          px_state.loop_count += 1;
        }
      }

      let next_frame_index = px_state.next_frame(&artwork);
      px_state.frame_index = next_frame_index;
      px_state.animation_timer = None;

      if px_state.current_direction == AnimationDirection::PingPong {
        let range = artwork.get_tag_range(&px_state.current_tag);

        if px_state.temporary_direction == AnimationDirection::Forward
          && px_state.frame_index >= range.end - 1
        {
          px_state.temporary_direction = AnimationDirection::Backward;
        }

        if px_state.temporary_direction == AnimationDirection::Backward
          && px_state.frame_index == range.start
        {
          px_state.temporary_direction = AnimationDirection::Forward;
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
          (
            mark_changed_px_files_as_pending,
            initialize_pending_px_files,
          )
            .chain(),
          (
            detect_added_or_updated_tag_component,
            detect_removed_tag_component,
            detect_added_animation_component,
            detect_removed_animation_component,
            detect_updated_frame_animation_component,
          ),
        )
          .chain(),
        (render::<Sprite>, update_frame_index),
      )
        .chain(),
    );
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
    utils::default,
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

  fn get_px_state(app: &App, entity: Entity) -> &PxState {
    app.world().entity(entity).get::<PxState>().unwrap()
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
    match tag {
      Some(tag) => {
        app.world_mut().entity_mut(entity).insert(PxTag::new(tag));
      }
      None => {
        app.world_mut().entity_mut(entity).remove::<PxTag>();
      }
    }
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
      .get_mut::<PxState>()
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
      .get_mut::<PxState>()
      .unwrap()
      .current_direction = direction;

    set_temporary_direction(app, entity, initial_direction);
  }

  fn set_temporary_direction(app: &mut App, entity: Entity, direction: AnimationDirection) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxState>()
      .unwrap()
      .temporary_direction = direction;
  }

  fn set_loop_count(app: &mut App, entity: Entity, loop_count: u16) {
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxState>()
      .unwrap()
      .loop_count = loop_count;
  }

  #[test]
  fn test_does_not_render_while_initialization_is_pending() {
    let mut app = App::new();
    app
      .insert_resource(Assets::<PxArtwork>::default())
      .insert_resource(Assets::<Image>::default())
      .add_systems(PostUpdate, render::<Sprite>);

    let file_data = std::fs::read("assets/orange.px").unwrap();
    let artwork = PxArtwork(Artwork::read(&file_data).unwrap());
    let artwork_handle = app
      .world_mut()
      .resource_mut::<Assets<PxArtwork>>()
      .add(artwork);
    let entity = app
      .world_mut()
      .spawn((
        PixquareFile {
          artwork: artwork_handle,
          ..Default::default()
        },
        Sprite::default(),
      ))
      .id();

    app.update();

    assert_eq!(
      app.world().entity(entity).get::<Sprite>().unwrap().image,
      Handle::<Image>::default()
    );
  }

  #[test]
  fn test_does_not_reinitialize_when_layer_visibility_changes() {
    let (mut app, entity) = create_px_file_app(&"assets/orange.px");
    app.update();

    set_frame_index(&mut app, entity, 3);
    set_loop_count(&mut app, entity, 7);
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PixquareFile>()
      .unwrap()
      .layer_visibility = LayerVisibility::All;

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 3);
    assert_eq!(state.loop_count, 7);
    assert!(
      !app
        .world()
        .entity(entity)
        .contains::<PendingPxInitialization>()
    );
  }

  #[test]
  fn test_reinitializes_after_artwork_handle_changes() {
    let (mut app, entity) = create_px_file_app(&"assets/orange.px");
    app.update();
    set_frame_index(&mut app, entity, 3);

    let file_data = std::fs::read("assets/balloon.px").unwrap();
    let artwork = PxArtwork(Artwork::read(&file_data).unwrap());
    let artwork_handle = app
      .world_mut()
      .resource_mut::<Assets<PxArtwork>>()
      .add(artwork);
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PixquareFile>()
      .unwrap()
      .artwork = artwork_handle.clone();

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.artwork_id, Some(artwork_handle.id()));
    assert_eq!(state.frame_index, 0);
    assert!(
      !app
        .world()
        .entity(entity)
        .contains::<PendingPxInitialization>()
    );
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

    assert_eq!(get_px_state(&app, entity).frame_index, 1);
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
      .get_mut::<PxState>()
      .unwrap()
      .frame_index = frames_len - 2;

    app.update();

    let state = app.world().entity(entity).get::<PxState>().unwrap();

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
      .get_mut::<PxState>()
      .unwrap()
      .frame_index = 1;

    app.update();

    let state = app.world().entity(entity).get::<PxState>().unwrap();

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

    let state = get_px_state(&app, entity);
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

    let state = app.world().entity(entity).get::<PxState>().unwrap();

    assert_eq!(state.frame_index, frames_len - 1);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
    assert_eq!(state.current_direction, AnimationDirection::PingPong);

    app.update();

    let state = get_px_state(&app, entity);
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
    set_temporary_direction(&mut app, entity, AnimationDirection::Backward);

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 0);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    app.update();

    let state = get_px_state(&app, entity);
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

    let state = get_px_state(&app, entity);
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
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));

    app.update();

    let state = get_px_state(&app, entity);
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
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));

    app.update();

    let state = get_px_state(&app, entity);
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
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));

    app.update();

    let state = get_px_state(&app, entity);
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
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));
    app.update();

    set_frame_index(&mut app, entity, 5);
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();

    assert_eq!(get_px_state(&app, entity).frame_index, 4);
  }

  #[test]
  fn test_wraps_to_tag_last_frame_when_tag_direction_is_backward() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Backward,
        play_state: AnimationPlayState::Paused,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));
    app.update();

    set_frame_index(&mut app, entity, 4);
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();

    assert_eq!(get_px_state(&app, entity).frame_index, 5);
  }

  #[test]
  fn test_reverses_at_tag_last_frame_when_tag_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Playing,
        ..default()
      });
    set_tag(&mut app, entity, Some("front_move".into()));
    app.update();

    set_frame_index(&mut app, entity, 4);

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 5);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 5);
    assert_eq!(state.temporary_direction, AnimationDirection::Backward);
  }

  #[test]
  fn test_stays_on_single_frame_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    set_tag(&mut app, entity, Some("front".into()));
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Playing,
        ..default()
      });

    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 3);
    assert_eq!(state.current_tag.clone().unwrap(), "front");
  }

  #[test]
  fn test_resets_animation_status_when_switching_to_valid_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    set_tag(&mut app, entity, Some("front_move".into()));
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
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

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 7);
    assert_eq!(state.current_tag.clone().unwrap(), "right_move");
    assert_eq!(state.loop_count, 0);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
  }

  #[test]
  fn test_preserves_active_tag_and_status_when_switching_to_missing_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    set_tag(&mut app, entity, Some("front_move".into()));
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: FRAME_DURATION.as_secs_f32(),
        direction: AnimationDirection::Forward,
        play_state: AnimationPlayState::Playing,
        ..default()
      });
    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 5);

    set_loop_count(&mut app, entity, 7);
    set_tag(&mut app, entity, Some("invalid".into()));
    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.current_tag.clone().unwrap(), "front_move");
    assert_eq!(state.frame_index, 4);
    assert_eq!(state.loop_count, 7);
  }

  #[test]
  fn test_uses_all_frames_when_missing_tag_is_selected_without_active_tag() {
    let (mut app, entity) = create_px_file_app(&"assets/character_move.px");
    set_tag(&mut app, entity, None);
    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
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

    let state = get_px_state(&app, entity);
    assert_eq!(state.frame_index, 8);
    assert!(state.current_tag.is_none());
    assert_eq!(state.loop_count, 7);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);

    set_play_state(&mut app, entity, AnimationPlayState::Playing);
    app.update();

    assert_eq!(get_px_state(&app, entity).frame_index, 0);
  }
}
