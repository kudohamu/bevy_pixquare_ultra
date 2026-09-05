use bevy::{
  app::{App, Plugin, PostUpdate},
  asset::{AsAssetId, AssetId, Assets, Handle},
  ecs::{
    change_detection::DetectChanges,
    component::{Component, Mutable},
    entity::Entity,
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Or, With, Without},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut, StaticSystemParam, SystemParam, SystemParamItem},
    world::Ref,
  },
  image::{Image, TextureAtlas, TextureAtlasLayout},
  log::error,
  math::URect,
  platform::collections::HashMap,
  prelude::AssetChanged,
  sprite::Sprite,
  sprite_render::{Material2d, MeshMaterial2d, SpriteSystems},
  time::{Time, Timer, TimerMode},
  ui::{UiSystems, widget::ImageNode},
};

#[cfg(feature = "3d")]
use bevy::pbr::{Material, MeshMaterial3d};

use crate::{
  data_type::{AnimationDirection, AnimationPlayState, LayerVisibility},
  event::{AdvanceAnimationFrameEvent, AnimationLoopFinishedEvent, PixquareFileInitializedEvent},
  loader::PxArtwork,
};

#[cfg(feature = "atlas_asset")]
use crate::loader::PxAtlasAsset;

#[derive(Debug, Component)]
#[require(PxState, PendingPxInitialization, PxRenderedImageCache)]
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
  pub duration: Option<f32>,
  pub direction: AnimationDirection,
  pub loop_count: u16,
  pub play_state: AnimationPlayState,
}

