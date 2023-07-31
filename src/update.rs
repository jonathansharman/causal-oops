use bevy::prelude::*;

use crate::{
	control::{Action, ControlEvent},
	level::{ChangeEvent, Character, Id, Level},
};

/// The next character to act.
#[derive(Event, Clone, Copy)]
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
	mut change_events: EventWriter<ChangeEvent>,
) {
	for control_event in control_events.iter() {
		match control_event {
			ControlEvent::Act(character_action) => {
				state.queue.push(*character_action);
				// If all characters have queued actions, execute the turn.
				if state.queue.len() == level.character_count() {
					let actions = Vec::from_iter(state.queue.drain(..));
					let change_event = level.update(actions);
					change_events.send(change_event);
				}
			}
			ControlEvent::Undo => {
				if let Some(change) = level.undo() {
					state.queue.clear();
					change_events.send(change);
				}
			}
			ControlEvent::Redo => {
				if let Some(change_event) = level.redo() {
					state.queue.clear();
					change_events.send(change_event);
				}
			}
		}
		// Send the next actor to the control and animation systems.
		let (&id, &character) = level
			.characters()
			.nth(state.queue.len())
			.expect("character out of bounds");
		next_actors.send(NextActor { id, character });
	}
}
