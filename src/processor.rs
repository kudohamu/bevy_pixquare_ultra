use std::time::Duration;

use bevy::{
  app::Plugin,
  asset::{
    AssetApp, AssetLoader, AsyncReadExt, AsyncWriteExt, Handle, processor::LoadTransformAndSave,
    saver::AssetSaver, transformer::IdentityAssetTransformer,
  },
  image::{CompressedImageFormats, Image},
  math::UVec2,
  reflect::TypePath,
  render::{render_resource::TextureFormat, renderer::RenderDevice},
};
use serde::{Deserialize, Serialize};

use crate::{
  data_type::{AnimationDirection, LayerVisibility},
  error::PixquareLoaderError,
  image::{decode_image, encode_image},
  loader::{PixquareLoader, PixquareLoaderSettings, PxArtwork, PxFrameMeta, PxTagMeta},
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

  fn new(data_len: u64) -> Self {
    Self {
      format_version: Self::CURRENT_FORMAT_VERSION,
      flags: 0,
      data_len,
    }
  }

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

  async fn write_to(
    &self,
    writer: &mut bevy::asset::io::Writer,
  ) -> Result<(), PixquareLoaderError> {
    writer.write_all(Self::MAGIC).await?;
    writer.write_all(&self.format_version.to_le_bytes()).await?;
    writer.write_all(&self.flags.to_le_bytes()).await?;
    writer.write_all(&self.data_len.to_le_bytes()).await?;

    Ok(())
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

impl From<AnimationDirection> for ProcessedAnimationDirectionV1 {
  fn from(value: AnimationDirection) -> Self {
    match value {
      AnimationDirection::Forward => Self::Forward,
      AnimationDirection::Backward => Self::Backward,
      AnimationDirection::PingPong => Self::PingPong,
    }
  }
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
  async fn from_artwork(
    artwork: &PxArtwork,
    asset: &bevy::asset::saver::SavedAsset<'_, PxArtwork>,
  ) -> Result<(ProcessedPxArtworkV1, Vec<Vec<u8>>), PixquareLoaderError> {
    let mut frames = Vec::with_capacity(artwork.frame_count());
    let mut images = Vec::with_capacity(artwork.frame_count() * 2);
    let mut encoded_images = Vec::with_capacity(artwork.frame_count() * 2);

    for frame_index in 0..artwork.frame_count() {
      for visibility in [LayerVisibility::Visible, LayerVisibility::All] {
        let label = PxFrameMeta::generate_image_label(frame_index, visibility);
        let image = asset
          .get_labeled::<Image, str>(&label)
          .ok_or_else(|| PixquareLoaderError::MissingLabeledImageData(label.clone()))?;

        let encoded_image = encode_image(ImageCodec::Qoi, image.get())?;
        let image_meta = ProcessedImageMetaV1 {
          label: label.clone(),
          codec: ProcessedImageCodecV1::Qoi,
          encoded_len: encoded_image.len() as u64,
        };

        images.push(image_meta);
        encoded_images.push(encoded_image);
      }

      let duration = artwork.frame_duration(frame_index).ok_or_else(|| {
        PixquareLoaderError::FrameIndexOutOfBounds {
          frame_index,
          frame_count: artwork.frame_count(),
        }
      })?;
      let frame_meta = ProcessedPxFrameMetaV1 {
        visible_layer_image_index: (frame_index * 2) as u32,
        all_layer_image_index: (frame_index * 2 + 1) as u32,
        duration: duration.as_millis() as u32,
      };
      frames.push(frame_meta);
    }

    let tags = artwork
      .tags()
      .iter()
      .map(|tag| ProcessedPxTagMetaV1 {
        name: tag.name.clone(),
        start_index: tag.start_index,
        end_index: tag.end_index,
        direction: tag.direction.into(),
        loop_count: tag.loop_count,
      })
      .collect();

    let px_data = ProcessedPxArtworkV1 {
      canvas_width: artwork.canvas_size().x,
      canvas_height: artwork.canvas_size().y,
      frames,
      tags,
      images,
    };

    Ok((px_data, encoded_images))
  }

  async fn read(
    reader: &mut dyn bevy::asset::io::Reader,
    header: &ProcessedPxArtworkHeader,
    supported_compressed_formats: CompressedImageFormats,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<(Self, Vec<Handle<Image>>), PixquareLoaderError> {
    let mut px_artwork_data_buf = vec![0_u8; header.data_len as usize];
    reader.read_exact(&mut px_artwork_data_buf).await?;

    let px_artwork_data: ProcessedPxArtworkV1 = rmp_serde::from_slice(&px_artwork_data_buf)?;

    let mut image_handles = Vec::with_capacity(px_artwork_data.images.len());

    for image_meta in &px_artwork_data.images {
      let mut image_buf = vec![0_u8; image_meta.encoded_len as usize];
      reader.read_exact(&mut image_buf).await?;

      let image = decode_image(
        image_meta.codec.into(),
        &image_buf,
        supported_compressed_formats,
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

  type Settings = PixquareLoaderSettings;

  type Error = PixquareLoaderError;

  async fn load(
    &self,
    reader: &mut dyn bevy::asset::io::Reader,
    _settings: &Self::Settings,
    load_context: &mut bevy::asset::LoadContext<'_>,
  ) -> Result<Self::Asset, Self::Error> {
    let header = ProcessedPxArtworkHeader::read(reader).await?;

    match header.format_version {
      1 => {
        let (processed_v1, image_handles) = ProcessedPxArtworkV1::read(
          reader,
          &header,
          self.supported_compressed_formats,
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

#[derive(Debug, TypePath)]
struct PixquareSaver;

impl AssetSaver for PixquareSaver {
  type Asset = PxArtwork;

  type Settings = ();

  type OutputLoader = ProcessedPixquareLoader;

  type Error = PixquareLoaderError;

  async fn save(
    &self,
    writer: &mut bevy::asset::io::Writer,
    asset: bevy::asset::saver::SavedAsset<'_, Self::Asset>,
    _settings: &Self::Settings,
  ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
    let px_artwork = asset.get();
    let (processed_artwork, image_bufs) =
      ProcessedPxArtworkV1::from_artwork(px_artwork, &asset).await?;
    let artwork_buf = rmp_serde::to_vec_named(&processed_artwork)?;

    let header = ProcessedPxArtworkHeader::new(artwork_buf.len() as u64);
    header.write_to(writer).await?;

    writer.write_all(&artwork_buf).await?;

    for image_buf in &image_bufs {
      writer.write_all(image_buf).await?;
    }

    let first_image_label = PxFrameMeta::generate_image_label(0, LayerVisibility::All);
    let image = asset.get_labeled::<Image, str>(&first_image_label);

    let output_settings = match image {
      Some(image) => PixquareLoaderSettings {
        sampler: image.sampler.clone(),
        is_srgb: image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb,
        asset_usage: image.asset_usage,
      },
      None => PixquareLoaderSettings::default(),
    };

    Ok(output_settings)
  }
}

pub struct PixquareProcessorPlugin;

impl Plugin for PixquareProcessorPlugin {
  fn build(&self, app: &mut bevy::app::App) {
    app.register_asset_processor::<LoadTransformAndSave<
      PixquareLoader,
      IdentityAssetTransformer<PxArtwork>,
      PixquareSaver,
    >>(LoadTransformAndSave::new(
      IdentityAssetTransformer::new(),
      PixquareSaver,
    ));
    app.set_default_asset_processor::<LoadTransformAndSave<
      PixquareLoader,
      IdentityAssetTransformer<PxArtwork>,
      PixquareSaver,
    >>("px");
  }

  fn finish(&self, app: &mut bevy::app::App) {
    let supported_compressed_formats = match app.world().get_resource::<RenderDevice>() {
      Some(device) => CompressedImageFormats::from_features(device.features()),
      None => CompressedImageFormats::NONE,
    };

    app.register_asset_loader(ProcessedPixquareLoader {
      supported_compressed_formats,
    });
  }
}
