use bevy::prelude::*;

use crate::level::CharacterColor;

#[derive(Resource)]
pub struct Materials {
	pub characters: [Handle<StandardMaterial>; CharacterColor::COUNT],
	pub indicator: Handle<StandardMaterial>,
}

impl Materials {
	pub fn load(material_assets: &mut Assets<StandardMaterial>) -> Self {
		Self {
			characters: std::array::from_fn(|idx| {
				material_assets.add(CharacterColor::from(idx as u8).color())
			}),
			indicator: material_assets.add(Color::WHITE),
		}
	}
}
