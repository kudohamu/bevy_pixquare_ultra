use std::time::Duration;

use bevy::{
  app::Plugin,
  asset::{AssetLoader, AsyncReadExt, Handle},
  image::{CompressedImageFormats, Image, ImageLoaderSettings},
  math::UVec2,
  reflect::TypePath,
};
use serde::{Deserialize, Serialize};

use crate::{
  data_type::AnimationDirection,
  error::PixquareLoaderError,
  image::decode_image,
  loader::{PxArtwork, PxFrameMeta, PxTagMeta},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ImageCodec {
  Qoi,
}

impl From<ProcessedImageCodecV1> for ImageCodec {
  fn from(value: ProcessedImageCodecV1) -> Self {
    match value {
      ProcessedImageCodecV1::Qoi => Self::Qoi,
    }
  }
}

#[derive(Debug, Clone)]
struct ProcessedPxArtworkHeader {
  format_version: u16,
  flags: u16,
  data_len: u64,
}

impl ProcessedPxArtworkHeader {
  const MAGIC: &[u8; 4] = b"PXUL";
  const CURRENT_FORMAT_VERSION: u16 = 1;
  const HEADER_SIZE: usize = 16;

  async fn read(reader: &mut dyn bevy::asset::io::Reader) -> Result<Self, PixquareLoaderError> {
    let mut magic = [0_u8; 4];
    reader.read_exact(&mut magic).await?;

    if &magic != Self::MAGIC {
      return Err(PixquareLoaderError::ProcessedPxArtworkLoadError);
    }

    let mut format_version = [0_u8; 2];
    reader.read_exact(&mut format_version).await?;

    let mut flags = [0_u8; 2];
    reader.read_exact(&mut flags).await?;

    let mut data_len = [0_u8; 8];
    reader.read_exact(&mut data_len).await?;

    Ok(Self {
      format_version: u16::from_le_bytes(format_version),
      flags: u16::from_le_bytes(flags),
      data_len: u64::from_le_bytes(data_len),
    })
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ProcessedImageCodecV1 {
  Qoi,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessedImageMetaV1 {
  label: String,
  codec: ProcessedImageCodecV1,
  encoded_len: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum ProcessedAnimationDirectionV1 {
  Forward,
  Backward,
  PingPong,
}

impl From<ProcessedAnimationDirectionV1> for AnimationDirection {
  fn from(value: ProcessedAnimationDirectionV1) -> Self {
    match value {
      ProcessedAnimationDirectionV1::Forward => Self::Forward,
      ProcessedAnimationDirectionV1::Backward => Self::Backward,
      ProcessedAnimationDirectionV1::PingPong => Self::PingPong,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessedPxTagMetaV1 {
  name: String,
  start_index: u16,
  end_index: u16,
  direction: ProcessedAnimationDirectionV1,
  loop_count: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessedPxFrameMetaV1 {
  visible_layer_image_index: u32,
  all_layer_image_index: u32,
  duration: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessedPxArtworkV1 {
  canvas_width: u32,
  canvas_height: u32,
  frames: Vec<ProcessedPxFrameMetaV1>,
  tags: Vec<ProcessedPxTagMetaV1>,
  images: Vec<ProcessedImageMetaV1>,
}

impl ProcessedPxArtworkV1 {
  async fn read(
    reader: &mut dyn bevy::asset::io::Reader,
    header: &ProcessedPxArtworkHeader,
    supported_compressed_formats: CompressedImageFormats,
    settings: &ImageLoaderSettings,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<(Self, Vec<Handle<Image>>), PixquareLoaderError> {
    let mut px_artwork_data_buf = vec![0_u8; header.data_len as usize];
    reader.read_exact(&mut px_artwork_data_buf).await?;

    let px_artwork_data: ProcessedPxArtworkV1 = rmp_serde::from_slice(&px_artwork_data_buf)?;

    let mut image_handles = Vec::with_capacity(px_artwork_data.images.len());

    for image_meta in &px_artwork_data.images {
      let mut image_buf = Vec::with_capacity(image_meta.encoded_len as usize);
      reader.read_exact(&mut image_buf).await?;

      let image = decode_image(
        image_meta.codec.into(),
        &image_buf,
        supported_compressed_formats,
        settings,
      )?;

      if image.width() != px_artwork_data.canvas_width
        || image.height() != px_artwork_data.canvas_height
      {
        return Err(PixquareLoaderError::ImageDimensionsMismatch {
          expected: UVec2::new(image.width(), image.height()),
          actual: UVec2::new(px_artwork_data.canvas_width, px_artwork_data.canvas_height),
        });
      }

      let image_handle = load_context.add_labeled_asset(image_meta.label.clone(), image);
      image_handles.push(image_handle);
    }

    Ok((px_artwork_data, image_handles))
  }
}

impl PxArtwork {
  fn from_processed_v1(data: &ProcessedPxArtworkV1, image_handles: Vec<Handle<Image>>) -> Self {
    let canvas_size = UVec2::new(data.canvas_width, data.canvas_height);

    let frames = data
      .frames
      .iter()
      .map(|frame| PxFrameMeta {
        visible_layer_image: image_handles[frame.visible_layer_image_index as usize].clone(),
        all_layer_image: image_handles[frame.all_layer_image_index as usize].clone(),
        duration: Duration::from_millis(frame.duration as u64),
      })
      .collect();

    let tags = data
      .tags
      .iter()
      .map(|tag| PxTagMeta {
        name: tag.name.clone(),
        start_index: tag.start_index,
        end_index: tag.end_index,
        direction: tag.direction.into(),
        loop_count: tag.loop_count,
      })
      .collect();

    Self::new(canvas_size, frames, tags)
  }
}

#[derive(Debug, TypePath)]
struct ProcessedPixquareLoader {
  supported_compressed_formats: CompressedImageFormats,
}

impl AssetLoader for ProcessedPixquareLoader {
  type Asset = PxArtwork;

  type Settings = ImageLoaderSettings;

  type Error = PixquareLoaderError;

  async fn load(
    &self,
    reader: &mut dyn bevy::asset::io::Reader,
    settings: &Self::Settings,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<Self::Asset, Self::Error> {
    let header = ProcessedPxArtworkHeader::read(reader).await?;

    match header.format_version {
      1 => {
        let (processed_v1, image_handles) = ProcessedPxArtworkV1::read(
          reader,
          &header,
          self.supported_compressed_formats,
          settings,
          load_context,
        )
        .await?;
        let px_artwork = PxArtwork::from_processed_v1(&processed_v1, image_handles);

        Ok(px_artwork)
      }
      _ => Err(PixquareLoaderError::UnsupportedFormatVersionError),
    }
  }
}

pub struct PixquareProcessorPlugin;

impl Plugin for PixquareProcessorPlugin {
  fn build(&self, app: &mut bevy::app::App) {}

  fn finish(&self, _app: &mut bevy::app::App) {}
}
