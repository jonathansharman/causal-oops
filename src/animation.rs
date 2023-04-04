use std::{ops::Mul, sync::Arc, time::Duration};

use bevy::{pbr::NotShadowCaster, prelude::*};
use bevy_easings::{Ease, EaseFunction, EasingType};

use crate::{
	control::{Action, ControlEvent},
	level::{Change, Id},
	materials::Materials,
	models::Models,
	update::NextActor,
};

/// Animates an object in a level.
#[derive(Component)]
pub struct Object {
	pub id: Id,
}

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
	query: Query<(Entity, &Object, &Transform)>,
	choosing_query: Query<Entity, With<ChoosingIndicator>>,
) {
	for NextActor { id, .. } in next_actors.iter() {
		for entity in &choosing_query {
			commands.entity(entity).despawn();
		}
		let entity = query
			.iter()
			.find_map(|(entity, object, _)| {
				(object.id == *id).then_some(entity)
			})
			.unwrap();
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
		commands.entity(entity).add_child(indicator);
	}

	for control_event in control_events.iter() {
		let ControlEvent::Act((id, action)) = control_event else { continue };

		let transform = query
			.iter()
			.find_map(|(_, object, transform)| {
				(object.id == *id).then_some(transform)
			})
			.unwrap()
			.mul_transform(Transform::from_translation(0.5 * Vec3::Y));

		let (mesh, transform) = match action {
			Action::Wait => (models.wait_mesh.clone(), transform),
			Action::Push(offset) => {
				(models.arrow_mesh.clone(), transform.mul(offset.transform()))
			}
			Action::Summon(offset) => {
				(models.arrow_mesh.clone(), transform.mul(offset.transform()))
			}
			Action::Return => (models.arrow_mesh.clone(), transform),
		};
		commands.spawn((
			PbrBundle {
				mesh,
				material: materials.indicator.clone(),
				transform,
				..default()
			},
			NotShadowCaster,
			ChoiceIndicator,
		));
	}
}

/// Remove indicators between turns.
pub fn clear_indicators(
	mut commands: Commands,
	change_events: EventReader<Arc<Change>>,
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
	mut change_events: EventReader<Arc<Change>>,
	animation_query: Query<(Entity, &Transform, &Object)>,
) {
	for change in change_events.iter() {
		// Apply movements.
		for (entity, from, object) in &animation_query {
			let Some(mv) = change.moves.get(&object.id) else { continue };
			commands.entity(entity).insert(from.ease_to(
				Transform::from(mv.to),
				EaseFunction::CubicInOut,
				EasingType::Once {
					duration: ANIMATION_DURATION,
				},
			));
		}
	}
}
