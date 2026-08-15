use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_asset_loader::prelude::*;
use bevy_pixquare_ultra::{
  PixquareUltraPlugin,
  data_type::AnimationState,
  loader::PxArtwork,
  renderer::{PixquareFile, PxFrameAnimation},
};

#[derive(AssetCollection, Resource)]
struct ArtworkAssets {
  #[asset(path = "character_move.px")]
  character_move: Handle<PxArtwork>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum SceneState {
  #[default]
  AssetLoading,
  Main,
}

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
    .init_state::<SceneState>()
    .add_loading_state(
      LoadingState::new(SceneState::AssetLoading)
        .continue_to_state(SceneState::Main)
        .load_collection::<ArtworkAssets>(),
    )
    .add_systems(Startup, setup)
    .add_systems(OnEnter(SceneState::Main), setup_main_scene)
    .add_systems(Update, detect_key_input)
    .run();
}

fn setup(mut commands: Commands) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));
}

fn setup_main_scene(mut commands: Commands, assets: Res<ArtworkAssets>) {
  commands.spawn((
    PixquareFile {
      artwork: assets.character_move.clone(),
      ..default()
    },
    PxFrameAnimation {
      tag: Some("front_move".into()),
      duration: 0.3,
      play_state: AnimationState::Playing,
      ..default()
    },
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn detect_key_input(
  q_frame_animations: Query<&mut PxFrameAnimation>,
  input: Res<ButtonInput<KeyCode>>,
) {
  for mut frame_animation in q_frame_animations {
    if input.pressed(KeyCode::KeyW) {
      frame_animation.tag = Some("back_move".into());
      frame_animation.play_state = AnimationState::Playing;
    } else if input.pressed(KeyCode::KeyD) {
      frame_animation.tag = Some("right_move".into());
      frame_animation.play_state = AnimationState::Playing;
    } else if input.pressed(KeyCode::KeyA) {
      frame_animation.tag = Some("right_move".into());
      frame_animation.play_state = AnimationState::Playing;
    } else if input.pressed(KeyCode::KeyS) {
      frame_animation.tag = Some("front_move".into());
      frame_animation.play_state = AnimationState::Playing;
    } else {
      frame_animation.play_state = AnimationState::Paused;

      if let Some(tag) = &frame_animation.tag {
        if tag == "front_move" {
          frame_animation.tag = Some("front".into());
        } else if tag == "right_move" {
          frame_animation.tag = Some("right".into());
        } else if tag == "back_move" {
          frame_animation.tag = Some("back".into());
        }
      } else {
        frame_animation.tag = Some("front".into());
      }
    }
  }
}
