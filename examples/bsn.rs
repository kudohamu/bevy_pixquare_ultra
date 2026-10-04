//! example of spawning artwork using bsn (Bevy Scene Notation) syntax.
//!
//! command: cargo run --features=atlas_asset --example bsn

use std::time::Duration;

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  prelude::{PixquareFile, PixquareUltraPlugin},
  renderer::{PxAtlas, PxAtlasName, PxFrameAnimation, PxTag},
};

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

fn spawn_balloon() -> impl Scene {
  bsn! {
    PixquareFile { artwork: "balloon.px" }
    PxFrameAnimation { duration: Duration::from_millis(500) }
    Sprite
    Transform::from_xyz(-15., 10., 0.)
  }
}

fn spawn_flower() -> impl Scene {
  bsn! {
    PixquareFile { artwork: "sprite.px" }
    PxAtlas::from_asset("sprite.pxatlas.ron")
    PxAtlasName("flower")
    Sprite
    Transform::from_xyz(15., 10., 0.)
  }
}

fn setup(mut commands: Commands) {
  commands.spawn_scene(bsn! {
    Camera2d
    Transform { scale: Vec3::splat(0.1) }
  });

  commands.spawn_scene(spawn_balloon());

  commands.spawn_scene(spawn_flower());

  commands.spawn_scene(bsn! {
    PixquareFile { artwork: "apple.px" }
    PxTag("green")
    Sprite
    Transform::from_xyz(0., -15., 0.)
  });
}
