use std::{sync::Arc, time::Duration};

use bevy::prelude::*;
use bevy_easings::{Ease, EaseFunction, EasingType};

use crate::{
	control::{Action, Turn},
	level::{Change, Id},
};

/// Animates an object in a level.
#[derive(Component)]
pub struct Object {
	pub id: Id,
}

const ANIMATION_DURATION: Duration = Duration::from_millis(200);

pub fn animate(
	mut commands: Commands,
	mut action_events: EventReader<Action>,
	mut turn_events: EventReader<Turn>,
	mut change_events: EventReader<Arc<Change>>,
	animation_query: Query<(Entity, &Transform, &Object)>,
) {
	// Add pending action indicators when the player queues character actions.
	for action in action_events.iter() {
		// TODO: Add pending action indicator.
		// match action {
		// 	Action::Wait => todo!(),
		// 	Action::Push(_) => todo!(),
		// 	Action::Summon(_) => todo!(),
		// 	Action::Return => todo!(),
		// }
	}
	// Remove pending action indicators when the player completes a turn.
	for _ in turn_events.iter() {
		// TODO: Remove pending action indicators.
	}
	// Animate level changes.
	for change in change_events.iter() {
		// Apply movements.
		for (entity, from, object) in &animation_query {
			let Some(mv) = change.moves.get(&object.id) else { continue };
			commands.entity(entity).insert(from.ease_to(
				Transform::from(mv.to),
				EaseFunction::CubicInOut,
				EasingType::Once {
					duration: ANIMATION_DURATION,
				},
			));
		}
	}
}
