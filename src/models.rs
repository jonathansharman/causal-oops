use bevy::prelude::*;

#[derive(Resource)]
pub struct Models {
	pub wall: Handle<Scene>,
	pub floor: Handle<Scene>,
	pub wooden_crate: Handle<Scene>,
	pub steel_crate: Handle<Scene>,
	pub stone_block: Handle<Scene>,
}

impl Models {
	pub fn load(scene_assets: &mut AssetServer) -> Self {
		Self {
			wall: scene_assets.load("models/wall.glb#Scene0"),
			floor: scene_assets.load("models/stone.glb#Scene0"),
			wooden_crate: scene_assets.load("models/wooden-crate.glb#Scene0"),
			steel_crate: scene_assets.load("models/steel-crate.glb#Scene0"),
			stone_block: scene_assets.load("models/sandstone-block.glb#Scene0"),
		}
	}
}
