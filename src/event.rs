use bevy::ecs::{entity::Entity, event::Event};

use crate::data_type::FrameStep;

#[derive(Debug, Event)]
pub struct PixquareFileInitializedEvent(pub Entity);

#[derive(Debug, Event)]
pub struct AnimationLoopFinishedEvent(pub Entity);

#[derive(Debug, Event)]
pub struct AdvanceAnimationFrameEvent {
  pub(crate) entity: Entity,
  pub(crate) step: FrameStep,
}

impl AdvanceAnimationFrameEvent {
  pub fn new(entity: Entity) -> Self {
    Self {
      entity,
      step: FrameStep::Fixed(1),
    }
  }

  pub fn with_step(entity: Entity, step: FrameStep) -> Self {
    Self { entity, step }
  }
}

#[derive(Debug, Event)]
pub struct RestartFrameAnimationEvent(pub Entity);
