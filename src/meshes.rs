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
			character: mesh_assets.add(Mesh::from(Extrusion::new(
				Triangle2d::new(
					Vec2::new(-0.5, -0.5),
					0.5 * Vec2::X,
					Vec2::new(-0.5, 0.5),
				),
				1.0,
			))),
			portal: mesh_assets.add(Mesh::from(Extrusion::new(
				Circle { radius: 0.5 },
				PORTAL_HEIGHT,
			))),
		}
	}
}
