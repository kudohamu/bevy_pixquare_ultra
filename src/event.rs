use bevy::ecs::{entity::Entity, event::Event};

#[derive(Event)]
pub struct PixquareFileInitializedEvent(pub Entity);
