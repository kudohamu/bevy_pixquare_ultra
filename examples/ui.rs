//! example of rendering to ui node.
//!
//! command: cargo run --example ui

use bevy::{image::ImageSamplerDescriptor, log::LogPlugin, prelude::*};
use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin};

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
    .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
  commands.spawn((Camera2d, Transform::default().with_scale(Vec3::splat(0.1))));

  commands
    .spawn((
      Node {
        display: Display::Flex,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        width: px(300.),
        height: px(200.),
        padding: UiRect::all(px(10.)),
        border: UiRect::all(px(5.)),
        border_radius: BorderRadius::all(px(15.)),
        ..default()
      },
      BorderColor::all(Srgba::BLUE),
    ))
    .with_children(|parent| {
      parent.spawn((
        Node {
          width: px(100.),
          height: px(100.),
          ..default()
        },
        children![(
          PixquareFile {
            artwork: server.load("orange.px"),
            ..default()
          },
          ImageNode::default(),
        )],
      ));
    });
}
