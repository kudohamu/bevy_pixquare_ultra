use bevy::image::{
  CompressedImageFormats, Image, ImageFormat, ImageLoaderSettings, ImageSampler, ImageType,
};

use crate::{error::PixquareLoaderError, processor::ImageCodec};

pub fn decode_image(
  codec: ImageCodec,
  encoded_buf: &[u8],
  supported_compressed_formats: CompressedImageFormats,
  settings: &ImageLoaderSettings,
) -> Result<Image, PixquareLoaderError> {
  let format = match codec {
    ImageCodec::Qoi => ImageFormat::Qoi,
  };

  let image = Image::from_buffer(
    encoded_buf,
    ImageType::Format(format),
    supported_compressed_formats,
    settings.is_srgb,
    ImageSampler::nearest(),
    settings.asset_usage,
  )?;

  Ok(image)
}
