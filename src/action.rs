use std::collections::VecDeque;

use bevy::prelude::*;

use crate::level::{Offset, ID};

/// An action that can be performed by a character.
pub enum Action {
	Wait,
	Push(Offset),
}

/// The actions to be performed in a turn, by character ID.
#[derive(Resource, Deref, DerefMut)]
pub struct PendingActions(VecDeque<(ID, Action)>);

impl PendingActions {
	pub fn new() -> Self {
		Self(VecDeque::new())
	}
}
