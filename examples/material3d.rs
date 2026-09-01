use bevy::{
  image::ImageSamplerDescriptor,
  log::LogPlugin,
  pbr::{Material, MaterialPlugin, MeshMaterial3d},
  prelude::*,
  render::render_resource::AsBindGroup,
  shader::ShaderRef,
};
use bevy_pixquare_ultra::{
  PixquareUltraPlugin,
  data_type::LayerVisibility,
  renderer::{PixquareFile, PxFrameAnimation, PxRenderAppExt, RenderPx},
};

const SHADER_ASSET_PATH: &str = "shaders/pixquare_material_3d.wgsl";

#[derive(Component)]
struct Cube;

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
      MaterialPlugin::<PixquareMaterial>::default(),
    ))
    .register_px_render_target::<MeshMaterial3d<PixquareMaterial>>()
    .add_systems(Startup, setup)
    .add_systems(Update, rotate_cube)
    .run();
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct PixquareMaterial {
  #[texture(0)]
  #[sampler(1)]
  texture: Option<Handle<Image>>,
}

impl Material for PixquareMaterial {
  fn fragment_shader() -> ShaderRef {
    SHADER_ASSET_PATH.into()
  }

  fn alpha_mode(&self) -> AlphaMode {
    AlphaMode::Blend
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
  commands.spawn((
    Camera3d::default(),
    Transform::from_xyz(0., 0., 5.).looking_at(Vec3::ZERO, Vec3::Y),
  ));

  commands.spawn((
    PointLight {
      shadows_enabled: true,
      ..default()
    },
    Transform::from_xyz(4., 6., 4.),
  ));

  let face_mesh = meshes.add(Rectangle::new(2., 2.));
  let material = materials.add(PixquareMaterial::default());
  let face_transforms = [
    Transform::from_xyz(0., 0., 1.),
    Transform::from_xyz(0., 0., -1.).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    Transform::from_xyz(1., 0., 0.)
      .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
    Transform::from_xyz(-1., 0., 0.)
      .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
    Transform::from_xyz(0., 1., 0.)
      .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    Transform::from_xyz(0., -1., 0.)
      .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
  ];

  commands
    .spawn((
      PixquareFile {
        artwork: asset_server.load("balloon.px"),
        layer_visibility: LayerVisibility::All,
        ..default()
      },
      PxFrameAnimation::default(),
      MeshMaterial3d(material.clone()),
      Transform::default(),
      Visibility::Visible,
      Cube,
    ))
    .with_children(|parent| {
      for transform in face_transforms {
        parent.spawn((
          Mesh3d(face_mesh.clone()),
          MeshMaterial3d(material.clone()),
          transform,
        ));
      }
    });
}

fn rotate_cube(mut q: Query<&mut Transform, With<Cube>>, time: Res<Time>) {
  for mut transform in &mut q {
    transform.rotate(Quat::from_rotation_y(time.delta_secs()));
  }
}
