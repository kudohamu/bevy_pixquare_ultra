use bevy::ecs::{entity::Entity, event::Event};

use crate::data_type::FrameStep;

/// Event that fires .px file loaded and initialized.
#[derive(Debug, Event)]
pub struct PixquareFileInitializedEvent(pub Entity);

/// Event that fires animation loop finished.
/// This event is fired when the number of loops reaches loop_count.
#[derive(Debug, Event)]
pub struct AnimationFinishedEvent(pub Entity);

/// Event for manually advancing the animation frame.
///
/// Use [`Self::new`] to advance one frame, or [`Self::with_step`] to specify
/// granularity with [`FrameStep`]. Frame advancement follows the animation's
/// direction, tag range, and loop-count settings.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_pixquare_ultra::event::AdvanceAnimationFrameEvent;
///
/// fn advance(mut commands: Commands, entity: Entity) {
///   commands.trigger(AdvanceAnimationFrameEvent::new(entity));
/// }
/// ```
#[derive(Debug, Event)]
pub struct AdvanceAnimationFrameEvent {
  /// Target entity.
  pub(crate) entity: Entity,
  /// Granularity for how much to advance frames.
  pub(crate) step: FrameStep,
}

impl AdvanceAnimationFrameEvent {
  /// Constructor that advances by a fixed number of 1 frame.
  pub fn new(entity: Entity) -> Self {
    Self {
      entity,
      step: FrameStep::Fixed(1),
    }
  }

  /// Constructor that advances by specified granularity.
  pub fn with_step(entity: Entity, step: FrameStep) -> Self {
    Self { entity, step }
  }
}

/// Event for manually restarting the animation frame.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_pixquare_ultra::event::RestartFrameAnimationEvent;
///
/// fn advance(mut commands: Commands, entity: Entity) {
///   commands.trigger(RestartFrameAnimationEvent(entity));
/// }
/// ```
#[derive(Debug, Event)]
pub struct RestartFrameAnimationEvent(pub Entity);
