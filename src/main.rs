use std::time::Duration;

use bevy::prelude::*;
use bevy_tweening::{
	lens::TransformPositionLens, Animator, EaseFunction, Tween, TweeningPlugin,
	TweeningType,
};
use iyes_loopless::prelude::*;

use action::{Action, PendingActions};
use level::{Change, Coords, Direction, Level, Object, Tile};
use material::Materials;
use mesh::Meshes;
use state::State;

mod action;
mod animation;
mod level;
mod material;
mod mesh;
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
		.add_plugin(TweeningPlugin)
		.run();
}

fn spawn_level(
	commands: &mut Commands,
	meshes: &Meshes,
	materials: &Materials,
	level: &Level,
) {
	// Spawn tile entities.
	for row in 0..level.height() {
		for col in 0..level.width() {
			match level.tile(Coords::new(row, col)) {
				Tile::Floor => commands.spawn_bundle(PbrBundle {
					mesh: meshes.block.clone(),
					material: materials.floor.clone(),
					transform: Transform::from_xyz(
						col as f32, -0.5, row as f32,
					),
					..default()
				}),
				Tile::Wall => commands.spawn_bundle(PbrBundle {
					mesh: meshes.block.clone(),
					material: materials.wall.clone(),
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
					mesh: meshes.character.clone(),
					material: materials.character.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				})
				.insert(animation::Object {
					id: level_object.id,
				}),
			Object::Crate => commands
				.spawn_bundle(PbrBundle {
					mesh: meshes.block.clone(),
					material: materials.wood.clone(),
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
	mut mesh_assets: ResMut<Assets<Mesh>>,
	mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
	// Load meshes and materials.
	let meshes = Meshes::load(&mut mesh_assets);
	let materials = Materials::load(&mut material_assets);

	// Create level.
	let level = level::test_level();
	spawn_level(&mut commands, &meshes, &materials, &level);
	commands.insert_resource(level);

	// Insert mesh and material resources.
	commands.insert_resource(meshes);
	commands.insert_resource(materials);

	commands.insert_resource(PendingActions::new());

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
	commands: Commands,
	input: Res<Input<KeyCode>>,
	mut level: ResMut<Level>,
	mut pending_actions: ResMut<PendingActions>,
	animation_query: Query<(Entity, &animation::Object)>,
) {
	if input.just_pressed(KeyCode::Z) {
		if let Some(change) = level.undo() {
			start_animation(commands, &change, animation_query);
			return;
		}
	}
	if input.just_pressed(KeyCode::X) {
		if let Some(change) = level.redo() {
			start_animation(commands, &change, animation_query);
			return;
		}
	}

	let action = if input.just_pressed(KeyCode::Space) {
		Some(Action::Wait)
	} else if input.just_pressed(KeyCode::Left) {
		Some(Action::Push(Direction::Left))
	} else if input.just_pressed(KeyCode::Right) {
		Some(Action::Push(Direction::Right))
	} else if input.just_pressed(KeyCode::Up) {
		Some(Action::Push(Direction::Up))
	} else if input.just_pressed(KeyCode::Down) {
		Some(Action::Push(Direction::Down))
	} else {
		None
	};

	if let Some(action) = action {
		if let Some(id) = level.character_ids().get(pending_actions.len()) {
			pending_actions.insert(*id, action);
			if pending_actions.len() == level.character_ids().len() {
				// All characters have been assigned moves. Execute turn.
				let change = level.update(&pending_actions);
				pending_actions.clear();
				start_animation(commands, &change, animation_query);
			}
		}
	}
}

const ANIMATION_DURATION: Duration = Duration::from_millis(200);

fn start_animation(
	mut commands: Commands,
	change: &Change,
	mut animation_query: Query<(Entity, &animation::Object)>,
) {
	// Apply movements.
	for (entity, object) in &mut animation_query {
		let Some(mv) = change.moves.get(&object.id) else { continue };
		commands.entity(entity).insert(Animator::new(Tween::new(
			EaseFunction::CubicInOut,
			TweeningType::Once,
			ANIMATION_DURATION,
			TransformPositionLens {
				start: Vec3::new(mv.from.col as f32, 0.5, mv.from.row as f32),
				end: Vec3::new(mv.to.col as f32, 0.5, mv.to.row as f32),
			},
		)));
	}
	commands.insert_resource(Timer::new(ANIMATION_DURATION, false));
	commands.insert_resource(NextState(State::Animate));
}

fn animate(mut commands: Commands, time: Res<Time>, mut timer: ResMut<Timer>) {
	timer.tick(time.delta());
	if timer.finished() {
		commands.insert_resource(NextState(State::Control));
	}
}
