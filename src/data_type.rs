use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
  Forward,
  Backward,
  PingPong,
}

impl AnimationDirection {
  pub(crate) fn from_px_direction(direction: pixquare::composite_type::AnimationDirection) -> Self {
    match direction {
      pixquare::composite_type::AnimationDirection::Forward => Self::Forward,
      pixquare::composite_type::AnimationDirection::Backward => Self::Backward,
      pixquare::composite_type::AnimationDirection::PingPong => Self::PingPong,
      pixquare::composite_type::AnimationDirection::Unknown(_) => Self::PingPong,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPlayState {
  Playing,
  Paused,
  Stopped,
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

impl std::fmt::Display for LayerVisibility {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Visible => write!(f, "visible"),
      Self::All => write!(f, "all"),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStep {
  Fixed(u16),
  Delta(Duration),
}
