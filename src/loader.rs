use std::{ops::Range, time::Duration};

use bevy::{
  app::{App, Plugin},
  asset::{Asset, AssetApp, AssetLoader, Handle, ReflectAsset, RenderAssetUsages},
  image::{Image, ImageSampler},
  log::warn,
  math::UVec2,
  reflect::{Reflect, TypePath},
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use pixquare::Artwork;
use serde::{Deserialize, Serialize};

use crate::{
  data_type::{AnimationDirection, LayerVisibility},
  error::PixquareLoaderError,
};

#[cfg(feature = "atlas_asset")]
use crate::error::PxAtlasLoaderError;

#[cfg(feature = "atlas_asset")]
use bevy::{math::URect, platform::collections::HashMap};

/// Represents Artwork data of Pixquare.
/// Holds only the minimal structural data required for rendering in bevy_pixquare_ultra.
#[derive(Debug, Asset, Reflect)]
#[reflect(Asset)]
pub struct PxArtwork {
  canvas_size: UVec2,
  frames: Vec<PxFrameMeta>,
  tags: Vec<PxTagMeta>,
}

impl PxArtwork {
  #[cfg(feature = "asset_processing")]
  pub(crate) fn new(canvas_size: UVec2, frames: Vec<PxFrameMeta>, tags: Vec<PxTagMeta>) -> Self {
    Self {
      canvas_size,
      frames,
      tags,
    }
  }

  /// Returns size of canvas.
  pub fn canvas_size(&self) -> UVec2 {
    self.canvas_size
  }

  /// Returns length of frames.
  pub fn frame_count(&self) -> usize {
    self.frames.len()
  }

  /// Returns tags.
  pub fn tags(&self) -> &[PxTagMeta] {
    &self.tags
  }

  /// Returns the handle of the frame image specified by the frame number.
  pub fn frame_image(
    &self,
    frame_index: usize,
    visibility: LayerVisibility,
  ) -> Option<&Handle<Image>> {
    let frame = self.frames.get(frame_index)?;

    match visibility {
      LayerVisibility::All => Some(&frame.all_layer_image),
      LayerVisibility::Visible => Some(&frame.visible_layer_image),
    }
  }

  /// Returns the frame duration specified by the frame number.
  pub fn frame_duration(&self, frame_index: usize) -> Option<&Duration> {
    let frame = self.frames.get(frame_index)?;

    Some(&frame.duration)
  }

  /// Returns the frame range specified by tag.
  /// If None is specified, returns the range for all frames.
  pub fn get_tag_range(&self, tag: &Option<String>) -> Range<u16> {
    match tag {
      Some(tag) => {
        let Some(tag) = self.tags.iter().find(|t| t.name == *tag) else {
          warn!("tag: `{}` is not found", tag);

          return 0..self.frames.len() as u16;
        };

        tag.start_index..(tag.end_index + 1)
      }
      None => 0..self.frames.len() as u16,
    }
  }

  /// Returns loop count specified by tag.
  pub fn get_loop_count_by_tag(&self, tag: &str) -> Option<u16> {
    let Some(tag) = self.tags.iter().find(|t| t.name == *tag) else {
      warn!("tag: `{}` is not found", tag);

      return None;
    };

    Some(tag.loop_count)
  }

  /// Returns animation direction specified by tag.
  pub fn get_animation_direction_by_tag(&self, tag: &str) -> Option<AnimationDirection> {
    let Some(tag) = self.tags.iter().find(|t| t.name == *tag) else {
      warn!("tag: `{}` is not found", tag);

      return None;
    };

    Some(tag.direction)
  }

  pub(crate) fn from_artwork(
    artwork: &Artwork,
    settings: &PixquareLoaderSettings,
    mut add_image: impl FnMut(String, Image) -> Handle<Image>,
  ) -> Result<Self, PixquareLoaderError> {
    let frames = artwork
      .get_frames()
      .iter()
      .enumerate()
      .map(
        |(frame_index, frame)| -> Result<PxFrameMeta, PixquareLoaderError> {
          let visible_layer_buf = artwork
            .get_frame_image_as_straight_alpha(frame_index, LayerVisibility::Visible.into())?;

          let texture_format = if settings.is_srgb {
            TextureFormat::Rgba8UnormSrgb
          } else {
            TextureFormat::Rgba8Unorm
          };
          let mut visible_layer_image = Image::new(
            Extent3d {
              width: artwork.canvas_size.width,
              height: artwork.canvas_size.height,
              depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            visible_layer_buf,
            texture_format,
            settings.asset_usage,
          );
          visible_layer_image.sampler = settings.sampler.clone();
          let visible_layer_image = add_image(
            PxFrameMeta::generate_image_label(frame_index, LayerVisibility::Visible),
            visible_layer_image,
          );

          let all_layer_buf =
            artwork.get_frame_image_as_straight_alpha(frame_index, LayerVisibility::All.into())?;

          let mut all_layer_image = Image::new(
            Extent3d {
              width: artwork.canvas_size.width,
              height: artwork.canvas_size.height,
              depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            all_layer_buf,
            texture_format,
            settings.asset_usage,
          );
          all_layer_image.sampler = settings.sampler.clone();
          let all_layer_image = add_image(
            PxFrameMeta::generate_image_label(frame_index, LayerVisibility::All),
            all_layer_image,
          );

          Ok(PxFrameMeta {
            visible_layer_image,
            all_layer_image,
            duration: Duration::from_millis(frame.duration as u64),
          })
        },
      )
      .collect::<Result<_, _>>()?;

    let tags = artwork
      .tags
      .iter()
      .map(|tag| PxTagMeta {
        name: tag.name.clone(),
        start_index: tag.start_index,
        end_index: tag.end_index,
        direction: AnimationDirection::from_px_direction(tag.direction),
        loop_count: tag.loop_count,
      })
      .collect();

    Ok(Self {
      canvas_size: UVec2::new(artwork.canvas_size.width, artwork.canvas_size.height),
      frames,
      tags,
    })
  }

  pub(crate) fn get_initial_frame_index(
    &self,
    tag: &Option<String>,
    direction: AnimationDirection,
  ) -> u16 {
    let range = self.get_tag_range(tag);

    match direction {
      AnimationDirection::Forward => range.start,
      AnimationDirection::Backward => range.end - 1,
      AnimationDirection::PingPong => range.start,
    }
  }

  pub(crate) fn is_valid_tag(&self, tag: &str) -> bool {
    self.tags.iter().position(|t| t.name == tag).is_some()
  }
}

#[derive(Debug, Clone, Reflect)]
pub(crate) struct PxFrameMeta {
  pub visible_layer_image: Handle<Image>,
  pub all_layer_image: Handle<Image>,
  pub duration: Duration,
}

impl PxFrameMeta {
  pub(crate) fn generate_image_label(frame_index: usize, visibility: LayerVisibility) -> String {
    format!("frame_{}_{}", frame_index, visibility)
  }
}

/// Represents tag data of Pixquare.
/// Holds only the minimal structural data required for rendering in bevy_pixquare_ultra.
#[derive(Debug, Clone, Reflect)]
pub struct PxTagMeta {
  pub name: String,
  pub start_index: u16,
  pub end_index: u16,
  pub direction: AnimationDirection,
  pub loop_count: u16,
}

/// Named texture-atlas regions data loaded from a `.pxatlas.ron` file.
///
/// Each region specifies a rectangle within the Pixquare artwork's canvas.
/// This asset contains region definitions, not image data.
///
/// A example of `.pxatlas.ron` file:
///
/// ```text
/// (
///   regions: {
///     "flower": (x: 0, y: 0, width: 16, height: 16),
///   },
/// )
/// ```
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_pixquare_ultra::prelude::{PixquareUltraPlugin, PixquareFile, PxAtlas, PxAtlasName};
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(PixquareUltraPlugin)
///         .add_systems(Startup, setup)
///         .run();
/// }
///
/// fn setup(mut commands: Commands, server: Res<AssetServer>) {
///   commands.spawn((
///     PixquareFile {
///       artwork: server.load("sprite.px"),
///       ..default()
///     },
///     PxAtlas::from_asset(server.load("sprite.pxatlas.ron")),
///     PxAtlasName::new("flower".into()),
///     Sprite::default(),
///     Transform::from_xyz(0., 0., 0.),
///   ));
/// }
/// ```
#[cfg(feature = "atlas_asset")]
#[derive(Debug, Clone, Asset, Reflect)]
#[reflect(Asset)]
pub struct PxAtlasAsset {
  pub(crate) regions: HashMap<String, URect>,
}

#[cfg(feature = "atlas_asset")]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PxAtlasRon {
  regions: HashMap<String, PxAtlasRegionRon>,
}

#[cfg(feature = "atlas_asset")]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PxAtlasRegionRon {
  x: u32,
  y: u32,
  width: u32,
  height: u32,
}

#[derive(Debug, TypePath)]
pub(crate) struct PixquareLoader;

/// Settings for how to load Pixquare images.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PixquareLoaderSettings {
  pub sampler: ImageSampler,
  pub is_srgb: bool,
  pub asset_usage: RenderAssetUsages,
}

