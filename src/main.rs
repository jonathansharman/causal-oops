use std::f32::consts::TAU;

use bevy::{
	input::{keyboard::KeyboardInput, ButtonState},
	prelude::*,
	render::camera::ScalingMode,
};
use bevy_easings::EasingsPlugin;

use control::ControlEvent;
use level::{ChangeEvent, Coords, Level, LevelEntity, Object, Tile};
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
		.add_plugins((
			DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Causal Oops".to_string(),
					..default()
				}),
				..default()
			}),
			EasingsPlugin::default(),
		))
		.init_state::<GameState>()
		.add_systems(Startup, setup)
		.add_systems(
			Update,
			(
				load_gltf_meshes.run_if(in_state(GameState::Loading)),
				spawn_level.run_if(in_state(GameState::SpawningLevel)),
				(
					control::control,
					update::update,
					(
						animation::animate_returnings,
						animation::animate_moves,
						animation::animate_summonings,
						animation::timed_despawn,
					),
					// Allow adding indicators on newly spawned entities.
					apply_deferred,
					animation::add_indicators,
					// Allow indicators to be added/removed in one frame.
					apply_deferred,
					animation::clear_indicators,
					change_level,
				)
					.chain()
					.run_if(in_state(GameState::Playing)),
			),
		)
		.add_event::<NextActor>()
		.add_event::<ControlEvent>()
		.add_event::<ChangeEvent>()
		.insert_resource(ClearColor(Color::BLACK))
		.insert_resource(level::test_level())
		.run();
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

fn spawn_level(
	mut commands: Commands,
	level: Res<Level>,
	models: Res<Models>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
	mut ambient_light: ResMut<AmbientLight>,
	mut next_actors: EventWriter<NextActor>,
	mut next_state: ResMut<NextState<GameState>>,
) {
	// Spawn tile entities.
	for row in 0..level.height() {
		for col in 0..level.width() {
			let tile_coords = Coords::new(row as i32, col as i32);
			match level.tile_at(tile_coords) {
				// Assume a fresh level has no open portals.
				Tile::Floor { .. } => commands.spawn((
					LevelEntity,
					SceneRoot(models.floor.clone()),
					tile_coords.transform(-0.5),
				)),
				Tile::Wall => commands.spawn((
					LevelEntity,
					SceneRoot(models.wall.clone()),
					tile_coords.transform(0.5),
				)),
			};
		}
	}

	// Spawn object entities.
	for level_object in level.iter_level_objects() {
		let transform = level_object.coords.transform(0.5);
		match level_object.object {
			Object::Character(c) => commands
				.spawn((
					LevelEntity,
					animation::Object {
						id: level_object.id,
						rotates: true,
					},
					transform,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						Mesh3d(meshes.character.clone()),
						MeshMaterial3d(
							materials.characters[c.color.idx()].clone(),
						),
					));
				}),
			Object::WoodenCrate => commands
				.spawn((
					LevelEntity,
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					transform,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneRoot(models.wooden_crate.clone()),
					));
				}),
			Object::SteelCrate => commands
				.spawn((
					LevelEntity,
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					transform,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneRoot(models.steel_crate.clone()),
					));
				}),
			Object::StoneBlock => commands
				.spawn((
					LevelEntity,
					animation::Object {
						id: level_object.id,
						rotates: false,
					},
					transform,
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						animation::ObjectBody,
						SceneRoot(models.stone_block.clone()),
					));
				}),
		};
	}

	// Add static camera overlooking the level.
	let offset = Vec3::new(-0.5, 0.5, 1.0);
	let level_size =
		Vec3::new(level.width() as f32, level.height() as f32, 0.0);
	let target = offset + 0.5 * Vec3::new(level_size.x, -level_size.y, 0.0);
	commands.spawn((
		LevelEntity,
		Camera3d::default(),
		Transform::from_translation(Vec3::new(
			target.x,
			-level_size.y,
			level_size.x.max(level_size.y),
		))
		.looking_at(target, Vec3::Z),
		Projection::Orthographic(OrthographicProjection {
			scaling_mode: ScalingMode::AutoMin {
				min_width: level_size.x,
				min_height: level_size.y,
			},
			..OrthographicProjection::default_3d()
		}),
	));

	// Add lighting.
	ambient_light.brightness = 250.0;
	commands.spawn((
		LevelEntity,
		DirectionalLight {
			illuminance: 0.3 * light_consts::lux::AMBIENT_DAYLIGHT,
			shadows_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_axis_angle(
			Vec3::new(1.0, 1.0, 0.0),
			-TAU / 16.0,
		)),
	));

	// Kick off the control loop by sending the first actor, if there is one.
	if let Some((&id, &character)) = level.characters_by_id().next() {
		next_actors.send(NextActor { id, character });
	}

	next_state.set(GameState::Playing);
}

fn change_level(
	mut commands: Commands,
	mut keyboard_events: EventReader<KeyboardInput>,
	mut level: ResMut<Level>,
	mut next_state: ResMut<NextState<GameState>>,
	level_entities: Query<Entity, With<level::LevelEntity>>,
) {
	for event in keyboard_events.read() {
		if event.state != ButtonState::Pressed {
			continue;
		}
		if let Some(next_level) = match event.key_code {
			KeyCode::Digit1 => Some(level::test_level()),
			KeyCode::Digit2 => Some(level::test_level_short()),
			KeyCode::Digit3 => Some(level::test_level_thin()),
			KeyCode::Digit4 => Some(level::test_level_large()),
			_ => None,
		} {
			// Despawn any existing level entities.
			for entity in level_entities.into_iter() {
				commands.entity(entity).despawn_recursive();
			}
			// Update the level resource and respawn the level.
			*level = next_level;
			next_state.set(GameState::SpawningLevel);
		}
	}
}