impl Default for PxFrameAnimation {
  fn default() -> Self {
    Self {
      duration: None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PxRenderedImageKey {
  artwork_id: AssetId<PxArtwork>,
  frame_index: u16,
  layer_visibility: LayerVisibility,
}

#[derive(Component, Default)]
struct PxRenderedImageCache {
  image: Option<Handle<Image>>,
  key: Option<PxRenderedImageKey>,
  dirty: bool,
}

#[derive(Debug, Component)]
pub struct PxAtlas {
  source: PxAtlasSource,
}

#[derive(Debug, Clone)]
enum PxAtlasSource {
  Code(HashMap<String, URect>),
  #[cfg(feature = "atlas_asset")]
  Asset(Handle<PxAtlasAsset>),
}

impl PxAtlas {
  pub fn new(data: HashMap<String, URect>) -> Self {
    Self {
      source: PxAtlasSource::Code(data),
    }
  }

  #[cfg(feature = "atlas_asset")]
  pub fn from_asset(handle: Handle<PxAtlasAsset>) -> Self {
    Self {
      source: PxAtlasSource::Asset(handle),
    }
  }
}

#[derive(Debug, Component)]
pub struct PxAtlasName(pub String);

impl PxAtlasName {
  pub fn new(name: String) -> Self {
    Self(name)
  }
}

#[derive(Debug, Component)]
pub(crate) struct PxAtlasMeta {
  artwork_id: AssetId<PxArtwork>,
  atlas_layout: Handle<TextureAtlasLayout>,
  atlas_indices: HashMap<String, usize>,
}

impl PxAtlasMeta {
  fn get_texture_atlas(&self, name: &str) -> Option<TextureAtlas> {
    let Some(index) = self.atlas_indices.get(name) else {
      return None;
    };

    Some(TextureAtlas {
      layout: self.atlas_layout.clone(),
      index: *index,
    })
  }
}

pub trait RenderPx {
  type Param: SystemParam + 'static;

  fn render_px(
    &mut self,
    texture: Handle<Image>,
    atlas: Option<TextureAtlas>,
    _param: &mut SystemParamItem<'_, '_, Self::Param>,
  );
}

impl RenderPx for Sprite {
  type Param = ();

  fn render_px(
    &mut self,
    texture: Handle<Image>,
    atlas: Option<TextureAtlas>,
    _param: &mut SystemParamItem<'_, '_, Self::Param>,
  ) {
    self.image = texture;
    self.texture_atlas = atlas;
  }
}

impl RenderPx for ImageNode {
  type Param = ();

  fn render_px(
    &mut self,
    texture: Handle<Image>,
    atlas: Option<TextureAtlas>,
    _param: &mut SystemParamItem<'_, '_, Self::Param>,
  ) {
    self.image = texture;
    self.texture_atlas = atlas;
  }
}

impl<M: Material2d + RenderPx> RenderPx for MeshMaterial2d<M> {
  type Param = (ResMut<'static, Assets<M>>, <M as RenderPx>::Param);

  fn render_px(
    &mut self,
    texture: Handle<Image>,
    atlas: Option<TextureAtlas>,
    param: &mut SystemParamItem<'_, '_, Self::Param>,
  ) {
    let Some(material) = param.0.get_mut(&*self) else {
      return;
    };
    material.render_px(texture, atlas, &mut param.1);
  }
}

#[cfg(feature = "3d")]
impl<M: Material + RenderPx> RenderPx for MeshMaterial3d<M> {
  type Param = (ResMut<'static, Assets<M>>, <M as RenderPx>::Param);

  fn render_px(
    &mut self,
    texture: Handle<Image>,
    atlas: Option<TextureAtlas>,
    param: &mut SystemParamItem<'_, '_, Self::Param>,
  ) {
    let Some(material) = param.0.get_mut(&*self) else {
      return;
    };
    material.render_px(texture, atlas, &mut param.1);
  }
}

/// Extension methods for registering custom Pixquare render targets.
pub trait PxRenderAppExt {
  /// Registers a component implementing [`RenderPx`] as a render target.
  fn register_px_render_target<T>(&mut self) -> &mut Self
  where
    T: RenderPx + Component<Mutability = Mutable>;
}

impl PxRenderAppExt for App {
  fn register_px_render_target<T>(&mut self) -> &mut Self
  where
    T: RenderPx + Component<Mutability = Mutable>,
  {
    self.add_systems(
      PostUpdate,
      apply_image::<T>
        .after(generate_image)
        .before(update_frame_index),
    )
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

fn mark_asset_changed_px_images_as_dirty(
  mut q_px: Query<&mut PxRenderedImageCache, AssetChanged<PixquareFile>>,
) {
  for mut rendered_image in &mut q_px {
    rendered_image.dirty = true;
  }
}

fn cleanup_removed_px_files(mut commands: Commands, mut removed: RemovedComponents<PixquareFile>) {
  for entity in removed.read() {
    let Ok(mut entity_commands) = commands.get_entity(entity) else {
      continue;
    };
    entity_commands.remove::<PxRenderedImageCache>();
  }
}

fn generate_image(
  mut q_px: Query<
    (&PixquareFile, &PxState, &mut PxRenderedImageCache),
    Without<PendingPxInitialization>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  for (px_file, px_state, mut rendered_image) in &mut q_px {
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let key = PxRenderedImageKey {
      artwork_id: px_file.artwork.id(),
      frame_index: px_state.frame_index,
      layer_visibility: px_file.layer_visibility,
    };
    if !rendered_image.dirty && rendered_image.key == Some(key) {
      continue;
    }

    let Some(frame_image) =
      artwork.frame_image(px_state.frame_index as usize, px_file.layer_visibility)
    else {
      error!(
        "could not find frame image(artwork_id: {}, frame_index: {}, visibility: {})",
        px_file.artwork.id(),
        px_state.frame_index,
        px_file.layer_visibility
      );
      continue;
    };
    rendered_image.image = Some(frame_image.clone());
    rendered_image.key = Some(key);
    rendered_image.dirty = false;
  }
}

fn apply_image<T: RenderPx + Component<Mutability = Mutable>>(
  mut q_px: Query<
    (
      &mut T,
      &PxRenderedImageCache,
      Option<&PxAtlasMeta>,
      Option<&PxAtlasName>,
    ),
    (
      Or<(
        Added<T>,
        Changed<PxRenderedImageCache>,
        Changed<PxAtlasMeta>,
        Changed<PxAtlasName>,
      )>,
      Without<PendingPxInitialization>,
    ),
  >,
  mut param: StaticSystemParam<T::Param>,
) {
  for (mut target, rendered_image, px_atlas, px_atlas_name) in &mut q_px {
    let Some(image) = &rendered_image.image else {
      continue;
    };
    let atlas = px_atlas.and_then(|atlas| {
      px_atlas_name.and_then(|atlas_name| {
        let Some(texture_atlas) = atlas.get_texture_atlas(&atlas_name.0) else {
          error!("atlas({}) is not found", atlas_name.0);

          return None;
        };

        Some(texture_atlas)
      })
    });

    target.render_px(image.clone(), atlas, &mut param);
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

    if let Some(frame_animation) = frame_animation {
      let initial_direction = match frame_animation.direction {
        AnimationDirection::Forward => AnimationDirection::Forward,
        AnimationDirection::Backward => AnimationDirection::Backward,
        AnimationDirection::PingPong => AnimationDirection::Forward,
      };

      px_state.frame_index =
        artwork.get_initial_frame_index(&px_state.current_tag, initial_direction);
      px_state.current_direction = frame_animation.direction;
      px_state.temporary_direction = initial_direction;
      px_state.loop_count = 0;
    } else {
      px_state.frame_index =
        artwork.get_initial_frame_index(&px_state.current_tag, AnimationDirection::Forward);
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

    animation_state.current_tag = None;

    if let Some(frame_animation) = frame_animation {
      let initial_direction = match frame_animation.direction {
        AnimationDirection::Forward => AnimationDirection::Forward,
        AnimationDirection::Backward => AnimationDirection::Backward,
        AnimationDirection::PingPong => AnimationDirection::Forward,
      };

      animation_state.frame_index =
        artwork.get_initial_frame_index(&animation_state.current_tag, initial_direction);
      animation_state.current_direction = frame_animation.direction;
      animation_state.temporary_direction = initial_direction;
      animation_state.loop_count = 0;
    } else {
      animation_state.frame_index =
        artwork.get_initial_frame_index(&animation_state.current_tag, AnimationDirection::Forward);
    }
  }
}

fn update_frame_index(
  mut commands: Commands,
  q_px: Query<
    (Entity, &PixquareFile, &mut PxFrameAnimation, &mut PxState),
    Without<PendingPxInitialization>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
  time: Res<Time>,
) {
  for (entity, px_file, mut frame_animation, mut px_state) in q_px {
    if frame_animation.play_state == AnimationPlayState::Paused {
      continue;
    }
    let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };

    let timer = match px_state.animation_timer.as_mut() {
      Some(timer) => timer,
      None => {
        let Some(default_frame_duration) = artwork.frame_duration(px_state.frame_index as usize)
        else {
          continue;
        };
        let timer = Timer::from_seconds(
          frame_animation
            .duration
            .unwrap_or(default_frame_duration.as_secs_f32()),
          TimerMode::Once,
        );
        px_state.animation_timer = Some(timer);

        px_state.animation_timer.as_mut().unwrap()
      }
    };

    timer.tick(time.delta());

    if timer.just_finished() {
      let is_loop_finished = advance_animation_frame(artwork, &mut frame_animation, &mut px_state);

      if is_loop_finished {
        commands.trigger(AnimationLoopFinishedEvent(entity));
      }
    }
  }
}

fn handle_advance_animation_frame_event(
  trigger: On<AdvanceAnimationFrameEvent>,
  mut commands: Commands,
  mut q_px: Query<
    (Entity, &PixquareFile, &mut PxFrameAnimation, &mut PxState),
    Without<PendingPxInitialization>,
  >,
  res_pxartworks: Res<Assets<PxArtwork>>,
) {
  let Ok((entity, px_file, mut frame_animation, mut px_state)) = q_px.get_mut(trigger.0) else {
    return;
  };
  let Some(artwork) = res_pxartworks.get(&px_file.artwork) else {
    return;
  };

  let is_loop_finished = advance_animation_frame(artwork, &mut frame_animation, &mut px_state);

  if is_loop_finished {
    commands.trigger(AnimationLoopFinishedEvent(entity));
  }
}

fn advance_animation_frame(
  px_artwork: &PxArtwork,
  frame_animation: &mut PxFrameAnimation,
  px_state: &mut PxState,
) -> bool {
  if frame_animation.loop_count != 0 && px_state.loop_count >= frame_animation.loop_count {
    frame_animation.play_state = AnimationPlayState::Paused;
    return false;
  }

  let range = px_artwork.get_tag_range(&px_state.current_tag);
  let previous_frame_index = px_state.frame_index;
  let previous_direction = px_state.temporary_direction;
  let next_frame_index = px_state.next_frame(&px_artwork);
  px_state.frame_index = next_frame_index;
  px_state.animation_timer = None;

  if px_state.current_direction == AnimationDirection::PingPong {
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

  let is_single_frame = range.len() == 1;
  let is_final_frame = match px_state.current_direction {
    AnimationDirection::Forward => {
      previous_frame_index == range.end - 1 && next_frame_index == range.start
    }
    AnimationDirection::Backward => {
      previous_frame_index == range.start && next_frame_index == range.end - 1
    }
    AnimationDirection::PingPong => {
      previous_direction == AnimationDirection::Backward && next_frame_index == range.start
    }
  };
  let is_loop_finished = is_single_frame || is_final_frame;

  if frame_animation.loop_count != 0 && is_loop_finished {
    px_state.loop_count += 1;

    if px_state.loop_count >= frame_animation.loop_count {
      frame_animation.play_state = AnimationPlayState::Paused;
      return true;
    }
  }

  return false;
}

fn initialize_px_atlas(
  mut commands: Commands,
  q_px: Query<(Entity, &PixquareFile, Ref<PxAtlas>, Option<&PxAtlasMeta>)>,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut res_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
  #[cfg(feature = "atlas_asset")] res_px_atlas_asset: Res<Assets<PxAtlasAsset>>,
) {
  for (entity, px_file, px_atlas, px_atlas_meta) in q_px {
    let Some(px_artwork) = res_pxartworks.get(&px_file.artwork) else {
      continue;
    };
    if !px_atlas.is_changed()
      && px_atlas_meta.is_some_and(|meta| px_file.artwork.id() == meta.artwork_id)
    {
      continue;
    }

    let mut atlas_indices = HashMap::new();
    let mut atlas_layout = TextureAtlasLayout::new_empty(px_artwork.canvas_size());

    let mut regions = match &px_atlas.source {
      PxAtlasSource::Code(regions) => regions.iter().collect::<Vec<_>>(),
      #[cfg(feature = "atlas_asset")]
      PxAtlasSource::Asset(handle) => {
        let Some(asset) = res_px_atlas_asset.get(handle) else {
          error!("pxatlas asset is not found: {:?}", handle.path());
          continue;
        };

        asset.regions.iter().collect::<Vec<_>>()
      }
    };
    regions.sort_by(|a, b| a.0.cmp(b.0));

    for (name, rect) in regions {
      let atlas_index = atlas_layout.add_texture(*rect);
      atlas_indices.insert(name.clone(), atlas_index);
    }

    let atlas_layout = res_atlas_layouts.add(atlas_layout);

    commands.entity(entity).insert(PxAtlasMeta {
      artwork_id: px_file.artwork.id(),
      atlas_layout,
      atlas_indices,
    });
  }
}

fn cleanup_removed_px_atlas(
  mut commands: Commands,
  mut removed: RemovedComponents<PxAtlas>,
  mut q_caches: Query<&mut PxRenderedImageCache>,
) {
  for entity in removed.read() {
    let Ok(mut entity_commands) = commands.get_entity(entity) else {
      continue;
    };
    entity_commands.remove::<PxAtlasMeta>();

    let Ok(mut cache) = q_caches.get_mut(entity) else {
      continue;
    };
    cache.dirty = true;
  }
}

fn cleanup_removed_px_atlas_name(
  mut removed: RemovedComponents<PxAtlasName>,
  mut q_caches: Query<&mut PxRenderedImageCache>,
) {
  for entity in removed.read() {
    let Ok(mut cache) = q_caches.get_mut(entity) else {
      continue;
    };
    cache.dirty = true;
  }
}

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app
      .add_systems(
        PostUpdate,
        (
          (
            mark_changed_px_files_as_pending,
            initialize_pending_px_files,
            (
              detect_removed_animation_component,
              detect_added_animation_component,
              detect_updated_frame_animation_component,
              detect_removed_tag_component,
              detect_added_or_updated_tag_component,
            )
              .chain(),
          )
            .chain()
            .before(generate_image),
          (initialize_px_atlas, mark_asset_changed_px_images_as_dirty).before(generate_image),
          (
            cleanup_removed_px_files,
            cleanup_removed_px_atlas,
            cleanup_removed_px_atlas_name,
          )
            .before(initialize_px_atlas),
          (
            generate_image,
            (
              apply_image::<Sprite>.before(SpriteSystems::ComputeSlices),
              apply_image::<ImageNode>.before(UiSystems::Content),
            ),
            update_frame_index,
          )
            .chain(),
        ),
      )
      .add_observer(handle_advance_animation_frame_event);
  }
}

#[cfg(test)]
mod tests {
  use std::{assert_eq, time::Duration};

  use bevy::{
    app::App,
    asset::Assets,
    ecs::{component::Component, entity::Entity, observer::On, resource::Resource},
    image::Image,
    math::Rect,
    sprite::Sprite,
    time::{TimePlugin, TimeUpdateStrategy},
    ui::widget::{ImageNode, NodeImageMode},
    utils::default,
  };
  use pixquare::model::Artwork;

  use crate::loader::PixquareLoaderSettings;

  use super::*;

  const FRAME_DURATION: Duration = Duration::from_millis(100);
  const TIME_STEP: Duration = Duration::from_millis(101);

  fn create_px_file_app(path: &str) -> (App, Entity) {
    create_px_file_app_with_target::<Sprite>(path)
  }

  fn add_px_artwork(app: &mut App, path: &str) -> Handle<PxArtwork> {
    let file_data = std::fs::read(path).unwrap();
    let artwork = Artwork::read(&file_data).unwrap();
    let settings = PixquareLoaderSettings::default();

    let px_artwork = {
      let mut images = app.world_mut().resource_mut::<Assets<Image>>();

      PxArtwork::from_artwork(&artwork, &settings, |_label, image| images.add(image)).unwrap()
    };

    app
      .world_mut()
      .resource_mut::<Assets<PxArtwork>>()
      .add(px_artwork)
  }

  fn create_px_file_app_with_target<T: Component + Default>(path: &str) -> (App, Entity) {
    let mut app = App::new();
    app
      .add_plugins((TimePlugin, PixquareRendererPlugin))
      .insert_resource(TimeUpdateStrategy::ManualDuration(TIME_STEP))
      .insert_resource(Assets::<PxArtwork>::default())
      .insert_resource(Assets::<Image>::default())
      .insert_resource(Assets::<TextureAtlasLayout>::default());

    #[cfg(feature = "atlas_asset")]
    app.insert_resource(Assets::<PxAtlasAsset>::default());

    app.update();

    let artwork = add_px_artwork(&mut app, path);
    let entity = app
      .world_mut()
      .spawn((
        PixquareFile {
          artwork,
          ..Default::default()
        },
        T::default(),
      ))
      .id();

    (app, entity)
  }

  fn get_px_artwork(app: &App, entity: Entity) -> &PxArtwork {
    let px_file = app.world().entity(entity).get::<PixquareFile>().unwrap();
    let res_pxartwork = app.world().get_resource::<Assets<PxArtwork>>().unwrap();
    let px_artwork = &res_pxartwork.get(&px_file.artwork).unwrap();

    return px_artwork;
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

  fn create_atlas_regions() -> HashMap<String, URect> {
    HashMap::from([
      ("flower".into(), URect::new(0, 0, 16, 16)),
      ("wood".into(), URect::new(16, 0, 32, 32)),
    ])
  }

  fn get_sprite_atlas_rect(app: &App, entity: Entity) -> Option<URect> {
    let sprite = app.world().entity(entity).get::<Sprite>()?;
    let atlas = sprite.texture_atlas.as_ref()?;
    let atlas_layouts = app.world().resource::<Assets<TextureAtlasLayout>>();

    atlas_layouts
      .get(&atlas.layout)?
      .textures
      .get(atlas.index)
      .map(|t| *t)
  }

  #[test]
  fn test_does_not_render_while_initialization_is_pending() {
    let mut app = App::new();
    app
      .insert_resource(Assets::<PxArtwork>::default())
      .insert_resource(Assets::<Image>::default())
      .add_systems(PostUpdate, (generate_image, apply_image::<Sprite>).chain());

    let artwork_handle = add_px_artwork(&mut app, "assets/orange.px");
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

    let artwork_handle = add_px_artwork(&mut app, "assets/balloon.px");
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
  fn test_renders_to_image_node_and_preserves_its_display_properties() {
    let (mut app, entity) = create_px_file_app_with_target::<ImageNode>("assets/orange.px");
    let rect = Rect::new(1., 2., 9., 10.);
    let initial_image = {
      let mut entity_mut = app.world_mut().entity_mut(entity);
      let mut image_node = entity_mut.get_mut::<ImageNode>().unwrap();
      image_node.rect = Some(rect);
      image_node.image_mode = NodeImageMode::Stretch;
      image_node.image.clone()
    };

    app.update();

    let image_node = app.world().entity(entity).get::<ImageNode>().unwrap();
    assert_ne!(image_node.image, initial_image);
    assert_eq!(image_node.rect, Some(rect));
    assert_eq!(image_node.image_mode, NodeImageMode::Stretch);

    let first_rendered_image = image_node.image.clone();
    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PixquareFile>()
      .unwrap()
      .layer_visibility = LayerVisibility::All;
    app.update();

    let image_node = app.world().entity(entity).get::<ImageNode>().unwrap();
    assert_ne!(image_node.image, first_rendered_image);
    assert_eq!(image_node.rect, Some(rect));
    assert_eq!(image_node.image_mode, NodeImageMode::Stretch);
  }

  #[test]
  fn test_renders_to_image_node_added_after_initialization() {
    let (mut app, entity) = create_px_file_app_with_target::<ImageNode>("assets/orange.px");
    app.world_mut().entity_mut(entity).remove::<ImageNode>();

    app.update();

    let initial_image = ImageNode::default().image;
    app.world_mut().entity_mut(entity).insert(ImageNode {
      image_mode: NodeImageMode::Stretch,
      ..default()
    });
    app.update();

    let image_node = app.world().entity(entity).get::<ImageNode>().unwrap();
    assert_ne!(image_node.image, initial_image);
    assert_eq!(image_node.image_mode, NodeImageMode::Stretch);
  }

  #[test]
  fn test_advances_to_next_frame_when_entity_has_px_animation_frame_component() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
    let px_artwork = &res_pxartwork.get(&px_file.artwork).unwrap();
    let frames_len = px_artwork.frame_count() as u16;

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
        direction: AnimationDirection::Backward,
        loop_count: 0,
        ..Default::default()
      });

    app.update();

    set_frame_index(&mut app, entity, 0);

    app.update();

    let state = get_px_state(&app, entity);
    let px_artwork = get_px_artwork(&app, entity);
    assert_eq!(state.frame_index, px_artwork.frame_count() as u16 - 1);
  }

  #[test]
  fn test_reverses_to_backward_at_last_frame_when_direction_is_ping_pong() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");

    let px_artwork = get_px_artwork(&app, entity);
    let frames_len = px_artwork.frame_count() as u16;

    app
      .world_mut()
      .get_entity_mut(entity)
      .unwrap()
      .insert(PxFrameAnimation {
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
  fn test_starts_new_tag_from_first_frame_when_ping_pong_was_moving_backward() {
    let (mut app, entity) = create_px_file_app("assets/character_move.px");
    app.world_mut().entity_mut(entity).insert((
      PxTag::new("front_move".into()),
      PxFrameAnimation {
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Paused,
        ..default()
      },
    ));
    app.update();

    set_frame_index(&mut app, entity, 5);
    set_temporary_direction(&mut app, entity, AnimationDirection::Backward);
    set_tag(&mut app, entity, Some("right_move".into()));
    app.update();

    let state = get_px_state(&app, entity);
    assert_eq!(state.current_tag.as_deref(), Some("right_move"));
    assert_eq!(state.frame_index, 7);
    assert_eq!(state.temporary_direction, AnimationDirection::Forward);
  }

  #[test]
  fn test_starts_from_first_frame_when_tag_is_removed_while_ping_pong_was_moving_backward() {
    let (mut app, entity) = create_px_file_app("assets/character_move.px");
    app.world_mut().entity_mut(entity).insert((
      PxTag::new("front_move".into()),
      PxFrameAnimation {
        direction: AnimationDirection::PingPong,
        play_state: AnimationPlayState::Paused,
        ..default()
      },
    ));
    app.update();

    set_frame_index(&mut app, entity, 5);
    set_temporary_direction(&mut app, entity, AnimationDirection::Backward);
    set_tag(&mut app, entity, None);
    app.update();

    let state = get_px_state(&app, entity);
    assert!(state.current_tag.is_none());
    assert_eq!(state.frame_index, 0);
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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
        duration: Some(FRAME_DURATION.as_secs_f32()),
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

  #[derive(Resource, Default)]
  struct ObservedAnimationLoopFinishedEvents(Vec<Entity>);

  fn observe_animation_loop_finished_event(
    event: On<AnimationLoopFinishedEvent>,
    mut observed_events: ResMut<ObservedAnimationLoopFinishedEvents>,
  ) {
    observed_events.0.push(event.0);
  }

  #[test]
  fn test_triggers_animation_loop_finished_event_once_when_loop_count_is_reached() {
    let (mut app, entity) = create_px_file_app("assets/balloon.px");
    app
      .init_resource::<ObservedAnimationLoopFinishedEvents>()
      .add_observer(observe_animation_loop_finished_event);
    app.world_mut().entity_mut(entity).insert(PxFrameAnimation {
      duration: Some(FRAME_DURATION.as_secs_f32()),
      loop_count: 1,
      ..default()
    });

    app.update();
    let last_frame_index = get_px_artwork(&app, entity).frame_count() as u16 - 1;
    set_frame_index(&mut app, entity, last_frame_index);
    set_loop_count(&mut app, entity, 0);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 0);
    assert_eq!(get_px_state(&app, entity).loop_count, 1);
    assert_eq!(
      app
        .world()
        .resource::<ObservedAnimationLoopFinishedEvents>()
        .0,
      vec![entity]
    );
    assert_eq!(
      app
        .world()
        .entity(entity)
        .get::<PxFrameAnimation>()
        .unwrap()
        .play_state,
      AnimationPlayState::Paused
    );

    app.update();
    assert_eq!(
      app
        .world()
        .resource::<ObservedAnimationLoopFinishedEvents>()
        .0,
      vec![entity]
    );
  }

  #[test]
  fn test_finishes_forward_animation_after_returning_to_tag_first_frame() {
    let (mut app, entity) = create_px_file_app("assets/character_move.px");
    app
      .init_resource::<ObservedAnimationLoopFinishedEvents>()
      .add_observer(observe_animation_loop_finished_event);
    app.world_mut().entity_mut(entity).insert((
      PxTag::new("front_move".into()),
      PxFrameAnimation {
        duration: Some(FRAME_DURATION.as_secs_f32()),
        direction: AnimationDirection::Forward,
        loop_count: 1,
        play_state: AnimationPlayState::Paused,
      },
    ));
    app.update();
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 5);
    assert_eq!(get_px_state(&app, entity).loop_count, 0);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 4);
    assert_eq!(get_px_state(&app, entity).loop_count, 1);
    assert_eq!(
      app
        .world()
        .resource::<ObservedAnimationLoopFinishedEvents>()
        .0,
      vec![entity]
    );
    assert_eq!(
      app
        .world()
        .entity(entity)
        .get::<PxFrameAnimation>()
        .unwrap()
        .play_state,
      AnimationPlayState::Paused
    );
  }

  #[test]
  fn test_finishes_backward_animation_after_returning_to_tag_last_frame() {
    let (mut app, entity) = create_px_file_app("assets/character_move.px");
    app
      .init_resource::<ObservedAnimationLoopFinishedEvents>()
      .add_observer(observe_animation_loop_finished_event);
    app.world_mut().entity_mut(entity).insert((
      PxTag::new("front_move".into()),
      PxFrameAnimation {
        duration: Some(FRAME_DURATION.as_secs_f32()),
        direction: AnimationDirection::Backward,
        loop_count: 1,
        play_state: AnimationPlayState::Paused,
      },
    ));
    app.update();
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 4);
    assert_eq!(get_px_state(&app, entity).loop_count, 0);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 5);
    assert_eq!(get_px_state(&app, entity).loop_count, 1);
    assert_eq!(
      app
        .world()
        .resource::<ObservedAnimationLoopFinishedEvents>()
        .0,
      vec![entity]
    );
    assert_eq!(
      app
        .world()
        .entity(entity)
        .get::<PxFrameAnimation>()
        .unwrap()
        .play_state,
      AnimationPlayState::Paused
    );
  }

  #[test]
  fn test_finishes_ping_pong_animation_after_returning_to_first_frame() {
    let (mut app, entity) = create_px_file_app("assets/balloon.px");
    app
      .init_resource::<ObservedAnimationLoopFinishedEvents>()
      .add_observer(observe_animation_loop_finished_event);
    app.world_mut().entity_mut(entity).insert(PxFrameAnimation {
      duration: Some(FRAME_DURATION.as_secs_f32()),
      direction: AnimationDirection::PingPong,
      loop_count: 1,
      play_state: AnimationPlayState::Paused,
    });
    app.update();

    let last_frame_index = get_px_artwork(&app, entity).frame_count() as u16 - 1;
    set_play_state(&mut app, entity, AnimationPlayState::Playing);

    for _ in 0..last_frame_index {
      app.update();
    }
    assert_eq!(get_px_state(&app, entity).frame_index, last_frame_index);
    assert_eq!(get_px_state(&app, entity).loop_count, 0);

    for _ in 0..last_frame_index - 1 {
      app.update();
    }
    assert_eq!(get_px_state(&app, entity).frame_index, 1);
    assert_eq!(get_px_state(&app, entity).loop_count, 0);

    app.update();
    assert_eq!(get_px_state(&app, entity).frame_index, 0);
    assert_eq!(get_px_state(&app, entity).loop_count, 1);
    assert_eq!(
      app
        .world()
        .resource::<ObservedAnimationLoopFinishedEvents>()
        .0,
      vec![entity]
    );
    assert_eq!(
      app
        .world()
        .entity(entity)
        .get::<PxFrameAnimation>()
        .unwrap()
        .play_state,
      AnimationPlayState::Paused
    );
  }

  #[test]
  fn test_uses_px_frame_duration_when_animation_duration_is_none() {
    let (mut app, entity) = create_px_file_app(&"assets/balloon.px");
    let px_frame_duration = *get_px_artwork(&app, entity).frame_duration(0).unwrap();
    let margin = Duration::from_millis(1);
    let duration_before_frame_advance = px_frame_duration.checked_sub(margin).unwrap();

    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
      TimeUpdateStrategy::ManualDuration(duration_before_frame_advance);
    app.world_mut().entity_mut(entity).insert(PxFrameAnimation {
      duration: None,
      ..Default::default()
    });

    app.update();

    let px_state = get_px_state(&app, entity);
    assert_eq!(px_state.frame_index, 0);
    assert!(
      px_state
        .animation_timer
        .as_ref()
        .unwrap()
        .duration()
        .abs_diff(px_frame_duration)
        < Duration::from_micros(1)
    );

    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
      TimeUpdateStrategy::ManualDuration(margin * 2);
    app.update();

    assert_eq!(get_px_state(&app, entity).frame_index, 1);
  }

  #[test]
  fn test_applies_selected_atlas_region_to_sprite() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("flower".into()),
    ));

    app.update();

    assert_eq!(
      get_sprite_atlas_rect(&app, entity),
      Some(URect::new(0, 0, 16, 16))
    );
  }

