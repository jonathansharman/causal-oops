use std::collections::VecDeque;

use bevy::{
	input::{ButtonState, keyboard::KeyboardInput},
	platform::collections::HashMap,
	prelude::*,
};

use crate::{
	level::{Id, Offset},
	update::NextActor,
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
			self.0
				.get(&input.key_code)
				.map(|button| (*button, input.state))
		})
	}
}

impl Default for KeyboardBindings {
	fn default() -> KeyboardBindings {
		KeyboardBindings(HashMap::from([
			(KeyCode::KeyZ, GameButton::Undo),
			(KeyCode::KeyX, GameButton::Redo),
			(KeyCode::KeyW, GameButton::Up),
			(KeyCode::ArrowUp, GameButton::Up),
			(KeyCode::KeyA, GameButton::Left),
			(KeyCode::ArrowLeft, GameButton::Left),
			(KeyCode::KeyS, GameButton::Down),
			(KeyCode::ArrowDown, GameButton::Down),
			(KeyCode::KeyD, GameButton::Right),
			(KeyCode::ArrowRight, GameButton::Right),
			(KeyCode::Space, GameButton::Wait),
			(KeyCode::ShiftLeft, GameButton::Act),
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

#[derive(Message)]
pub enum Control {
	Act((Id, Action)),
	Undo,
	Redo,
}

/// Local state for the control system, for handling multi-input/multi-frame
/// controls.
#[derive(Default)]
pub struct ControlState {
	input_buffer: VecDeque<(GameButton, ButtonState)>,
	next_actor: Option<NextActor>,
	act_button_held: bool,
}

/// Consumes keyboard/gamepad input and produces higher-level control events to
/// be consumed by the update and animation systems.
pub fn control(
	mut state: Local<ControlState>,
	mut keyboard_messsages: MessageReader<KeyboardInput>,
	mut next_actors: MessageReader<NextActor>,
	mut controls: MessageWriter<Control>,
) {
	// TODO: Make this a resource and support custom input bindings.
	let keybinds = KeyboardBindings::default();
	// Buffer inputs so that update and animation systems can run after each
	// control message.
	state
		.input_buffer
		.extend(keybinds.adapt(&mut keyboard_messsages.read()));

	// Set the next actor if there is one. There should be at most one next
	// actor per frame.
	if let Some(next_actor) = next_actors.read().last() {
		state.next_actor = Some(*next_actor);
	}
	// Get the next actor or return if there's no actor to control.
	let Some(actor) = state.next_actor else {
		return;
	};

	let act = |action: Action| -> Option<Control> {
		Some(Control::Act((actor.id, action)))
	};

	// Consume buffered input until a control message is received.
	while let Some((button, button_state)) = state.input_buffer.pop_front() {
		// Get the next control and/or update internal state.
		let control = match (button, button_state) {
			(GameButton::Undo, ButtonState::Pressed) => Some(Control::Undo),
			(GameButton::Redo, ButtonState::Pressed) => Some(Control::Redo),
			(GameButton::Up, ButtonState::Pressed) => {
				if actor.character.can_summon() && state.act_button_held {
					act(Action::Summon(Offset::UP))
				} else if actor.character.can_push() {
					act(Action::Push(Offset::UP))
				} else {
					None
				}
			}
			(GameButton::Left, ButtonState::Pressed) => {
				if actor.character.can_summon() && state.act_button_held {
					act(Action::Summon(Offset::LEFT))
				} else if actor.character.can_push() {
					act(Action::Push(Offset::LEFT))
				} else {
					None
				}
			}
			(GameButton::Down, ButtonState::Pressed) => {
				if actor.character.can_summon() && state.act_button_held {
					act(Action::Summon(Offset::DOWN))
				} else if actor.character.can_push() {
					act(Action::Push(Offset::DOWN))
				} else {
					None
				}
			}
			(GameButton::Right, ButtonState::Pressed) => {
				if actor.character.can_summon() && state.act_button_held {
					act(Action::Summon(Offset::RIGHT))
				} else if actor.character.can_push() {
					act(Action::Push(Offset::RIGHT))
				} else {
					None
				}
			}
			(GameButton::Wait, ButtonState::Pressed) => act(Action::Wait),
			(GameButton::Act, ButtonState::Pressed) => {
				// The Act button is contextual. If the actor has the ability to
				// return, it's the return button. If it has the ability to
				// summon, it's a modifier button.
				if !state.act_button_held {
					state.act_button_held = true;
					actor
						.character
						.can_return()
						.then(|| act(Action::Return))
						.flatten()
				} else {
					None
				}
			}
			(GameButton::Act, ButtonState::Released) => {
				state.act_button_held = false;
				None
			}
			_ => None,
		};
		// If there was a control message, write it, reset the next actor, and
		// return so that the update and animation systems can respond.
		if let Some(control) = control {
			state.next_actor = None;
			controls.write(control);
			return;
		}
	}
}
