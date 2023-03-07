use bevy::prelude::*;

#[derive(Resource)]
pub struct Meshes {
	pub character: Handle<Mesh>,
	pub block: Handle<Mesh>,
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
			block: mesh_assets.add(Mesh::from(shape::Cube { size: 1.0 })),
		}
	}
}
