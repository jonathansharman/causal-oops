use std::time::Duration;

use bevy::prelude::*;
use bevy_easings::{Ease, EaseFunction, EasingType, EasingsPlugin};

use action::{Action, PendingActions};
use level::{Change, Coords, Level, Object, Offset, Tile};
use material::Materials;
use mesh::Meshes;
use state::GameState;

mod action;
mod animation;
mod level;
mod material;
mod mesh;
mod state;

fn main() {
	App::new()
		.add_startup_system(setup)
		.add_state::<GameState>()
		.add_system(control.in_set(OnUpdate(GameState::Control)))
		.add_system(animate.in_set(OnUpdate(GameState::Animate)))
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
	meshes: &Meshes,
	materials: &Materials,
	level: &Level,
) {
	// Spawn tile entities.
	for row in 0..level.height() {
		for col in 0..level.width() {
			match level.tile(Coords::new(row as i32, col as i32)) {
				Tile::Floor => commands.spawn(PbrBundle {
					mesh: meshes.block.clone(),
					material: materials.floor.clone(),
					transform: Transform::from_xyz(
						col as f32, -0.5, row as f32,
					),
					..default()
				}),
				Tile::Wall => commands.spawn(PbrBundle {
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
			Object::Character { index } => commands.spawn((
				PbrBundle {
					mesh: meshes.character.clone(),
					material: materials.characters[index].clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				},
				animation::Object {
					id: level_object.id,
				},
			)),
			Object::Crate { .. } => commands.spawn((
				PbrBundle {
					mesh: meshes.block.clone(),
					material: materials.wood.clone(),
					transform: Transform::from_xyz(col as f32, 0.5, row as f32),
					..default()
				},
				animation::Object {
					id: level_object.id,
				},
			)),
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
	commands.spawn(PointLightBundle {
		point_light: PointLight {
			intensity: 1500.0,
			shadows_enabled: true,
			..default()
		},
		transform: Transform::from_xyz(4.0, 8.0, 4.0),
		..default()
	});

	// Add static camera overlooking the level.
	commands.spawn(Camera3dBundle {
		transform: Transform::from_xyz(3.0, 8.0, 8.0)
			.looking_at(Vec3::new(3.0, 0.0, 3.0), Vec3::Y),
		..default()
	});
}

fn control(
	next_state: ResMut<NextState<GameState>>,
	commands: Commands,
	input: Res<Input<KeyCode>>,
	mut level: ResMut<Level>,
	mut pending_actions: ResMut<PendingActions>,
	animation_query: Query<(Entity, &animation::Object)>,
) {
	if input.just_pressed(KeyCode::Z) {
		if let Some(change) = level.undo() {
			start_animation(next_state, commands, &change, animation_query);
			return;
		}
	}
	if input.just_pressed(KeyCode::X) {
		if let Some(change) = level.redo() {
			start_animation(next_state, commands, &change, animation_query);
			return;
		}
	}

	let action = if input.just_pressed(KeyCode::Space) {
		Some(Action::Wait)
	} else if input.just_pressed(KeyCode::Left) {
		Some(Action::Push(Offset::LEFT))
	} else if input.just_pressed(KeyCode::Right) {
		Some(Action::Push(Offset::RIGHT))
	} else if input.just_pressed(KeyCode::Up) {
		Some(Action::Push(Offset::UP))
	} else if input.just_pressed(KeyCode::Down) {
		Some(Action::Push(Offset::DOWN))
	} else {
		None
	};

	if let Some(action) = action {
		if let Some(id) = level.character_ids().get(pending_actions.len()) {
			pending_actions.push_back((*id, action));
			if pending_actions.len() == level.character_ids().len() {
				// All characters have been assigned moves. Execute turn.
				let change = level.update(&pending_actions);
				pending_actions.clear();
				start_animation(next_state, commands, &change, animation_query);
			}
		}
	}
}

const ANIMATION_DURATION: Duration = Duration::from_millis(200);

#[derive(Resource, Deref, DerefMut)]
struct AnimateTimer(Timer);

impl Default for AnimateTimer {
	fn default() -> Self {
		Self(Timer::new(ANIMATION_DURATION, TimerMode::Once))
	}
}

fn start_animation(
	mut next_state: ResMut<NextState<GameState>>,
	mut commands: Commands,
	change: &Change,
	mut animation_query: Query<(Entity, &animation::Object)>,
) {
	// Apply movements.
	for (entity, object) in &mut animation_query {
		let Some(mv) = change.moves.get(&object.id) else { continue };
		commands
			.entity(entity)
			.insert(Transform::from(mv.from).ease_to(
				Transform::from(mv.to),
				EaseFunction::CubicInOut,
				EasingType::Once {
					duration: ANIMATION_DURATION,
				},
			));
	}
	commands.insert_resource(AnimateTimer::default());
	next_state.set(GameState::Animate);
}

fn animate(
	mut next_state: ResMut<NextState<GameState>>,
	time: Res<Time>,
	mut timer: ResMut<AnimateTimer>,
) {
	timer.tick(time.delta());
	if timer.finished() {
		next_state.set(GameState::Control);
	}
}
