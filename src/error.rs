use std::fmt::Display;

use pixquare::error::{ArtworkOperationError, ParseError};

#[derive(Debug)]
pub enum PixquareLoaderError {
  ReadError(std::io::Error),
  ParseError(String),
  ArtworkOperationError(ArtworkOperationError),
}

impl Display for PixquareLoaderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ReadError(err) => write!(f, "Read file error: {err}"),
      Self::ParseError(err) => write!(f, "Parse file error: {err}"),
      Self::ArtworkOperationError(err) => write!(f, "artwork operation error: {err}"),
    }
  }
}

impl std::error::Error for PixquareLoaderError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::ReadError(err) => Some(err),
      Self::ArtworkOperationError(err) => Some(err),
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
