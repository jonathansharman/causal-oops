use crate::level::{Direction, ID};

/// An [`Action`] performed by a character.
pub struct CharacterAction {
	pub id: ID,
	pub action: Action,
}

/// An action that can be performed by a character.
pub enum Action {
	/// Moving or pushing in some direction.
	Push(Direction),
}
