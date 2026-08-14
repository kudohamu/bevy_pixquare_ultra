use bevy::{
  app::{Plugin, PostUpdate},
  asset::{AsAssetId, Assets, Handle, RenderAssetUsages},
  ecs::{
    component::{Component, Mutable},
    query::{Changed, Or},
    system::{Query, Res, ResMut},
    world::Ref,
  },
  image::{Image, ImageSampler},
  log::error,
  prelude::AssetChanged,
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
  sprite::Sprite,
  time::{Time, Timer, TimerMode},
};
use pixquare::utility_type::LayerVisibility;

use crate::{
  data_type::{AnimationDirection, AnimationState},
  loader::PxArtwork,
};

#[derive(Debug, Component)]
#[require(PxAnimationStatus)]
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
}

impl Default for PxAnimationStatus {
  fn default() -> Self {
    Self {
      frame_index: 0,
      current_direction: AnimationDirection::Forward,
      loop_count: 0,
      animation_timer: None,
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

    let Some(timer) = animation_status.animation_timer.as_mut() else {
      animation_status.animation_timer = Some(Timer::from_seconds(
        frame_animation.duration,
        TimerMode::Once,
      ));
      continue;
    };

    if timer.is_finished() {
      if frame_animation.loop_count != 0 {
        if animation_status.loop_count >= frame_animation.loop_count {
          frame_animation.play_state = AnimationState::Paused;
          continue;
        }
        if animation_status.frame_index as usize >= artwork.0.frames_len() {
          animation_status.loop_count += 1;
        }
      }

      let next_frame_index = frame_animation.next_frame(&artwork, &animation_status);
      animation_status.frame_index = next_frame_index;
      animation_status.animation_timer = None;
    } else {
      timer.tick(time.delta());
    }
  }
}

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_systems(PostUpdate, (render::<Sprite>, update_frame_index));
  }
}

impl PxFrameAnimation {
  fn next_frame(&self, artwork: &PxArtwork, status: &PxAnimationStatus) -> u16 {
    let delta: i16 = if status.current_direction == AnimationDirection::Forward {
      1
    } else {
      -1
    };

    if let Some(_tag) = &self.tag {
      0
    } else {
      let layer_frames_len = artwork.0.layers.get(0).map_or(0, |l| l.frames.len());
      let tilemap_layer_frames_len = artwork
        .0
        .tilemap_layers
        .get(0)
        .map_or(0, |l| l.frames.len());
      let frames_len = layer_frames_len.max(tilemap_layer_frames_len);

      0 + (status.frame_index as i16 - 0 + delta).rem_euclid(frames_len as i16 - 0) as u16
    }
  }
}
