//! example of implementing nine-patch scaling using texture_slice.
//!
//! command: cargo run --example texture_slice

use bevy::{log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ScaleDirection {
  Expand,
  #[default]
  Shrink,
}

fn main() {
  App::new()
    .add_plugins(
      DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(LogPlugin {
          filter: "wgpu=warn,bevy_ecs=info,bevy_pixquare_ultra=debug".into(),
          ..default()
        }),
    )
    .add_plugins(PixquareUltraPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, change_sprite_size)
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

fn change_sprite_size(
  mut q_px: Query<&mut Sprite, With<PixquareFile>>,
  time: Res<Time>,
  mut horizontal_direction: Local<ScaleDirection>,
  mut vertical_direction: Local<ScaleDirection>,
) {
  for mut sprite in q_px.iter_mut() {
    let mut next_size = sprite.custom_size.unwrap();

    match *horizontal_direction {
      ScaleDirection::Expand => {
        next_size.x = (sprite.custom_size.unwrap().x + 12. * time.delta_secs()).min(50.);

        if next_size.x == 50. {
          *horizontal_direction = ScaleDirection::Shrink;
        }
      }
      ScaleDirection::Shrink => {
        next_size.x = (sprite.custom_size.unwrap().x - 12. * time.delta_secs()).max(20.);

        if next_size.x == 20. {
          *horizontal_direction = ScaleDirection::Expand;
        }
      }
    }

    match *vertical_direction {
      ScaleDirection::Expand => {
        next_size.y = (sprite.custom_size.unwrap().y + 12. * time.delta_secs()).min(30.);

        if next_size.y == 30. {
          *vertical_direction = ScaleDirection::Shrink;
        }
      }
      ScaleDirection::Shrink => {
        next_size.y = (sprite.custom_size.unwrap().y - 12. * time.delta_secs()).max(10.);

        if next_size.y == 10. {
          *vertical_direction = ScaleDirection::Expand;
        }
      }
    }

    sprite.custom_size = Some(next_size);
  }
}
