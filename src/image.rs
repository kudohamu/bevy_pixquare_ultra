use bevy::{
  image::{CompressedImageFormats, Image, ImageFormat, ImageType},
  render::render_resource::TextureFormat,
};
use image::{ExtendedColorType, ImageEncoder, codecs::qoi::QoiEncoder};

use crate::{error::PixquareLoaderError, loader::PixquareLoaderSettings, processor::ImageCodec};

pub fn encode_image(codec: ImageCodec, image: &Image) -> Result<Vec<u8>, PixquareLoaderError> {
  if image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
    && image.texture_descriptor.format != TextureFormat::Rgba8Unorm
  {
    return Err(PixquareLoaderError::UnsupportedProcessingTargetImageError);
  }

  let image_width = image.width();
  let image_height = image.height();
  let image_data = image
    .data
    .as_ref()
    .ok_or(PixquareLoaderError::MissingPorcessingTargetImageData)?;
  let expected_data_len = image_width as usize * image_height as usize * 4;
  if image_data.len() != expected_data_len {
    return Err(PixquareLoaderError::InvalidProcessingImageDataLength {
      expected: expected_data_len,
      actual: image_data.len(),
    });
  }

  let mut buf = Vec::new();
  match codec {
    ImageCodec::Qoi => {
      QoiEncoder::new(&mut buf).write_image(
        &image_data,
        image_width,
        image_height,
        ExtendedColorType::Rgba8,
      )?;
    }
  }

  Ok(buf)
}

pub fn decode_image(
  codec: ImageCodec,
  encoded_buf: &[u8],
  supported_compressed_formats: CompressedImageFormats,
  settings: &PixquareLoaderSettings,
) -> Result<Image, PixquareLoaderError> {
  let format = match codec {
    ImageCodec::Qoi => ImageFormat::Qoi,
  };

  let image = Image::from_buffer(
    encoded_buf,
    ImageType::Format(format),
    supported_compressed_formats,
    settings.is_srgb,
    settings.sampler.clone(),
    settings.asset_usage,
  )?;

  Ok(image)
}
