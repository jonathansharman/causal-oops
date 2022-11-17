use bevy::{prelude::*, utils::HashMap};

use crate::level::{Direction, ID};

/// An action that can be performed by a character.
pub enum Action {
	Wait,
	Push(Direction),
}

/// The actions to be performed in a turn, by character ID.
#[derive(Resource, Deref, DerefMut)]
pub struct PendingActions(HashMap<ID, Action>);

impl PendingActions {
	pub fn new() -> Self {
		Self(HashMap::new())
	}
}
