#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
  Forward,
  Backward,
  PingPong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPlayState {
  Playing,
  Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerVisibility {
  Visible,
  All,
}

impl From<LayerVisibility> for pixquare::utility_type::LayerVisibility {
  fn from(value: LayerVisibility) -> Self {
    match value {
      LayerVisibility::Visible => pixquare::utility_type::LayerVisibility::Visible,
      LayerVisibility::All => pixquare::utility_type::LayerVisibility::All,
    }
  }
}
