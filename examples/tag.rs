//! example of specifying the frame to render by tag.
//! switch tag on left mouse click.
//!
//! command: cargo run --example tag

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin, PxTag};

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
    .add_systems(Update, switch_tag)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands.spawn((
    PixquareFile {
      artwork: server.load("apple.px"),
      ..default()
    },
    PxTag::new("red".into()),
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn switch_tag(
  mut px_tag: Single<&mut PxTag, With<PixquareFile>>,
  inputs: Res<ButtonInput<MouseButton>>,
) {
  if inputs.just_pressed(MouseButton::Left) {
    if px_tag.0 == "red" {
      px_tag.0 = "green".into();
    } else {
      px_tag.0 = "red".into();
    }
  }
}
