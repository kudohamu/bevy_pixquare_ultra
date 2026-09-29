//! example of rendering px image to 2d material.
//!
//! command: cargo run --example material2d

use bevy::{
  image::ImageSamplerDescriptor,
  log::LogPlugin,
  prelude::*,
  render::render_resource::AsBindGroup,
  shader::ShaderRef,
  sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};
use bevy_pixquare_ultra::{
  prelude::{PixquareFile, PixquareUltraPlugin, PxFrameAnimation},
  renderer::{PxRenderAppExt, RenderPx},
};

const SHADER_ASSET_PATH: &str = "shaders/pixquare_material.wgsl";

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
    .add_plugins((
      PixquareUltraPlugin,
      Material2dPlugin::<PixquareMaterial>::default(),
    ))
    .register_px_render_target::<MeshMaterial2d<PixquareMaterial>>()
    .add_systems(Startup, setup)
    .run();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct PixquareMaterial {
  #[texture(0)]
  #[sampler(1)]
  texture: Option<Handle<Image>>,
}

impl Material2d for PixquareMaterial {
  fn fragment_shader() -> ShaderRef {
    SHADER_ASSET_PATH.into()
  }

  fn alpha_mode(&self) -> AlphaMode2d {
    AlphaMode2d::Blend
  }
}

impl RenderPx for PixquareMaterial {
  type Param = ();

  fn render_px(&mut self, texture: Handle<Image>, _atlas: Option<TextureAtlas>, _param: &mut ()) {
    self.texture = Some(texture);
  }
}

fn setup(
  mut commands: Commands,
  asset_server: Res<AssetServer>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<PixquareMaterial>>,
) {
  commands.spawn(Camera2d);

  commands.spawn((
    PixquareFile {
      artwork: asset_server.load("balloon.px"),
      ..default()
    },
    PxFrameAnimation::default(),
    Mesh2d(meshes.add(Rectangle::new(128., 128.))),
    MeshMaterial2d(materials.add(PixquareMaterial::default())),
    Transform::default(),
  ));
}
