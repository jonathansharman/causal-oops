use bevy::prelude::*;

#[derive(States, PartialEq, Eq, Debug, Clone, Hash, Default)]
pub enum GameState {
	#[default]
	Loading,
	CreatingLevel,
	Playing,
}
