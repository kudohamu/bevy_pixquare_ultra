use std::fmt::Display;

#[cfg(feature = "asset_processing")]
use bevy::math::UVec2;
use pixquare::error::{ArtworkOperationError, ParseError};

/// Represents errors that occur when loading a pixquare file.
#[derive(Debug)]
pub enum PixquareLoaderError {
  ArtworkOperationError(ArtworkOperationError),
  #[cfg(feature = "asset_processing")]
  FrameIndexOutOfBounds {
    frame_index: usize,
    frame_count: usize,
  },
  #[cfg(feature = "asset_processing")]
  ImageDimensionsMismatch {
    expected: UVec2,
    actual: UVec2,
  },
  #[cfg(feature = "asset_processing")]
  InvalidProcessingImageDataLength {
    expected: usize,
    actual: usize,
  },
  #[cfg(feature = "asset_processing")]
  MissingLabeledImageData(String),
  #[cfg(feature = "asset_processing")]
  MissingPorcessingTargetImageData,
  ParseError(String),
  #[cfg(feature = "asset_processing")]
  ProcessedPxArtworkDeserializationError(rmp_serde::decode::Error),
  #[cfg(feature = "asset_processing")]
  ProcessedPxArtworkLoadError,
  #[cfg(feature = "asset_processing")]
  QoiEncodeError(image::ImageError),
  #[cfg(feature = "asset_processing")]
  QoiDecodeError(bevy::image::TextureError),
  ReadError(std::io::Error),
  #[cfg(feature = "asset_processing")]
  RmpSerializeError(rmp_serde::encode::Error),
  #[cfg(feature = "asset_processing")]
  UnsupportedFormatVersionError,
  #[cfg(feature = "asset_processing")]
  UnsupportedProcessingTargetImageError,
}

impl Display for PixquareLoaderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ArtworkOperationError(err) => write!(f, "artwork operation error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::FrameIndexOutOfBounds {
        frame_index,
        frame_count,
      } => {
        write!(
          f,
          "frame index out of bounds: frame_index={frame_index} frame_count={frame_count}"
        )
      }
      #[cfg(feature = "asset_processing")]
      Self::ImageDimensionsMismatch { expected, actual } => {
        write!(
          f,
          "mismatched image dimensions: exptected={expected}, actual={actual}"
        )
      }
      #[cfg(feature = "asset_processing")]
      Self::InvalidProcessingImageDataLength { expected, actual } => {
        write!(
          f,
          "invalid length of processing image data: expected={expected} actual={actual}"
        )
      }
      #[cfg(feature = "asset_processing")]
      Self::MissingLabeledImageData(label) => {
        write!(f, "labeled image data is not found: {label}")
      }
      #[cfg(feature = "asset_processing")]
      Self::MissingPorcessingTargetImageData => write!(f, "processing image data is not found"),
      Self::ParseError(err) => write!(f, "parse file error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::ProcessedPxArtworkDeserializationError(err) => {
        write!(f, "processed px artwork deserializaiton error: {err}")
      }
      #[cfg(feature = "asset_processing")]
      Self::ProcessedPxArtworkLoadError => write!(f, "invalid processed px artwork data"),
      #[cfg(feature = "asset_processing")]
      Self::QoiEncodeError(err) => write!(f, "failed to encode image data to qoi format: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::QoiDecodeError(err) => write!(f, "failed to decode image data of qoi format: {err}"),
      Self::ReadError(err) => write!(f, "read file error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::RmpSerializeError(err) => {
        write!(f, "failed to serialize as message pack format: {err}")
      }
      #[cfg(feature = "asset_processing")]
      Self::UnsupportedFormatVersionError => write!(f, "unsupported format version"),
      #[cfg(feature = "asset_processing")]
      Self::UnsupportedProcessingTargetImageError => {
        write!(f, "unsupported processing target image")
      }
    }
  }
}

impl std::error::Error for PixquareLoaderError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::ArtworkOperationError(err) => Some(err),
      #[cfg(feature = "asset_processing")]
      Self::ProcessedPxArtworkDeserializationError(err) => Some(err),
      #[cfg(feature = "asset_processing")]
      Self::QoiDecodeError(err) => Some(err),
      Self::ReadError(err) => Some(err),
      _ => None,
    }
  }
}

impl From<std::io::Error> for PixquareLoaderError {
  fn from(err: std::io::Error) -> Self {
    Self::ReadError(err)
  }
}

impl<'a> From<ParseError<&'a [u8]>> for PixquareLoaderError {
  fn from(err: ParseError<&'a [u8]>) -> Self {
    Self::ParseError(err.to_string())
  }
}

impl From<ArtworkOperationError> for PixquareLoaderError {
  fn from(err: ArtworkOperationError) -> Self {
    Self::ArtworkOperationError(err)
  }
}

#[cfg(feature = "asset_processing")]
impl From<rmp_serde::decode::Error> for PixquareLoaderError {
  fn from(err: rmp_serde::decode::Error) -> Self {
    Self::ProcessedPxArtworkDeserializationError(err)
  }
}

#[cfg(feature = "asset_processing")]
impl From<image::ImageError> for PixquareLoaderError {
  fn from(err: image::ImageError) -> Self {
    Self::QoiEncodeError(err)
  }
}

#[cfg(feature = "asset_processing")]
impl From<bevy::image::TextureError> for PixquareLoaderError {
  fn from(err: bevy::image::TextureError) -> Self {
    Self::QoiDecodeError(err)
  }
}

#[cfg(feature = "asset_processing")]
impl From<rmp_serde::encode::Error> for PixquareLoaderError {
  fn from(err: rmp_serde::encode::Error) -> Self {
    Self::RmpSerializeError(err)
  }
}

/// Represents errors that occur when loading a Ron file for a texture atlas.
#[cfg(feature = "atlas_asset")]
#[derive(Debug)]
pub enum PxAtlasLoaderError {
  DeserializeError(ron::de::SpannedError),
  ReadError(std::io::Error),
}

#[cfg(feature = "atlas_asset")]
impl Display for PxAtlasLoaderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::DeserializeError(err) => write!(f, "deserialize ron file error: {err}"),
      Self::ReadError(err) => write!(f, "read file error: {err}"),
    }
  }
}

#[cfg(feature = "atlas_asset")]
impl std::error::Error for PxAtlasLoaderError {}

#[cfg(feature = "atlas_asset")]
impl From<std::io::Error> for PxAtlasLoaderError {
  fn from(err: std::io::Error) -> Self {
    Self::ReadError(err)
  }
}

#[cfg(feature = "atlas_asset")]
impl From<ron::de::SpannedError> for PxAtlasLoaderError {
  fn from(err: ron::de::SpannedError) -> Self {
    Self::DeserializeError(err)
  }
}
