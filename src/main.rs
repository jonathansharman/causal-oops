use bevy::{prelude::*, utils::HashMap};
use iyes_loopless::prelude::*;

use history::{Change, Move};
use level::{Coords, Level, Object, Tile, ID};
use state::State;

mod animation;
mod history;
mod level;
mod state;

fn main() {
	App::new()
		.add_startup_system(setup)
		.add_system(control.run_in_state(State::Control))
		.add_system(animate.run_in_state(State::Animate))
		.add_loopless_state(State::Control)
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
	for level_object in level.iter_objects() {
		let Coords { row, col } = level_object.coords;
		match level_object.object {
			Object::Character => commands
				.spawn_bundle(PbrBundle {
					mesh: character_mesh.clone(),
					material: character_material.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				})
				.insert(animation::Object {
					id: level_object.id,
				}),
			Object::Crate => commands
				.spawn_bundle(PbrBundle {
					mesh: block_mesh.clone(),
					material: crate_material.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				})
				.insert(animation::Object {
					id: level_object.id,
				}),
		};
	}
}

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	// Create level.
	let level = level::test_level();
	spawn_level(&mut commands, &mut meshes, &mut materials, &level);
	commands.insert_resource(level);

	// Create an empty change.
	commands.insert_resource(Change {
		moves: HashMap::new(),
	});

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

fn control(
	mut commands: Commands,
	input: Res<Input<KeyCode>>,
	mut level: ResMut<Level>,
	mut change: ResMut<Change>,
) {
	let (mut d_row, mut d_col): (i8, i8) = (0, 0);
	if input.just_pressed(KeyCode::Left) {
		d_col -= 1;
	}
	if input.just_pressed(KeyCode::Right) {
		d_col += 1;
	}
	if input.just_pressed(KeyCode::Up) {
		d_row -= 1;
	}
	if input.just_pressed(KeyCode::Down) {
		d_row += 1;
	}

	if d_col != 0 || d_row != 0 {
		// TODO: This assumes there's always exactly one character, with ID 0.
		let id = ID(0);
		let object = level.get_object_mut(&id).unwrap();
		let old_coords = object.coords;
		if d_row == -1 {
			object.coords.row -= 1
		} else if d_row == 1 {
			object.coords.row += 1
		}
		if d_col == -1 {
			object.coords.col -= 1
		} else if d_col == 1 {
			object.coords.col += 1
		}
		change.moves.insert(
			object.id,
			Move {
				from: old_coords,
				to: object.coords,
			},
		);
		commands.insert_resource(NextState(State::Animate));
	}
}

fn animate(
	mut commands: Commands,
	mut query: Query<(&animation::Object, &mut Transform)>,
	mut change: ResMut<Change>,
) {
	// Apply movements.
	for (object, mut transform) in &mut query {
		if let Some(mv) = change.moves.get(&object.id) {
			let Coords { row, col } = mv.to;
			*transform = Transform::from_xyz(col as f32, 0.5, row as f32)
		}
	}
	change.moves.clear();
	commands.insert_resource(NextState(State::Control));
}
