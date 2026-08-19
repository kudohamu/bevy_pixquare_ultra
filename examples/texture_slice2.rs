use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::{
  PixquareUltraPlugin, event::PixquareFileInitializedEvent, renderer::PixquareFile,
};
use pixquare::utility_type::LayerVisibility;

/// If you want to avoid the following error log that appears before the .px file is applied to the Sprite,
/// you can also observe the Pixquare file initialization event triggered by bevy_pixquare_ultra.
///
/// ```
/// ERROR bevy_sprite::texture_slice::slicer: TextureSlicer::border has out of bounds values. No slicing will be applied
/// ```
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
    .add_observer(setup_texture_slice)
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands.spawn((
    PixquareFile {
      artwork: server.load("signboard.px"),
      layer_visibility: LayerVisibility::Visible,
      ..default()
    },
    Sprite {
      custom_size: Some(Vec2::new(50.0, 30.0)),
      ..default()
    },
    Transform::from_xyz(0., 0., 0.),
  ));
}

fn setup_texture_slice(event: On<PixquareFileInitializedEvent>, mut q_sprites: Query<&mut Sprite>) {
  let Ok(mut sprite) = q_sprites.get_mut(event.0) else {
    return;
  };

  let slicer = TextureSlicer {
    border: BorderRect::all(4.),
    center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
    ..default()
  };

  sprite.image_mode = SpriteImageMode::Sliced(slicer);
}
