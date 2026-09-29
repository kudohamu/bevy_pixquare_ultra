use bevy::app::Plugin;

#[cfg(feature = "asset_processing")]
use crate::processor::PixquareProcessorPlugin;
use crate::{loader::PixquareLoaderPlugin, renderer::PixquareRendererPlugin};

pub mod data_type;
pub mod error;
pub mod event;
#[cfg(feature = "asset_processing")]
mod image;
pub mod loader;
#[cfg(feature = "asset_processing")]
pub mod processor;
pub mod renderer;

pub mod prelude {
  pub use crate::PixquareUltraPlugin;
  pub use crate::data_type::{AnimationDirection, AnimationPlayState, LayerVisibility};
  pub use crate::renderer::{PixquareFile, PxAtlas, PxAtlasName, PxFrameAnimation, PxTag};
}

/// A bevy plugin to load and render pixquare data.
///
/// ```no_run
/// use bevy::{image::ImageSamplerDescriptor, prelude::*};
/// use bevy_pixquare_ultra::prelude::{PixquareFile, PixquareUltraPlugin};
///
/// fn main() {
///   App::new()
///     .add_plugins(
///       DefaultPlugins
///         .set(ImagePlugin {
///           default_sampler: ImageSamplerDescriptor::nearest(),
///         }),
///     )
///     .add_plugins(PixquareUltraPlugin)
///     .add_systems(Startup, setup)
///     .run();
/// }
///
/// fn setup(mut commands: Commands, server: Res<AssetServer>) {
///   commands.spawn((Camera2d, Transform::default()));
///
///   commands.spawn((
///     PixquareFile {
///       artwork: server.load("sprite.px"),
///       ..default()
///     },
///     Sprite::default(),
///     Transform::from_xyz(0., 0., 0.),
///   ));
/// }
/// ```
pub struct PixquareUltraPlugin;

impl Plugin for PixquareUltraPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_plugins((PixquareLoaderPlugin, PixquareRendererPlugin));
    #[cfg(feature = "asset_processing")]
    app.add_plugins(PixquareProcessorPlugin);
  }
}
