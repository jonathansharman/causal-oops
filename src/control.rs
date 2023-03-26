use std::collections::VecDeque;

use bevy::{
	input::{keyboard::KeyboardInput, ButtonState},
	prelude::*,
	utils::HashMap,
};

use crate::{
	level::{Id, Offset},
	update::CharacterAbilities,
};

/// An abstraction over keys and gamepad buttons.
#[derive(Clone, Copy)]
enum GameButton {
	Undo,
	Redo,
	Up,
	Left,
	Down,
	Right,
	Wait,
	Act,
}

/// Maps keys to game buttons.
struct KeyboardBindings(HashMap<KeyCode, GameButton>);

impl KeyboardBindings {
	/// Converts keyboard input events into game button events.
	fn adapt<'s, 'k>(
		&'s self,
		iter: impl IntoIterator<Item = &'k KeyboardInput> + 's,
	) -> impl Iterator<Item = (GameButton, ButtonState)> + 's + 'k
	where
		's: 'k,
	{
		iter.into_iter().filter_map(|input| {
			input
				.key_code
				.and_then(|key_code| self.0.get(&key_code))
				.map(|button| (*button, input.state))
		})
	}
}

impl Default for KeyboardBindings {
	fn default() -> KeyboardBindings {
		KeyboardBindings(HashMap::from([
			(KeyCode::Z, GameButton::Undo),
			(KeyCode::X, GameButton::Redo),
			(KeyCode::W, GameButton::Up),
			(KeyCode::Up, GameButton::Up),
			(KeyCode::A, GameButton::Left),
			(KeyCode::Left, GameButton::Left),
			(KeyCode::S, GameButton::Down),
			(KeyCode::Down, GameButton::Down),
			(KeyCode::D, GameButton::Right),
			(KeyCode::Right, GameButton::Right),
			(KeyCode::Space, GameButton::Wait),
			(KeyCode::LShift, GameButton::Act),
		]))
	}
}

/// An action that can be performed by a character.
#[derive(Clone, Copy)]
pub enum Action {
	Wait,
	Push(Offset),
	Summon(Offset),
	Return,
}

/// Each character's queued action for the next turn.
#[derive(Resource, Deref, DerefMut)]
pub struct CharacterActions(Vec<(Id, Action)>);

impl CharacterActions {
	pub fn new() -> CharacterActions {
		CharacterActions(Vec::new())
	}
}

/// The player's choice for a single turn.
pub enum Turn {
	Act(Vec<(Id, Action)>),
	Undo,
	Redo,
}

/// Used to handle control events that involve multiple input events, probably
/// over multiple frames.
#[derive(Default)]
pub struct ControlState {
	summoning: bool,
	events_buffer: VecDeque<(GameButton, ButtonState)>,
}

/// Consumes keyboard/gamepad input and produces higher-level control events to
/// be consumed by the update and animation systems.
pub fn control(
	mut keyboard_events: EventReader<KeyboardInput>,
	mut state: Local<ControlState>,
	character_abilities: Res<CharacterAbilities>,
	mut character_actions: ResMut<CharacterActions>,
	mut action_events: EventWriter<Action>,
	mut turn_events: EventWriter<Turn>,
) {
	// TODO: Custom input bindings
	let keybinds = KeyboardBindings::default();
	// After a full turn has been input, the control system needs to wait a
	// frame before processing the remaining input events so the update system
	// can update the level and generate the new list of character abilities.
	// Therefore, we need to buffer input events.
	state
		.events_buffer
		.extend(keybinds.adapt(&mut keyboard_events));

	while let Some((button, button_state)) = state.events_buffer.pop_front() {
		// Get the ID and abilities of the next character to act.
		let (id, abilities) = character_abilities[character_actions.len()];

		// Get the next action or else continue/return.
		let action = match (button, button_state) {
			(GameButton::Undo, ButtonState::Pressed) => {
				character_actions.clear();
				turn_events.send(Turn::Undo);
				return;
			}
			(GameButton::Redo, ButtonState::Pressed) => {
				character_actions.clear();
				turn_events.send(Turn::Redo);
				return;
			}
			(GameButton::Up, ButtonState::Pressed) => {
				if state.summoning {
					Action::Summon(Offset::UP)
				} else if abilities.can_push {
					Action::Push(Offset::UP)
				} else {
					continue;
				}
			}
			(GameButton::Left, ButtonState::Pressed) => {
				if state.summoning {
					Action::Summon(Offset::LEFT)
				} else if abilities.can_push {
					Action::Push(Offset::LEFT)
				} else {
					continue;
				}
			}
			(GameButton::Down, ButtonState::Pressed) => {
				if state.summoning {
					Action::Summon(Offset::DOWN)
				} else if abilities.can_push {
					Action::Push(Offset::DOWN)
				} else {
					continue;
				}
			}
			(GameButton::Right, ButtonState::Pressed) => {
				if state.summoning {
					Action::Summon(Offset::RIGHT)
				} else if abilities.can_push {
					Action::Push(Offset::RIGHT)
				} else {
					continue;
				}
			}
			(GameButton::Wait, ButtonState::Pressed) => Action::Wait,
			(GameButton::Act, ButtonState::Pressed) => {
				// The Act button is contextual. If the next character has the
				// ability to return, it's the return button. If it has the
				// ability to summon, it's a modifier button.
				if abilities.can_return {
					Action::Return
				} else {
					if abilities.can_summon {
						state.summoning = true;
					}
					continue;
				}
			}
			(GameButton::Act, ButtonState::Released) => {
				state.summoning = false;
				continue;
			}
			_ => continue,
		};
		action_events.send(action);
		character_actions.push((id, action));
		state.summoning = false;

		// If all characters have queued actions, send the turn.
		if character_actions.len() == character_abilities.len() {
			let actions = Vec::from_iter(character_actions.drain(..));
			turn_events.send(Turn::Act(actions));
			return;
		}
	}
}
