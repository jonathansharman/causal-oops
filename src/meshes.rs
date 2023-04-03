use bevy::prelude::*;

#[derive(Resource)]
pub struct Meshes {
	pub character: Handle<Mesh>,
}

impl Meshes {
	pub fn load(mesh_assets: &mut Assets<Mesh>) -> Self {
		Self {
			character: mesh_assets.add(
				Mesh::try_from(shape::Icosphere {
					radius: 0.5,
					subdivisions: 3,
				})
				.unwrap(),
			),
		}
	}
}
