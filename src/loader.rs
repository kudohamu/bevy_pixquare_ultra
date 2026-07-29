use std::fmt::Display;

use bevy::{
  app::{App, Plugin},
  asset::{Asset, AssetApp, AssetLoader},
  log::debug,
  reflect::TypePath,
};
use pixquare::{error::ParseError, model::Artwork};

#[derive(Debug, Asset, TypePath)]
pub struct PxArtwork(pub Artwork);

#[derive(Debug, TypePath)]
struct PixquareLoader;

#[derive(Debug)]
enum PixquareLoaderError {
  ReadError(std::io::Error),
  ParseError(String),
}

impl Display for PixquareLoaderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ReadError(err) => write!(f, "Read file error: {err}"),
      Self::ParseError(err) => write!(f, "Parse file error: {err}"),
    }
  }
}

impl std::error::Error for PixquareLoaderError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
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

impl AssetLoader for PixquareLoader {
  type Asset = PxArtwork;
  type Settings = ();
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

    debug!("loaded");
    Ok(PxArtwork(artwork))
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
