//! example of using bevy's asset processing pipeline.
//!
//! command: cargo run --features=asset_processing --example asset_processing

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin, PxFrameAnimation};

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
        })
        .set(AssetPlugin {
          // NOTE: This specification is required to enable Bevy's asset processing pipeline.
          mode: AssetMode::Processed,
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
      artwork: server.load("balloon.px"),
      ..default()
    },
    PxFrameAnimation::default(),
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}
