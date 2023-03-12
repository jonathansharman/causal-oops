use bevy::prelude::Component;

use crate::level;

/// Animates an object in a level.
#[derive(Component)]
pub struct Object {
	pub id: level::Id,
}
