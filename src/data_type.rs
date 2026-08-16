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
