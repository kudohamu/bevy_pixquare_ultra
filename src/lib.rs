use bevy::app::Plugin;

use crate::{loader::PixquareLoaderPlugin, renderer::PixquareRendererPlugin};

pub mod data_type;
pub mod loader;
pub mod renderer;

pub struct PixquareUltraPlugin;

impl Plugin for PixquareUltraPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_plugins((PixquareLoaderPlugin, PixquareRendererPlugin));
  }
}
