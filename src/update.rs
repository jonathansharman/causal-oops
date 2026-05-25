use bevy::prelude::*;

use crate::{
	control::{Action, Control},
	level::{ChangeMessage, Character, Id, Level},
};

/// The next character to act.
#[derive(Message, Clone, Copy)]
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
	mut controls: MessageReader<Control>,
	mut next_actors: MessageWriter<NextActor>,
	mut changes: MessageWriter<ChangeMessage>,
) {
	for control in controls.read() {
		match control {
			Control::Act(character_action) => {
				state.queue.push(*character_action);
				// If all characters have queued actions, execute the turn.
				if state.queue.len() == level.character_count() {
					let actions = Vec::from_iter(state.queue.drain(..));
					let change_event = level.update(actions);
					changes.write(change_event);
				}
			}
			Control::Undo => {
				if let Some(change) = level.undo() {
					state.queue.clear();
					changes.write(change);
				}
			}
			Control::Redo => {
				if let Some(change) = level.redo() {
					state.queue.clear();
					changes.write(change);
				}
			}
		}
		// Send the next actor to the control and animation systems.
		let (&id, &character) = level
			.characters_by_id()
			.nth(state.queue.len())
			.expect("character out of bounds");
		next_actors.write(NextActor { id, character });
	}
}
