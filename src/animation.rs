use std::{f32::consts::FRAC_PI_2, time::Duration};

use bevy::{pbr::NotShadowCaster, prelude::*};
use bevy_easings::{Ease, EaseFunction, EasingType};

use crate::{
	control::{Action, ControlEvent},
	level::{ChangeEvent, Id},
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

pub fn animate(
	mut commands: Commands,
	mut change_events: EventReader<ChangeEvent>,
	object_query: Query<(Entity, &Children, &Transform, &Object)>,
	body_query: Query<(Entity, &Transform), With<ObjectBody>>,
	meshes: Res<Meshes>,
	materials: Res<Materials>,
) {
	for change in change_events.iter() {
		// Apply movements.
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
		for summon in change.summons.values() {
			if summon.reversed {
				// Despawn entities for summoned character and opened portal.
				for (parent, _, _, object) in &object_query {
					if object.id == summon.id {
						commands.entity(parent).despawn_recursive();
						break;
					}
				}
			} else {
				// Spawn entities for summoned character and opened portal.
				let transform = Transform::from_xyz(
					summon.coords.col as f32,
					0.5,
					summon.coords.row as f32,
				);
				commands
					.spawn((
						Object {
							id: summon.id,
							rotates: true,
						},
						SpatialBundle { ..default() },
						transform.with_scale(Vec3::ZERO).ease_to(
							transform.with_scale(Vec3::ONE),
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
									[summon.color.idx()]
								.clone(),
								transform: Transform::from_rotation(
									Quat::from_rotation_y(-FRAC_PI_2),
								),
								..default()
							},
						));
					});
			}
		}
		// TODO: Despawn entities for returned characters and closed portals.
	}
}
