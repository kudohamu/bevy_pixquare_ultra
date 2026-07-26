use bevy::{image::ImageSamplerDescriptor, prelude::*};
use bevy_pixquare_ultra::PixquareUltraPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin {
      default_sampler: ImageSamplerDescriptor::nearest(),
    }))
    .add_plugins(PixquareUltraPlugin)
    .run();
}