impl Default for PixquareLoaderSettings {
  fn default() -> Self {
    Self {
      sampler: ImageSampler::nearest(),
      is_srgb: true,
      asset_usage: RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    }
  }
}

impl AssetLoader for PixquareLoader {
  type Asset = PxArtwork;
  type Settings = PixquareLoaderSettings;
  type Error = PixquareLoaderError;

  async fn load(
    &self,
    reader: &mut dyn bevy::asset::io::Reader,
    settings: &Self::Settings,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<Self::Asset, Self::Error> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    let artwork = Artwork::read(&bytes)?;
    let px_artwork = PxArtwork::from_artwork(&artwork, settings, |label, image| {
      load_context.add_labeled_asset(label, image)
    })?;

    Ok(px_artwork)
  }

  fn extensions(&self) -> &[&str] {
    &["px"]
  }
}

#[cfg(feature = "atlas_asset")]
#[derive(Debug, TypePath)]
struct PxAtlasLoader;

#[cfg(feature = "atlas_asset")]
impl AssetLoader for PxAtlasLoader {
  type Asset = PxAtlasAsset;
  type Settings = ();
  type Error = PxAtlasLoaderError;

  async fn load(
    &self,
    reader: &mut dyn bevy::asset::io::Reader,
    _settings: &Self::Settings,
    _load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<Self::Asset, Self::Error> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;

    let atlas_ron = ron::de::from_bytes::<PxAtlasRon>(&bytes)?;
    let mut regions = HashMap::new();

    for (name, r) in atlas_ron.regions {
      regions.insert(name, URect::new(r.x, r.y, r.x + r.width, r.y + r.height));
    }

    Ok(PxAtlasAsset { regions })
  }

