use std::{ops::Range, time::Duration};

use bevy::{
  app::{App, Plugin},
  asset::{Asset, AssetApp, AssetLoader, Handle, RenderAssetUsages},
  image::{Image, ImageSampler},
  log::warn,
  math::UVec2,
  reflect::TypePath,
  render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use pixquare::model::Artwork;

use crate::{
  data_type::{AnimationDirection, LayerVisibility},
  error::PixquareLoaderError,
};

#[derive(Debug, Asset, TypePath)]
pub struct PxArtwork {
  pub canvas_size: UVec2,
  pub(crate) frames: Vec<PxFrameMeta>,
  pub tags: Vec<PxTagMeta>,
}

impl PxArtwork {
  pub fn from_artwork(
    artwork: &Artwork,
    mut add_image: impl FnMut(String, Image) -> Handle<Image>,
  ) -> Result<Self, PixquareLoaderError> {
    let frames = artwork
      .get_frames()
      .iter()
      .enumerate()
      .map(
        |(frame_index, frame)| -> Result<PxFrameMeta, PixquareLoaderError> {
          let visible_layer_buf =
            artwork.get_frame_image(frame_index, LayerVisibility::Visible.into())?;

          let mut visible_layer_image = Image::new(
            Extent3d {
              width: artwork.canvas_size.width,
              height: artwork.canvas_size.height,
              depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            visible_layer_buf,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
          );
          visible_layer_image.sampler = ImageSampler::nearest();
          let visible_layer_image =
            add_image(format!("frame_{frame_index}_visible"), visible_layer_image);

          let all_layer_buf = artwork.get_frame_image(frame_index, LayerVisibility::All.into())?;

          let mut all_layer_image = Image::new(
            Extent3d {
              width: artwork.canvas_size.width,
              height: artwork.canvas_size.height,
              depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            all_layer_buf,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
          );
          all_layer_image.sampler = ImageSampler::nearest();
          let all_layer_image = add_image(format!("frame_{frame_index}_all"), all_layer_image);

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

  pub fn get_tag_range(&self, tag: &Option<String>) -> Range<u16> {
    match tag {
      Some(tag) => {
        let Some(tag) = self.tags.iter().find(|t| t.name == *tag) else {
          warn!("tag: `{}` is not found", tag);

          return 0..self.frames.len() as u16;
        };

        tag.start_index..(tag.end_index + 1)
      }
      None => {
        return 0..self.frames.len() as u16;
      }
    }
  }

  pub fn get_initial_frame_index(
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

  pub fn is_valid_tag(&self, tag: &str) -> bool {
    self.tags.iter().position(|t| t.name == tag).is_some()
  }
}

#[derive(Debug)]
pub(crate) struct PxFrameMeta {
  pub visible_layer_image: Handle<Image>,
  pub all_layer_image: Handle<Image>,
  pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct PxTagMeta {
  pub name: String,
  pub start_index: u16,
  pub end_index: u16,
  pub direction: AnimationDirection,
  pub loop_count: u16,
}

#[derive(Debug, TypePath)]
struct PixquareLoader;

impl AssetLoader for PixquareLoader {
  type Asset = PxArtwork;
  type Settings = ();
  type Error = PixquareLoaderError;

  async fn load(
    &self,
    reader: &mut dyn bevy::asset::io::Reader,
    _settings: &Self::Settings,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<Self::Asset, Self::Error> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await?;
    let artwork = Artwork::read(&bytes)?;
    let px_artwork = PxArtwork::from_artwork(&artwork, |label, image| {
      load_context.add_labeled_asset(label, image)
    })?;

    Ok(px_artwork)
  }

  fn extensions(&self) -> &[&str] {
    &["px"]
  }
}

#[derive(Debug)]
pub struct PixquareLoaderPlugin;

impl Plugin for PixquareLoaderPlugin {
  fn build(&self, app: &mut App) {
    app.init_asset::<PxArtwork>();
    app.register_asset_loader(PixquareLoader);
  }
}
