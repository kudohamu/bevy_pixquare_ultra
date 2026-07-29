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
  log::debug,
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
  sprite::Sprite,
};

use crate::loader::PxArtwork;

#[derive(Component, Default)]
pub struct PixquareFile {
  pub artwork: Handle<PxArtwork>,
}

trait RenderPx {
  type Extra<'e>;

  fn render_px(&mut self, texture: Handle<Image>, _extra: &mut Self::Extra<'_>);
}

impl RenderPx for Sprite {
  type Extra<'e> = ();

  fn render_px(&mut self, texture: Handle<Image>, _extra: &mut Self::Extra<'_>) {
    self.image = texture;
    debug!("render!");
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

    let mut rgba_bytes = Vec::<u8>::with_capacity(
      (artwork.0.canvas_size.width * artwork.0.canvas_size.height * 4) as usize,
    );

    for color in artwork.0.frame_contents[0].colors.iter() {
      rgba_bytes.push(color.r);
      rgba_bytes.push(color.g);
      rgba_bytes.push(color.b);
      rgba_bytes.push(color.a);
    }

    let mut image = Image::new(
      Extent3d {
        width: artwork.0.canvas_size.width,
        height: artwork.0.canvas_size.height,
        depth_or_array_layers: 1,
      },
      TextureDimension::D2,
      rgba_bytes,
      TextureFormat::Rgba8UnormSrgb,
      RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    image.sampler = ImageSampler::nearest();

    let texture_handle = images.add(image);
    target.render_px(texture_handle, &mut extra);
  }
}

#[derive(Debug)]
pub struct PixquareRendererPlugin;

impl Plugin for PixquareRendererPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_systems(PostUpdate, render::<Sprite>);
  }
}
