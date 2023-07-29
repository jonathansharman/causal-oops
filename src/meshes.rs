use bevy::prelude::*;

pub const PORTAL_HEIGHT: f32 = 0.1;

#[derive(Resource)]
pub struct Meshes {
	pub character: Handle<Mesh>,
	pub portal: Handle<Mesh>,
}

impl Meshes {
	pub fn load(mesh_assets: &mut Assets<Mesh>) -> Self {
		Self {
			character: mesh_assets.add(
				Mesh::try_from(shape::Cylinder {
					radius: 0.5,
					height: 1.0,
					resolution: 3,
					..default()
				})
				.unwrap(),
			),
			portal: mesh_assets.add(
				Mesh::try_from(shape::Cylinder {
					radius: 0.5,
					height: PORTAL_HEIGHT,
					..default()
				})
				.unwrap(),
			),
		}
	}
}
