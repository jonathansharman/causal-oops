use std::{f32::consts::FRAC_PI_2, time::Duration};

use bevy::{
	pbr::{NotShadowCaster, NotShadowReceiver},
	prelude::*,
};
use bevy_easings::{Ease, EaseFunction, EasingType};

use crate::{
	control::{Action, ControlEvent},
	level::{ChangeEvent, Coords, Id},
	materials::Materials,
	meshes::Meshes,
	models::Models,
	update::NextActor,
};

/// Component for animating an object in a level.
#[derive(Component)]
pub struct Object {
	pub id: Id,
	pub rotates: bool,
}

/// Component for animating a portal in a level.
#[derive(Component)]
pub struct Portal {
	pub coords: Coords,
}

/// Marks the "body" of an object's animation. Making an `ObjectBody` entity a
/// child of an [`Object`] entity allows setting the body's rotation
/// independently from the rotation of UI elements (such as turn indicators)
/// associated with that `Object`.
#[derive(Component)]
pub struct ObjectBody;

#[derive(Component)]
pub struct ChoosingIndicator;

#[derive(Component)]
pub struct ChoiceIndicator;

/// Add indicators for pending actions and next actor.
pub fn add_indicators(
	mut commands: Commands,
	models: Res<Models>,
	materials: Res<Materials>,
	mut next_actors: EventReader<NextActor>,
	mut control_events: EventReader<ControlEvent>,
	object_query: Query<(Entity, &Object, &Transform)>,
	choosing_query: Query<Entity, With<ChoosingIndicator>>,
) {
	// Next actor
	for NextActor { id: actor_id, .. } in next_actors.iter() {
		// Clear any existing choosing indicators.
		for entity in &choosing_query {
			commands.entity(entity).despawn();
		}
		// Spawn a new choosing indicator.
		let transform = Transform::from_translation(0.5 * Vec3::Y);
		let indicator = commands
			.spawn((
				PbrBundle {
					mesh: models.question_mesh.clone(),
					material: materials.indicator.clone(),
					transform,
					..default()
				},
				NotShadowCaster,
				NotShadowReceiver,
				ChoosingIndicator,
			))
			.id();
		// Make the indicator a child of the next actor.
		let actor = object_query
			.iter()
			.find_map(|(entity, object, _)| {
				(object.id == *actor_id).then_some(entity)
			})
			.unwrap();
		commands.entity(actor).add_child(indicator);
	}

	// Pending actions
	for control_event in control_events.iter() {
		let ControlEvent::Act((actor_id, action)) = control_event else {
			continue;
		};
		// Get the mesh and transform for the pending action indicator.
		let transform = Transform::from_translation(0.5 * Vec3::Y);
		let (mesh, transform) = match action {
			Action::Wait => (models.wait_mesh.clone(), transform),
			Action::Push(offset) => (
				models.arrow_mesh.clone(),
				transform.with_rotation(Quat::from_rotation_y(offset.angle())),
			),
			Action::Summon(_offset) => (models.summon_mesh.clone(), transform),
			Action::Return => (models.return_mesh.clone(), transform),
		};
		// Spawn the indicator.
		let indicator = commands
			.spawn((
				PbrBundle {
					mesh,
					material: materials.indicator.clone(),
					transform,
					..default()
				},
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
			.unwrap();
		commands.entity(actor).add_child(indicator);
	}
}

/// Remove indicators between turns.
pub fn clear_indicators(
	mut commands: Commands,
	change_events: EventReader<ChangeEvent>,
	choice_query: Query<Entity, With<ChoiceIndicator>>,
) {
	if !change_events.is_empty() {
		for entity in &choice_query {
			commands.entity(entity).despawn();
		}
	}
}

const ANIMATION_DURATION: Duration = Duration::from_millis(200);

pub fn animate_returns(
	mut commands: Commands,
	mut change_events: EventReader<ChangeEvent>,
	object_query: Query<(Entity, &Object)>,
	portal_query: Query<(Entity, &Portal)>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
) {
	for change in change_events.iter() {
		for ret in change.returns.values() {
			let returner_transform = Transform::from_xyz(
				ret.coords.col as f32,
				0.5,
				ret.coords.row as f32,
			);
			let portal_transform = Transform::from_xyz(
				ret.coords.col as f32,
				0.5 * crate::meshes::PORTAL_HEIGHT,
				ret.coords.row as f32,
			);
			if ret.reversed {
				// Spawn returning character.
				commands
					.spawn((
						Object {
							id: ret.id,
							rotates: true,
						},
						SpatialBundle { ..default() },
						returner_transform.with_scale(Vec3::ZERO).ease_to(
							returner_transform.with_scale(Vec3::ONE),
							EaseFunction::CubicIn,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						),
					))
					.with_children(|child_builder| {
						child_builder.spawn((
							ObjectBody,
							PbrBundle {
								mesh: meshes.character.clone(),
								material: materials.characters
									[ret.character.color.idx()]
								.clone(),
								transform: Transform::from_rotation(
									Quat::from_rotation_y(-FRAC_PI_2),
								),
								..default()
							},
						));
					});
				// Spawn closed portal.
				commands.spawn((
					Portal { coords: ret.coords },
					NotShadowCaster,
					NotShadowReceiver,
					PbrBundle {
						mesh: meshes.portal.clone(),
						material: materials.characters
							[ret.character.color.idx()]
						.clone(),
						..default()
					},
					portal_transform.with_scale(Vec3::ZERO).ease_to(
						portal_transform.with_scale(Vec3::ONE),
						EaseFunction::CubicIn,
						EasingType::Once {
							duration: ANIMATION_DURATION,
						},
					),
				));
			} else {
				// Despawn returning character.
				for (entity, object) in &object_query {
					if object.id == ret.id {
						commands.entity(entity).insert((
							DespawnTimer::from_duration(ANIMATION_DURATION),
							returner_transform.with_scale(Vec3::ONE).ease_to(
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
					if portal.coords == ret.coords {
						commands.entity(entity).insert((
							DespawnTimer::from_duration(ANIMATION_DURATION),
							portal_transform.with_scale(Vec3::ONE).ease_to(
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
}

pub fn animate_moves(
	mut commands: Commands,
	mut change_events: EventReader<ChangeEvent>,
	object_query: Query<(Entity, &Children, &Transform, &Object)>,
	body_query: Query<(Entity, &Transform), With<ObjectBody>>,
) {
	for change in change_events.iter() {
		for (parent, children, from, object) in &object_query {
			let Some(mv) = change.moves.get(&object.id) else { continue };
			commands.entity(parent).insert(from.ease_to(
				Transform::from(mv.to_coords),
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
							Transform::from_rotation(Quat::from_rotation_y(
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

pub fn animate_summons(
	mut commands: Commands,
	mut change_events: EventReader<ChangeEvent>,
	object_query: Query<(Entity, &Object)>,
	portal_query: Query<(Entity, &Portal)>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
) {
	for change in change_events.iter() {
		for summon in change.summons.values() {
			let summon_transform = Transform::from_xyz(
				summon.coords.col as f32,
				0.5,
				summon.coords.row as f32,
			);
			let portal_transform = Transform::from_xyz(
				summon.coords.col as f32,
				0.5 * crate::meshes::PORTAL_HEIGHT,
				summon.coords.row as f32,
			);
			if summon.reversed {
				// Despawn summoned character.
				for (entity, object) in &object_query {
					if object.id == summon.id {
						commands.entity(entity).insert((
							DespawnTimer::from_duration(ANIMATION_DURATION),
							summon_transform.with_scale(Vec3::ONE).ease_to(
								summon_transform.with_scale(Vec3::ZERO),
								EaseFunction::CubicIn,
								EasingType::Once {
									duration: ANIMATION_DURATION,
								},
							),
						));
						break;
					}
				}
				// Despawn opened portal.
				for (entity, portal) in &portal_query {
					if portal.coords == summon.coords {
						commands.entity(entity).insert((
							DespawnTimer::from_duration(ANIMATION_DURATION),
							portal_transform.with_scale(Vec3::ONE).ease_to(
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
			} else {
				// Spawn summoned character.
				commands
					.spawn((
						Object {
							id: summon.id,
							rotates: true,
						},
						SpatialBundle { ..default() },
						summon_transform.with_scale(Vec3::ZERO).ease_to(
							summon_transform.with_scale(Vec3::ONE),
							EaseFunction::CubicIn,
							EasingType::Once {
								duration: ANIMATION_DURATION,
							},
						),
					))
					.with_children(|child_builder| {
						child_builder.spawn((
							ObjectBody,
							PbrBundle {
								mesh: meshes.character.clone(),
								material: materials.characters
									[summon.summon_color.idx()]
								.clone(),
								transform: Transform::from_rotation(
									Quat::from_rotation_y(-FRAC_PI_2),
								),
								..default()
							},
						));
					});
				// Spawn opened portal.
				commands.spawn((
					Portal {
						coords: summon.coords,
					},
					NotShadowCaster,
					NotShadowReceiver,
					PbrBundle {
						mesh: meshes.portal.clone(),
						material: materials.characters
							[summon.portal_color.idx()]
						.clone(),
						..default()
					},
					portal_transform.with_scale(Vec3::ZERO).ease_to(
						portal_transform.with_scale(Vec3::ONE),
						EaseFunction::CubicIn,
						EasingType::Once {
							duration: ANIMATION_DURATION,
						},
					),
				));
			}
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
		if timer.finished() {
			commands.entity(entity).despawn_recursive();
		}
	}
}
