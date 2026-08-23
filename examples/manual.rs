use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  PixquareUltraPlugin,
  data_type::AnimationPlayState,
  event::AdvanceAnimationFrameEvent,
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
    .add_systems(Update, advance_animation_frame)
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
      play_state: AnimationPlayState::Paused,
      ..default()
    },
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn advance_animation_frame(
  mut commands: Commands,
  q_px: Query<Entity, With<PixquareFile>>,
  inputs: Res<ButtonInput<MouseButton>>,
) {
  if inputs.just_pressed(MouseButton::Left) {
    for entity in q_px {
      commands.trigger(AdvanceAnimationFrameEvent(entity));
    }
  }
}
