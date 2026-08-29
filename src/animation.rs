use std::time::Duration;

use bevy::{
	light::{NotShadowCaster, NotShadowReceiver},
	prelude::*,
};
use bevy_easings::{Ease, EaseFunction, EasingType};

use crate::{
	control::{Action, Control},
	level::{
		ChangeMessage, Coords, Descent, Id, LevelEntity, Returning, Summoning,
	},
	materials::Materials,
	meshes::Meshes,
	models::Models,
	update::NextActor,
};

/// Component for animating an object in a level.
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct Object {
	pub id: Id,
	pub rotates: bool,
}

/// Component for animating a portal in a level.
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct Portal {
	pub coords: Coords,
}

/// Marks the "body" of an object's animation. Making an `ObjectBody` entity a
/// child of an [`Object`] entity allows setting the body's rotation
/// independently from the rotation of UI elements (such as turn indicators)
/// associated with that `Object`.
#[derive(Component)]
#[require(Transform, Visibility)]
pub struct ObjectBody;

#[derive(Component)]
pub struct ChoosingIndicator;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct ChoiceIndicator;

/// Add indicators for pending actions and next actor.
pub fn add_indicators(
	mut commands: Commands,
	models: Res<Models>,
	materials: Res<Materials>,
	mut next_actors: MessageReader<NextActor>,
	mut controls: MessageReader<Control>,
	object_query: Query<(Entity, &Object, &Transform)>,
	choosing_query: Query<Entity, With<ChoosingIndicator>>,
) {
	let transform = Transform::from_translation(0.5 * Vec3::Z);

	// Next actor
	for NextActor { actor, only } in next_actors.read() {
		// Clear any existing choosing indicators.
		for entity in &choosing_query {
			commands.entity(entity).despawn();
		}
		// If this is the only actor, there's no need to show an indicator.
		if *only {
			continue;
		}
		// Spawn a new choosing indicator.
		let indicator = commands
			.spawn((
				Mesh3d(models.question_mesh.clone()),
				MeshMaterial3d(materials.indicator.clone()),
				transform,
				NotShadowCaster,
				NotShadowReceiver,
				ChoosingIndicator,
			))
			.id();
		// Make the indicator a child of the next actor.
		let actor = object_query
			.iter()
			.find_map(|(entity, object, _)| {
				(object.id == actor.id).then_some(entity)
			})
			.expect("next actor entity not found");
		commands.entity(actor).add_child(indicator);
	}

	// Pending actions
	for control in controls.read() {
		let Control::Act((actor_id, action)) = control else {
			continue;
		};
		// Get the mesh and transform for the pending action indicator.
		let (mesh, transform) = match action {
			Action::Wait => (models.wait_mesh.clone(), transform),
			Action::Push(offset) => (
				models.arrow_mesh.clone(),
				transform.with_rotation(Quat::from_rotation_z(offset.angle())),
			),
			Action::Summon(_offset) => (models.summon_mesh.clone(), transform),
			Action::Return => (models.return_mesh.clone(), transform),
			Action::Descend => (models.descend_mesh.clone(), transform),
		};
		// Spawn the indicator.
		let indicator = commands
			.spawn((
				Mesh3d(mesh),
				MeshMaterial3d(materials.indicator.clone()),
				transform,
				NotShadowCaster,
				NotShadowReceiver,
				ChoiceIndicator,
			))
			.id();
		// Make the indicator a child of the pending actor.
		let actor = object_query
			.iter()
			.find_map(|(entity, object, _)| {
				(object.id == *actor_id).then_some(entity)
			})
			.expect("pending actor entity not found");
		commands.entity(actor).add_child(indicator);
	}
}

/// Remove indicators between turns.
pub fn clear_indicators(
	mut commands: Commands,
	changes: MessageReader<ChangeMessage>,
	choice_query: Query<Entity, With<ChoiceIndicator>>,
) {
	if !changes.is_empty() {
		for entity in &choice_query {
			commands.entity(entity).despawn();
		}
	}
}

const ANIMATION_DURATION: Duration = Duration::from_millis(200);

pub fn animate_descent(
	mut commands: Commands,
	mut changes: MessageReader<ChangeMessage>,
	object_query: Query<(Entity, &Object)>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
) {
	for change in changes.read() {
		if let Some(Descent { descender, reverse }) = change.descent {
			let above = descender.coords.transform(0.5);
			let below = above.with_translation(above.translation - Vec3::Z);
			if reverse {
				// Respawn un-descending character.
				commands
					.spawn((
						LevelEntity,
						Object {
							id: descender.id,
							rotates: true,
						},
						below.with_scale(Vec3::ZERO),
						below.with_scale(Vec3::ZERO).ease_to(
							above,
							EaseFunction::CubicIn,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						),
					))
					.with_children(|child_builder| {
						child_builder.spawn((
							ObjectBody,
							Mesh3d(meshes.character.clone()),
							MeshMaterial3d(
								materials.characters
									[descender.character.color.idx()]
								.clone(),
							),
							Transform::from_rotation(Quat::from_rotation_z(
								descender.angle,
							)),
						));
					});
			} else {
				// Despawn descending character.
				for (entity, object) in &object_query {
					if object.id == descender.id {
						commands.entity(entity).insert((
							DespawnTimer::from_duration(ANIMATION_DURATION),
							above.ease_to(
								below,
								EaseFunction::CubicIn,
								EasingType::Once {
									duration: ANIMATION_DURATION,
								},
							),
						));
						break;
					}
				}
			}
		}
	}
}

