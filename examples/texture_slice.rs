//! example of implementing nine-patch scaling using texture_slice.
//!
//! command: cargo run --example texture_slice

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin};

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
  let slicer = TextureSlicer {
    border: BorderRect::all(4.),
    center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    ..default()
  };

  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands.spawn((
    PixquareFile {
      artwork: server.load("signboard.px"),
      ..default()
    },
    Sprite {
      custom_size: Some(Vec2::new(50.0, 30.0)),
      image_mode: SpriteImageMode::Sliced(slicer),
      ..default()
    },
    Transform::from_xyz(0., 0., 0.),
  ));
}