  fn extensions(&self) -> &[&str] {
    &["pxatlas.ron"]
  }
}

/// A bevy plugin for loading Pixquare artwork file.
#[derive(Debug)]
pub struct PixquareLoaderPlugin;

impl Plugin for PixquareLoaderPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_asset::<PxArtwork>()
      .register_asset_loader(PixquareLoader);

    #[cfg(feature = "atlas_asset")]
    app
      .init_asset::<PxAtlasAsset>()
      .register_asset_loader(PxAtlasLoader);
  }
}

#[cfg(all(test, feature = "atlas_asset"))]
mod tests {
  use bevy::{
    app::TaskPoolPlugin,
    asset::{AssetPlugin, AssetServer, Assets, LoadState},
  };

  use super::*;

  #[test]
  fn test_loads_px_atlas_asset_from_ron_file() {
    let mut app = App::new();
    app.add_plugins((
      TaskPoolPlugin::default(),
      AssetPlugin::default(),
      PixquareLoaderPlugin,
    ));

    let asset_server = app.world().resource::<AssetServer>().clone();
    let atlas_handle = asset_server.load::<PxAtlasAsset>("sprite.pxatlas.ron");

    for _ in 0..10_000 {
      app.update();

      match asset_server.load_state(&atlas_handle) {
        LoadState::Loaded => break,
        LoadState::Failed(error) => panic!("failed to load pxatlas asset: {error}"),
        _ => continue,
      }
    }

    assert!(matches!(
      asset_server.load_state(&atlas_handle),
      LoadState::Loaded
    ));

    let atlas_assets = app.world().resource::<Assets<PxAtlasAsset>>();
    let atlas = atlas_assets.get(&atlas_handle).unwrap();

    assert_eq!(atlas.regions.len(), 4);
    assert_eq!(atlas.regions.get("flower"), Some(&URect::new(0, 0, 16, 16)));
    assert_eq!(atlas.regions.get("wood"), Some(&URect::new(16, 0, 32, 32)));
    assert_eq!(
      atlas.regions.get("board"),
      Some(&URect::new(32, 48, 48, 64))
    );
    assert_eq!(
      atlas.regions.get("block"),
      Some(&URect::new(48, 48, 64, 64))
    );
  }
}
