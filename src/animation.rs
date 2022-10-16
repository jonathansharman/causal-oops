use bevy::prelude::Component;

use crate::level;

/// Animates an object in a level.
#[derive(Component)]
pub struct LevelObject {
	pub id: level::ID,
}
