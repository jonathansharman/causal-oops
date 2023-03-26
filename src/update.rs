use std::sync::Arc;

use bevy::prelude::*;

use crate::{
	control::Turn,
	level::{Abilities, Change, Id, Level},
};

/// Each character's abilities for the next turn.
#[derive(Resource, Deref, DerefMut)]
pub struct CharacterAbilities(Vec<(Id, Abilities)>);

impl CharacterAbilities {
	pub fn new(
		abilities: impl Into<Vec<(Id, Abilities)>>,
	) -> CharacterAbilities {
		CharacterAbilities(abilities.into())
	}
}

/// Consumes control events to update the level and produces change events.
pub fn update(
	mut level: ResMut<Level>,
	mut turn_events: EventReader<Turn>,
	mut character_abilities: ResMut<CharacterAbilities>,
	mut change_events: EventWriter<Arc<Change>>,
) {
	for turn in turn_events.into_iter() {
		match turn {
			Turn::Act(actions) => {
				let change = level.update(actions.iter().copied());
				*character_abilities =
					CharacterAbilities::new(level.character_abilities());
				change_events.send(change);
			}
			Turn::Undo => {
				if let Some(change) = level.undo() {
					*character_abilities =
						CharacterAbilities::new(level.character_abilities());
					change_events.send(change);
				}
			}
			Turn::Redo => {
				if let Some(change) = level.redo() {
					*character_abilities =
						CharacterAbilities::new(level.character_abilities());
					change_events.send(change);
				}
			}
		}
	}
}