  #[test]
  fn test_updates_sprite_atlas_when_atlas_name_changes() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("flower".into()),
    ));
    app.update();

    app
      .world_mut()
      .entity_mut(entity)
      .get_mut::<PxAtlasName>()
      .unwrap()
      .0 = "wood".into();
    app.update();

    assert_eq!(
      get_sprite_atlas_rect(&app, entity),
      Some(URect::new(16, 0, 32, 32))
    );
  }

  #[test]
  fn test_clears_sprite_atlas_when_atlas_name_is_removed() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("flower".into()),
    ));
    app.update();
    assert!(get_sprite_atlas_rect(&app, entity).is_some());

    app.world_mut().entity_mut(entity).remove::<PxAtlasName>();
    app.update();

    assert!(
      app
        .world()
        .entity(entity)
        .get::<Sprite>()
        .unwrap()
        .texture_atlas
        .is_none()
    );
  }

  #[test]
  fn test_clears_sprite_atlas_when_px_atlas_is_removed() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("flower".into()),
    ));
    app.update();
    assert!(get_sprite_atlas_rect(&app, entity).is_some());

    app.world_mut().entity_mut(entity).remove::<PxAtlas>();
    app.update();

    assert!(
      app
        .world()
        .entity(entity)
        .get::<Sprite>()
        .unwrap()
        .texture_atlas
        .is_none()
    );
    assert!(app.world().entity(entity).get::<PxAtlasMeta>().is_none());
  }

  #[test]
  fn test_rebuilds_atlas_layout_when_px_atlas_changes() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("flower".into()),
    ));
    app.update();

    let updated_region = URect::new(32, 32, 48, 48);
    app
      .world_mut()
      .entity_mut(entity)
      .insert(PxAtlas::new([("flower".into(), updated_region)].into()));
    app.update();

    assert_eq!(get_sprite_atlas_rect(&app, entity), Some(updated_region));
  }

  #[test]
  fn test_does_not_apply_atlas_when_region_name_is_missing() {
    let (mut app, entity) = create_px_file_app("assets/sprite.px");
    let expected_image = get_px_artwork(&app, entity)
      .frame_image(0, LayerVisibility::Visible)
      .unwrap()
      .clone();
    app.world_mut().entity_mut(entity).insert((
      PxAtlas::new(create_atlas_regions()),
      PxAtlasName::new("missing".into()),
    ));

    app.update();

    let sprite = app.world().entity(entity).get::<Sprite>().unwrap();
    assert_eq!(sprite.image, expected_image);
    assert!(sprite.texture_atlas.is_none());
  }
}
