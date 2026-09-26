use bevy::ecs::{entity::Entity, event::Event};

#[derive(Debug, Event)]
pub struct PixquareFileInitializedEvent(pub Entity);

#[derive(Debug, Event)]
pub struct AnimationLoopFinishedEvent(pub Entity);

#[derive(Debug, Event)]
pub struct AdvanceAnimationFrameEvent(pub Entity);

#[derive(Debug, Event)]
pub struct RestartFrameAnimationEvent(pub Entity);
