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
			character: mesh_assets.add(Mesh::from(Cylinder {
				radius: 0.5,
				half_height: 0.5,
			})),
			portal: mesh_assets.add(Mesh::from(Cylinder {
				radius: 0.5,
				half_height: 0.5 * PORTAL_HEIGHT,
			})),
		}
	}
}
