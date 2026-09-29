//! example of rendering only a specific region of image using a texture atlas.
//! You can also define regions using a Ron file.
//!
//! command: cargo run --features=atlas_asset --example texture_atlas_from_asset

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin, PxAtlas, PxAtlasName};

fn main() {
  App::new()
    .add_plugins(
      DefaultPlugins
        .set(ImagePlugin {
          default_sampler: ImageSamplerDescriptor::nearest(),
        })
        .set(LogPlugin {
          filter: "wgpu=warn,bevy_ecs=info,bevy_pixquare_ultra=debug".into(),
          ..default()
        }),
    )
    .add_plugins(PixquareUltraPlugin)
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands.spawn((
    PixquareFile {
      artwork: server.load("sprite.px"),
      ..default()
    },
    PxAtlas::from_asset(server.load("sprite.pxatlas.ron")),
    PxAtlasName::new("flower".into()),
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}
