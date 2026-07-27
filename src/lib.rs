use bevy::app::Plugin;

use crate::loader::PixquareLoaderPlugin;

pub mod loader;

pub struct PixquareUltraPlugin;

impl Plugin for PixquareUltraPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.add_plugins(PixquareLoaderPlugin);
  }
}
