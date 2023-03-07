use bevy::prelude::States;

/// The state of the game.
#[derive(States, Clone, PartialEq, Eq, Debug, Hash, Default)]
pub enum GameState {
	/// Receiving player input
	#[default]
	Control,
	/// Animating level change
	Animate,
}
