use bevy::utils::HashMap;

use crate::level::{Coords, Level, ID};

/// A movement of an object from one tile to another.
pub struct Move {
	pub from: Coords,
	pub to: Coords,
}

/// A change from one [`Level`][crate::level::Level] state to another.
pub struct Change {
	pub moves: HashMap<ID, Move>,
}

impl Change {
	/// Applies this change to `level`.
	pub fn apply(&self, level: &mut Level) {
		for (id, mv) in self.moves.iter() {
			level.move_object(id, mv.from, mv.to);
		}
	}
}

