use std::time::Duration;

/// Represents direction of animation.
/// Variants correspond to the value that can be specified in Pixquare.
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

/// Represents playback state of the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPlayState {
  /// Playing animation.
  Playing,
  /// Paused animation.
  Paused,
  /// Stopped animation.
  /// Specifying this variant resets the internal animation state.
  Stopped,
}

/// Represents which layers to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerVisibility {
  /// Only visible layers.
  Visible,
  /// All layers. Include invisible layers.
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

/// Represents the frame advancement granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStep {
  /// Advances by a fixed number of frames.
  Fixed(u16),
  /// Advances by the number of frames corresponding to the given duration.
  Delta(Duration),
}
