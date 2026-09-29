//! example observing animation finished events of PxFrameAnimation component.
//!
//! command: cargo run --example observe_animation_finished_events

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  event::{AnimationFinishedEvent, AnimationLoopFinishedEvent},
  prelude::{PixquareFile, PixquareUltraPlugin, PxFrameAnimation},
};

fn main() {
  App::new()
    .add_plugins(
      DefaultPlugins
        .set(ImagePlugin {
          default_sampler: ImageSamplerDescriptor::nearest(),
        })
        .set(LogPlugin {
          filter: "wgpu=warn,bevy_ecs=info,bevy_pixquare_ultra=debug,observe_animation_finished_events=debug".into(),
          ..default()
        }),
    )
    .add_plugins(PixquareUltraPlugin)
    .add_systems(Startup, setup)
    .add_observer(handle_animation_loop_finished)
    .add_observer(handle_animation_finish)
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
      loop_count: Some(3),
      ..default()
    },
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn handle_animation_loop_finished(trigger: On<AnimationLoopFinishedEvent>) {
  debug!("finished animation loop: entity({})", trigger.entity);
}

fn handle_animation_finish(trigger: On<AnimationFinishedEvent>) {
  debug!("finished animation: entity({})", trigger.0);
}
