use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  PixquareUltraPlugin,
  data_type::AnimationPlayState,
  renderer::{PixquareFile, PxFrameAnimation, PxTag},
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
    .add_systems(Update, detect_key_input)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));
  commands.spawn((
    PixquareFile {
      artwork: server.load("character_move.px"),
      ..default()
    },
    PxTag::new("front".into()),
    PxFrameAnimation {
      duration: 0.3,
      play_state: AnimationPlayState::Playing,
      ..default()
    },
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn detect_key_input(
  mut commands: Commands,
  q_frame_animations: Query<(Entity, &mut Sprite, &mut PxFrameAnimation, &mut PxTag)>,
  input: Res<ButtonInput<KeyCode>>,
) {
  for (entity, mut sprite, mut frame_animation, mut px_tag) in q_frame_animations {
    if input.pressed(KeyCode::KeyW) {
      frame_animation.play_state = AnimationPlayState::Playing;
      sprite.flip_x = false;
      commands
        .entity(entity)
        .insert(PxTag::new("back_move".into()));
    } else if input.pressed(KeyCode::KeyD) {
      frame_animation.play_state = AnimationPlayState::Playing;
      sprite.flip_x = false;
      commands
        .entity(entity)
        .insert(PxTag::new("right_move".into()));
    } else if input.pressed(KeyCode::KeyA) {
      frame_animation.play_state = AnimationPlayState::Playing;
      sprite.flip_x = true;
      commands
        .entity(entity)
        .insert(PxTag::new("right_move".into()));
    } else if input.pressed(KeyCode::KeyS) {
      frame_animation.play_state = AnimationPlayState::Playing;
      sprite.flip_x = false;
      commands
        .entity(entity)
        .insert(PxTag::new("front_move".into()));
    } else {
      frame_animation.play_state = AnimationPlayState::Paused;

      if px_tag.0 == "front_move" {
        px_tag.0 = "front".into();
      } else if px_tag.0 == "right_move" {
        px_tag.0 = "right".into();
      } else if px_tag.0 == "back_move" {
        px_tag.0 = "back".into();
      }
    }
  }
}
