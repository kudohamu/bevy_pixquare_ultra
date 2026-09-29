//! example of rendering only a specific region of image using a texture atlas.
//! defines named regions with display bounds, allowing you to specify which region to render.
//!
//! command: cargo run --example texture_atlas

use bevy::{
  image::ImageSamplerDescriptor, log::LogPlugin, platform::collections::HashMap, prelude::*,
};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin, PxAtlas, PxAtlasName};

#[derive(Debug, Component)]
struct Plant;

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
    .add_systems(Update, update_atlas_name)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  let mut atlas_regions = HashMap::new();
  atlas_regions.insert("flower".into(), URect::new(0, 0, 16, 16));
  atlas_regions.insert("wood".into(), URect::new(16, 0, 32, 32));
  atlas_regions.insert("board".into(), URect::new(32, 48, 48, 64));
  atlas_regions.insert("block".into(), URect::new(48, 48, 64, 64));

  commands.spawn((
    PixquareFile {
      artwork: server.load("sprite.px"),
      ..default()
    },
    PxAtlas::new(atlas_regions.clone()),
    PxAtlasName::new("flower".into()),
    Sprite::default(),
    Transform::from_xyz(0., 0., 0.),
    Plant,
  ));

  commands.spawn((
    PixquareFile {
      artwork: server.load("sprite.px"),
      ..default()
    },
    PxAtlas::new(atlas_regions.clone()),
    PxAtlasName::new("block".into()),
    Sprite::default(),
    Transform::from_xyz(-24., 0., 0.),
  ));

  let slicer = TextureSlicer {
    border: BorderRect::all(2.),
    center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    ..default()
  };

  commands.spawn((
    PixquareFile {
      artwork: server.load("sprite.px"),
      ..default()
    },
    PxAtlas::new(atlas_regions.clone()),
    PxAtlasName::new("board".into()),
    Sprite {
      custom_size: Some(Vec2::new(24.0, 40.0)),
      image_mode: SpriteImageMode::Sliced(slicer),
      ..default()
    },
    Transform::from_xyz(32., 0., 0.),
  ));
}

fn update_atlas_name(
  q_px: Query<&mut PxAtlasName, With<Plant>>,
  inputs: Res<ButtonInput<KeyCode>>,
  mut index: Local<usize>,
) {
  for mut px_atlas_name in q_px {
    if inputs.just_pressed(KeyCode::Space) {
      let names = ["flower", "wood"];
      let next_index = (*index + 1) % names.len();
      px_atlas_name.0 = names[next_index].to_string();
      *index = next_index;
    }
  }
}
