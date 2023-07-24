use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy_easings::EasingsPlugin;

use control::ControlEvent;
use level::{ChangeEvent, Coords, Level, Object, Tile};
use materials::Materials;
use meshes::Meshes;
use models::{load_gltf_meshes, Models};
use states::GameState;
use update::NextActor;

mod animation;
mod control;
mod level;
mod materials;
mod meshes;
mod models;
mod states;
mod update;

fn main() {
	App::new()
		.add_state::<GameState>()
		.add_systems(Startup, setup)
		.add_systems(
			Update,
			(
				load_gltf_meshes.run_if(in_state(GameState::Loading)),
				create_level.run_if(in_state(GameState::CreatingLevel)),
				(
					control::control,
					update::update,
					animation::add_indicators,
					// Allow indicators to be added/removed in one frame.
					apply_deferred,
					animation::clear_indicators,
					animation::animate,
				)
					.chain()
					.run_if(in_state(GameState::Playing)),
			),
		)
		.add_event::<NextActor>()
		.add_event::<ControlEvent>()
		.add_event::<ChangeEvent>()
		.insert_resource(ClearColor(Color::BLACK))
		.add_plugins((
			DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Causal Oops".to_string(),
					..default()
				}),
				..default()
			}),
			EasingsPlugin,
		))
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
				Tile::Floor { portal_summoner } => {
					let scene = match portal_summoner {
						Some(summoner_id) => {
							let color = level
								.characters()
								.iter()
								.find_map(|(id, c)| {
									(*id == summoner_id).then_some(c.color)
								})
								.unwrap();
							// TODO: Portal models.
							models.floor.clone()
						}
						None => models.floor.clone(),
					};
					commands.spawn(SceneBundle {
						scene,
						transform: Transform::from_xyz(
							col as f32, -0.5, row as f32,
						),
						..default()
					})
				}
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
		let spatial_bundle = SpatialBundle {
			transform: Transform::from_xyz(col as f32, 0.5, row as f32),
			..default()
		};
		match level_object.object {
			Object::Character(c) => commands
				.spawn((
					animation::Object {
						id: level_object.id,
						rotates: true,
					},
					spatial_bundle,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						PbrBundle {
							mesh: meshes.character.clone(),
							material: materials.characters[c.color.idx()]
								.clone(),
							transform: Transform::from_rotation(
								Quat::from_rotation_y(-FRAC_PI_2),
							),
							..default()
						},
					));
				}),
			Object::WoodenCrate => commands
				.spawn((
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					spatial_bundle,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneBundle {
							scene: models.wooden_crate.clone(),
							..default()
						},
					));
				}),
			Object::SteelCrate => commands
				.spawn((
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					spatial_bundle,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneBundle {
							scene: models.steel_crate.clone(),
							..default()
						},
					));
				}),
			Object::StoneBlock => commands
				.spawn((
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					spatial_bundle,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneBundle {
							scene: models.stone_block.clone(),
							..default()
						},
					));
				}),
		};
	}
}

// Loads and inserts models, meshes, and materials.
fn setup(
	mut commands: Commands,
	mut asset_server: ResMut<AssetServer>,
	mut mesh_assets: ResMut<Assets<Mesh>>,
	mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
	commands.insert_resource(Models::load(&mut asset_server));
	commands.insert_resource(Meshes::load(&mut mesh_assets));
	commands.insert_resource(Materials::load(&mut material_assets));
}

fn create_level(
	mut commands: Commands,
	models: Res<Models>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
	mut next_actors: EventWriter<NextActor>,
	mut next_state: ResMut<NextState<GameState>>,
) {
	let level = level::test_level();
	spawn_level(&mut commands, &models, &meshes, &materials, &level);
	// Kick off the control loop by sending the first actor, if there is one.
	if let Some(&(id, character)) = level.characters().first() {
		next_actors.send(NextActor { id, character });
	}

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

	next_state.set(GameState::Playing);
}
