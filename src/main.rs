use std::sync::Arc;

use bevy::prelude::*;
use bevy_easings::EasingsPlugin;

use control::{Action, CharacterActions, Turn};
use level::{Change, Coords, Level, Object, Tile};
use material::Materials;
use mesh::Meshes;
use models::Models;
use update::CharacterAbilities;

mod animation;
mod control;
mod level;
mod material;
mod mesh;
mod models;
mod update;

fn main() {
	App::new()
		.add_startup_system(setup)
		.add_systems(
			(control::control, update::update, animation::animate).chain(),
		)
		.add_event::<Action>()
		.add_event::<Turn>()
		.add_event::<Arc<Change>>()
		.insert_resource(ClearColor(Color::BLACK))
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Causal Oops".to_string(),
				resolution: (800.0, 600.0).into(),
				..default()
			}),
			..default()
		}))
		.add_plugin(EasingsPlugin)
		.run();
}

fn spawn_level(
	commands: &mut Commands,
	models: &Models,
	meshes: &Meshes,
	materials: &Materials,
	level: &Level,
) {
	// Spawn tile entities.
	for row in 0..level.height() {
		for col in 0..level.width() {
			match level.tile(Coords::new(row as i32, col as i32)) {
				Tile::Floor => commands.spawn(SceneBundle {
					scene: models.floor.clone(),
					transform: Transform::from_xyz(
						col as f32, -0.5, row as f32,
					),
					..default()
				}),
				Tile::Wall => commands.spawn(SceneBundle {
					scene: models.wall.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				}),
			};
		}
	}

	// Spawn object entities.
	for level_object in level.iter_objects() {
		let Coords { row, col } = level_object.coords;
		let transform = Transform::from_xyz(col as f32, 0.5, row as f32);
		let object_animation = animation::Object {
			id: level_object.id,
		};
		match level_object.object {
			Object::Character { idx } => commands.spawn((
				PbrBundle {
					mesh: meshes.character.clone(),
					material: materials.characters[idx].clone(),
					transform,
					..default()
				},
				object_animation,
			)),
			Object::WoodenCrate => commands.spawn((
				SceneBundle {
					scene: models.wooden_crate.clone(),
					transform,
					..default()
				},
				object_animation,
			)),
			Object::SteelCrate => commands.spawn((
				SceneBundle {
					scene: models.steel_crate.clone(),
					transform,
					..default()
				},
				object_animation,
			)),
			Object::StoneBlock => commands.spawn((
				SceneBundle {
					scene: models.stone_block.clone(),
					transform,
					..default()
				},
				object_animation,
			)),
		};
	}
}

fn setup(
	mut commands: Commands,
	mut asset_server: ResMut<AssetServer>,
	mut mesh_assets: ResMut<Assets<Mesh>>,
	mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
	// Load models, meshes, and materials.
	let models = Models::load(&mut asset_server);
	let meshes = Meshes::load(&mut mesh_assets);
	let materials = Materials::load(&mut material_assets);

	// Create and spawn level.
	let level = level::test_level();
	commands
		.insert_resource(CharacterAbilities::new(level.character_abilities()));
	commands.insert_resource(CharacterActions::new());
	spawn_level(&mut commands, &models, &meshes, &materials, &level);

	// Insert mesh and material resources.
	commands.insert_resource(meshes);
	commands.insert_resource(materials);

	// Add static camera overlooking the level.
	let center_x = (level.width() as f32 - 1.0) / 2.0;
	let center_z = (level.height() as f32 - 1.0) / 2.0;
	let diameter = level.width().max(level.height()) as f32;
	commands.spawn(Camera3dBundle {
		transform: Transform::from_xyz(
			center_x,
			diameter,
			level.height() as f32,
		)
		.looking_at(Vec3::new(center_x, 0.0, center_z), Vec3::Y),
		..default()
	});
	// Add lighting.
	commands.spawn(PointLightBundle {
		point_light: PointLight {
			intensity: 2500.0,
			shadows_enabled: true,
			..default()
		},
		transform: Transform::from_xyz(0.0, 10.0, 0.0),
		..default()
	});

	// Insert level resource.
	commands.insert_resource(level);
}
