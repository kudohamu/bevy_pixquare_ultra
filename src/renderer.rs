use bevy::{
  app::{Plugin, PostUpdate},
  asset::{Assets, Handle, RenderAssetUsages},
  ecs::{
    change_detection::DetectChanges,
    component::{Component, Mutable},
    system::{Query, Res, ResMut},
    world::Ref,
  },
  image::{Image, ImageSampler},
  log::error,
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
  sprite::Sprite,
};
use pixquare::utility_type::LayerVisibility;

use crate::loader::PxArtwork;

#[derive(Component)]
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
  mut q_px: Query<(&mut T, Ref<PixquareFile>)>,
  res_pxartworks: Res<Assets<PxArtwork>>,
  mut images: ResMut<Assets<Image>>,
  mut extra: <T as RenderPx>::Extra<'_>,
) {
  if !res_pxartworks.is_changed() {
    return;
  }

  for (mut target, px) in q_px.iter_mut() {
    if !px.is_changed() {
      return;
    }
    let Some(artwork) = res_pxartworks.get(&px.artwork) else {
      continue;
    };

    match artwork.0.get_frame_image(0, px.layer_visibility) {
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

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_systems(PostUpdate, render::<Sprite>);
  }
}
