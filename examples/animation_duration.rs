//! example of overriding the frame duration.
//!
//! command: cargo run --example animation_duration

use std::time::Duration;

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
        }),
    )
    .add_plugins(PixquareUltraPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, change_animation_duration)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands.spawn((
    PixquareFile {
      artwork: server.load("balloon.px"),
      ..default()
    },
    PxFrameAnimation {
      duration: Some(Duration::from_millis(300)),
      ..default()
    },
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn change_animation_duration(
  q_px: Query<&mut PxFrameAnimation, With<PixquareFile>>,
  inputs: Res<ButtonInput<KeyCode>>,
) {
  for mut px_animation in q_px {
    if inputs.just_pressed(KeyCode::KeyA) {
      let Some(duration) = px_animation.duration else {
        continue;
      };
      px_animation.duration = Some(Duration::from_secs_f32(
        (duration.as_secs_f32() + 0.05).clamp(0.0, 1.0),
      ));
    }
    if inputs.just_pressed(KeyCode::KeyD) {
      let Some(duration) = px_animation.duration else {
        continue;
      };
      px_animation.duration = Some(Duration::from_secs_f32(
        (duration.as_secs_f32() - 0.05).clamp(0.0, 1.0),
      ));
    }
  }
}
