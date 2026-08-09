use std::collections::VecDeque;

use bevy::{
	input::{ButtonState, keyboard::KeyboardInput},
	platform::collections::HashMap,
	prelude::*,
};

use crate::{
	level::{Id, Level, Offset},
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
	Descend,
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
	// The act button is multipurpose. This tracks whether the act button has
	// been pressed but not yet released or handled.
	modifier: bool,
}

/// Consumes keyboard/gamepad input and produces higher-level control events to
/// be consumed by the update and animation systems.
pub fn control(
	level: Res<Level>,
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
	if let Some(next_actor) = next_actors.read().next() {
		state.next_actor = Some(*next_actor);
	}
	// Get the next actor or return if there's no actor to control.
	let Some(NextActor { actor }) = state.next_actor else {
		return;
	};

	let act = |action: Action| -> Option<Control> {
		Some(Control::Act((actor.id, action)))
	};

	// Consume buffered input until a control message is received.
	while let Some((button, button_state)) = state.input_buffer.pop_front() {
		let mut go = |offset: Offset| {
			let modifier = state.modifier;
			state.modifier = false;
			if actor.can_summon() && modifier {
				act(Action::Summon(offset))
			} else if actor.can_push() {
				act(Action::Push(offset))
			} else {
				None
			}
		};

		// Get the next control and/or update internal state.
		let control = match (button, button_state) {
			(GameButton::Undo, ButtonState::Pressed) => Some(Control::Undo),
			(GameButton::Redo, ButtonState::Pressed) => Some(Control::Redo),
			(GameButton::Up, ButtonState::Pressed) => go(Offset::UP),
			(GameButton::Left, ButtonState::Pressed) => go(Offset::LEFT),
			(GameButton::Down, ButtonState::Pressed) => go(Offset::DOWN),
			(GameButton::Right, ButtonState::Pressed) => go(Offset::RIGHT),
			(GameButton::Wait, ButtonState::Pressed) => act(Action::Wait),
			(GameButton::Act, ButtonState::Pressed) => {
				// We don't know yet whether the act button will be used to
				// summon, return, or descend (or do nothing).
				state.modifier = true;
				None
			}
			(GameButton::Act, ButtonState::Released) => {
				if state.modifier {
					state.modifier = false;
					if actor.can_return() {
						act(Action::Return)
					} else if actor.can_descend(&level) {
						act(Action::Descend)
					} else {
						None
					}
				} else {
					// The act button press already modified a direction.
					None
				}
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
