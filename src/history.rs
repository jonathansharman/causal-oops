use bevy::utils::HashMap;

use crate::level::{Coords, ID};

/// A movement of an object from one tile to another.
pub struct Move {
	pub from: Coords,
	pub to: Coords,
}

/// A change from one [`Level`][crate::level::Level] state to another.
pub struct Change {
	pub moves: HashMap<ID, Move>,
}