pub fn animate_returnings(
	mut commands: Commands,
	mut changes: MessageReader<ChangeMessage>,
	object_query: Query<(Entity, &Object)>,
	portal_query: Query<(Entity, &Portal)>,
) {
	for change in changes.read() {
		for Returning { returner, .. } in change.returnings.values() {
			let returner_transform = returner.coords.transform(0.5);
			let portal_transform = returner
				.coords
				.transform(0.5 * crate::meshes::PORTAL_HEIGHT);
			// Despawn returning character.
			for (entity, object) in &object_query {
				if object.id == returner.id {
					commands.entity(entity).insert((
						DespawnTimer::from_duration(ANIMATION_DURATION),
						returner_transform.ease_to(
							returner_transform.with_scale(Vec3::ZERO),
							EaseFunction::CubicIn,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						),
					));
					break;
				}
			}
			// Despawn closed portal.
			for (entity, portal) in &portal_query {
				if portal.coords == returner.coords {
					commands.entity(entity).insert((
						DespawnTimer::from_duration(ANIMATION_DURATION),
						portal_transform.ease_to(
							portal_transform.with_scale(Vec3::ZERO),
							EaseFunction::CubicIn,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						),
					));
					break;
				}
			}
		}
	}
}

pub fn animate_moves(
	mut commands: Commands,
	mut changes: MessageReader<ChangeMessage>,
	object_query: Query<(Entity, &Children, &Transform, &Object)>,
	body_query: Query<(Entity, &Transform), With<ObjectBody>>,
) {
	for change in changes.read() {
		for (parent, children, from, object) in &object_query {
			let Some(mv) = change.moves.get(&object.id) else {
				continue;
			};
			commands.entity(parent).insert(from.ease_to(
				mv.to_coords.transform(0.5),
				EaseFunction::CubicInOut,
				EasingType::Once {
					duration: ANIMATION_DURATION,
				},
			));
			// Rotating the parent entity directly would cause indicators to
			// rotate as well. Instead, rotate just the child "body" entity.
			if object.rotates {
				for child in children {
					if let Ok((body, from)) = body_query.get(*child) {
						commands.entity(body).insert(from.ease_to(
							Transform::from_rotation(Quat::from_rotation_z(
								mv.to_angle,
							)),
							EaseFunction::CubicInOut,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						));
					}
				}
			}
		}
	}
}

pub fn animate_summonings(
	mut commands: Commands,
	mut changes: MessageReader<ChangeMessage>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
) {
	for change in changes.read() {
		for Summoning {
			summon,
			portal_color,
			..
		} in change.summonings.values()
		{
			let summon_transform = summon.coords.transform(0.5);
			let portal_transform =
				summon.coords.transform(0.5 * crate::meshes::PORTAL_HEIGHT);
			// Spawn summoned character.
			commands
				.spawn((
					LevelEntity,
					Object {
						id: summon.id,
						rotates: true,
					},
					summon_transform.with_scale(Vec3::ZERO),
					summon_transform.with_scale(Vec3::ZERO).ease_to(
						summon_transform,
						EaseFunction::CubicIn,
						EasingType::Once {
							duration: ANIMATION_DURATION,
						},
					),
				))
				.with_children(|child_builder| {
					child_builder.spawn((
						ObjectBody,
						Mesh3d(meshes.character.clone()),
						MeshMaterial3d(
							materials.characters[summon.character.color.idx()]
								.clone(),
						),
						Transform::from_rotation(Quat::from_rotation_z(
							summon.angle,
						)),
					));
				});
			// Spawn opened portal.
			commands.spawn((
				LevelEntity,
				Portal {
					coords: summon.coords,
				},
				NotShadowCaster,
				NotShadowReceiver,
				Mesh3d(meshes.portal.clone()),
				MeshMaterial3d(
					materials.characters[portal_color.idx()].clone(),
				),
				portal_transform.with_scale(Vec3::ZERO),
				portal_transform.with_scale(Vec3::ZERO).ease_to(
					portal_transform,
					EaseFunction::CubicIn,
					EasingType::Once {
						duration: ANIMATION_DURATION,
					},
				),
			));
		}
	}
}

/// Marks an entity to be recursively despawned after a fixed time.
#[derive(Component, Deref, DerefMut)]
pub struct DespawnTimer(Timer);

impl DespawnTimer {
	fn from_duration(duration: Duration) -> DespawnTimer {
		DespawnTimer(Timer::from_seconds(
			duration.as_secs_f32(),
			TimerMode::Once,
		))
	}
}

/// Recursively despawns entities whose [`DespawnTimer`]s have finished.
pub fn timed_despawn(
	mut commands: Commands,
	mut query: Query<(Entity, &mut DespawnTimer)>,
	time: Res<Time>,
) {
	for (entity, mut timer) in &mut query {
		timer.tick(time.delta());
		if timer.is_finished() {
			commands.entity(entity).despawn();
		}
	}
}
