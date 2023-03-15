use bevy::prelude::*;

#[derive(Resource)]
pub struct Materials {
	pub characters: [Handle<StandardMaterial>; 3],
}

impl Materials {
	pub fn load(material_assets: &mut Assets<StandardMaterial>) -> Self {
		Self {
			characters: [
				material_assets.add(Color::rgb(0.2, 0.7, 0.2).into()),
				material_assets.add(Color::rgb(0.7, 0.2, 0.2).into()),
				material_assets.add(Color::rgb(0.2, 0.2, 0.7).into()),
			],
		}
	}
}
