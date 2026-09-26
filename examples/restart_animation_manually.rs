use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  PixquareUltraPlugin,
  event::RestartFrameAnimationEvent,
  renderer::{PixquareFile, PxFrameAnimation},
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
    .add_systems(Update, restart_animation)
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

fn restart_animation(
  mut commands: Commands,
  q_px: Query<Entity, With<PixquareFile>>,
  inputs: Res<ButtonInput<MouseButton>>,
) {
  if inputs.just_pressed(MouseButton::Left) {
    for entity in q_px {
      commands.trigger(RestartFrameAnimationEvent(entity));
    }
  }
}
