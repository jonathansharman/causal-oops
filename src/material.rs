use bevy::prelude::*;

#[derive(Resource)]
pub struct Materials {
	pub character: Handle<StandardMaterial>,
	pub wood: Handle<StandardMaterial>,
	pub floor: Handle<StandardMaterial>,
	pub wall: Handle<StandardMaterial>,
}

impl Materials {
	pub fn load(material_assets: &mut Assets<StandardMaterial>) -> Self {
		Self {
			character: material_assets.add(Color::rgb(0.2, 0.7, 0.2).into()),
			wood: material_assets.add(Color::rgb(0.8, 0.6, 0.4).into()),
			floor: material_assets.add(Color::rgb(0.5, 0.4, 0.3).into()),
			wall: material_assets.add(Color::rgb(0.5, 0.1, 0.1).into()),
		}
	}
}
