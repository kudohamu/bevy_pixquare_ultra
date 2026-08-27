use std::fmt::Display;

use bevy::math::UVec2;
use pixquare::error::{ArtworkOperationError, ParseError};

#[derive(Debug)]
pub enum PixquareLoaderError {
  ArtworkOperationError(ArtworkOperationError),
  #[cfg(feature = "asset_processing")]
  ImageDimensionsMismatch {
    expected: UVec2,
    actual: UVec2,
  },
  ParseError(String),
  #[cfg(feature = "asset_processing")]
  ProcessedPxArtworkDeserializationError(rmp_serde::decode::Error),
  #[cfg(feature = "asset_processing")]
  ProcessedPxArtworkLoadError,
  #[cfg(feature = "asset_processing")]
  QoiDecodeError(bevy::image::TextureError),
  ReadError(std::io::Error),
  #[cfg(feature = "asset_processing")]
  UnsupportedFormatVersionError,
}

impl Display for PixquareLoaderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ArtworkOperationError(err) => write!(f, "artwork operation error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::ImageDimensionsMismatch { expected, actual } => {
        write!(
          f,
          "mismatched image dimensions: exptected: {expected}, actual: {actual}"
        )
      }
      Self::ParseError(err) => write!(f, "parse file error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::ProcessedPxArtworkDeserializationError(err) => {
        write!(f, "processed px artwork deserializaiton error: {err}")
      }
      #[cfg(feature = "asset_processing")]
      Self::ProcessedPxArtworkLoadError => write!(f, "invalid processed px artwork data"),
      #[cfg(feature = "asset_processing")]
      Self::QoiDecodeError(err) => write!(f, "failed to decode image data of qoi format: {err}"),
      Self::ReadError(err) => write!(f, "read file error: {err}"),
      #[cfg(feature = "asset_processing")]
      Self::UnsupportedFormatVersionError => write!(f, "unsupported format version"),
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
impl From<bevy::image::TextureError> for PixquareLoaderError {
  fn from(err: bevy::image::TextureError) -> Self {
    Self::QoiDecodeError(err)
  }
}
