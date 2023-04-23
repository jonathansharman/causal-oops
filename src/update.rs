use std::sync::Arc;

use bevy::prelude::*;

use crate::{
	control::{Action, ControlEvent},
	level::{Change, Character, Id, Level},
};

/// The next character to act.
#[derive(Clone, Copy)]
pub struct NextActor {
	pub id: Id,
	pub character: Character,
}

/// Local state for the update system, to store queued actions.
#[derive(Default)]
pub struct UpdateState {
	/// Each character's queued action for the next turn.
	queue: Vec<(Id, Action)>,
}

/// Consumes control events to update the level and produces change events.
pub fn update(
	mut state: Local<UpdateState>,
	mut level: ResMut<Level>,
	mut control_events: EventReader<ControlEvent>,
	mut next_actors: EventWriter<NextActor>,
	mut change_events: EventWriter<Arc<Change>>,
) {
	for control_event in control_events.iter() {
		match control_event {
			ControlEvent::Act(character_action) => {
				state.queue.push(*character_action);
				// If all characters have queued actions, execute the turn.
				if state.queue.len() == level.characters().len() {
					let actions = Vec::from_iter(state.queue.drain(..));

					let change = level.update(actions.iter().copied());
					change_events.send(change);
				}
			}
			ControlEvent::Undo => {
				if let Some(change) = level.undo() {
					state.queue.clear();
					change_events.send(change);
				}
			}
			ControlEvent::Redo => {
				if let Some(change) = level.redo() {
					state.queue.clear();
					change_events.send(change);
				}
			}
		}
		// Send the next actor to the control and animation systems.
		let (id, character) = level.characters()[state.queue.len()];
		next_actors.send(NextActor { id, character });
	}
}
