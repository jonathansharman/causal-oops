use bevy::prelude::*;
use level::{Coords, Level, Object, Tile};

mod history;
mod level;

fn main() {
	App::new()
		.add_startup_system(setup)
		.insert_resource(WindowDescriptor {
			title: "Causal Oops".to_string(),
			width: 800.0,
			height: 600.0,
			..Default::default()
		})
		.insert_resource(ClearColor(Color::BLACK))
		.add_plugins(DefaultPlugins)
		.run();
}

fn spawn_level(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	level: &Level,
) {
	// Create meshes.
	let character_mesh = meshes.add(Mesh::from(shape::Icosphere {
		radius: 0.5,
		subdivisions: 3,
	}));
	let block_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

	// Create materials.
	let character_material = materials.add(Color::rgb(0.2, 0.7, 0.2).into());
	let crate_material = materials.add(Color::rgb(0.8, 0.6, 0.4).into());
	let floor_material = materials.add(Color::rgb(0.5, 0.4, 0.3).into());
	let wall_material = materials.add(Color::rgb(0.5, 0.1, 0.1).into());

	// Spawn tile entities.
	for row in 0..level.height() {
		for col in 0..level.width() {
			match level.tile(Coords::new(row, col)) {
				Tile::Floor => commands.spawn_bundle(PbrBundle {
					mesh: block_mesh.clone(),
					material: floor_material.clone(),
					transform: Transform::from_xyz(
						col as f32, -0.5, row as f32,
					),
					..default()
				}),
				Tile::Wall => commands.spawn_bundle(PbrBundle {
					mesh: block_mesh.clone(),
					material: wall_material.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				}),
			};
		}
	}

	// Spawn object entities.
	for (coords, level_object) in level.objects() {
		match level_object.object {
			Object::Character => commands.spawn_bundle(PbrBundle {
				mesh: character_mesh.clone(),
				material: character_material.clone(),
				transform: Transform::from_xyz(
					coords.col as f32,
					0.5,
					coords.row as f32,
				),
				..default()
			}),
			Object::Crate => commands.spawn_bundle(PbrBundle {
				mesh: block_mesh.clone(),
				material: crate_material.clone(),
				transform: Transform::from_xyz(
					coords.col as f32,
					0.5,
					coords.row as f32,
				),
				..default()
			}),
		};
	}
}

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let level = level::test_level();
	spawn_level(&mut commands, &mut meshes, &mut materials, &level);
	commands.insert_resource(level);

	// Add lighting.
	commands.spawn_bundle(PointLightBundle {
		point_light: PointLight {
			intensity: 1500.0,
			shadows_enabled: true,
			..default()
		},
		transform: Transform::from_xyz(4.0, 8.0, 4.0),
		..default()
	});

	// Add static camera overlooking the level.
	commands.spawn_bundle(Camera3dBundle {
		transform: Transform::from_xyz(2.0, 5.0, 5.0)
			.looking_at(Vec3::new(2.0, 0.0, 2.0), Vec3::Y),
		..default()
	});
}
