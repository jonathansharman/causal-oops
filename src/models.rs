use bevy::{
	gltf::{Gltf, GltfMesh},
	prelude::*,
	utils::HashMap,
};

use crate::states::GameState;

type GetMeshMut = fn(&mut Models) -> &mut Handle<Mesh>;

#[derive(Resource)]
pub struct Models {
	pub wall: Handle<Scene>,
	pub floor: Handle<Scene>,
	pub wooden_crate: Handle<Scene>,
	pub steel_crate: Handle<Scene>,
	pub stone_block: Handle<Scene>,

	pub question_mesh: Handle<Mesh>,
	pub arrow_mesh: Handle<Mesh>,

	// Used to track which Gltf assets haven't finished loading yet and to
	// determine which mesh their contents should be loaded into.
	unloaded: HashMap<Handle<Gltf>, GetMeshMut>,
}

impl Models {
	pub fn load(asset_server: &mut AssetServer) -> Self {
		let mut unloaded: HashMap<Handle<Gltf>, GetMeshMut> = HashMap::new();
		unloaded.insert(asset_server.load("models/question.glb"), |models| {
			&mut models.question_mesh
		});
		unloaded.insert(asset_server.load("models/arrow.glb"), |models| {
			&mut models.arrow_mesh
		});
		Self {
			wall: asset_server.load("models/wall.glb#Scene0"),
			floor: asset_server.load("models/stone.glb#Scene0"),
			wooden_crate: asset_server.load("models/wooden-crate.glb#Scene0"),
			steel_crate: asset_server.load("models/steel-crate.glb#Scene0"),
			stone_block: asset_server.load("models/sandstone-block.glb#Scene0"),
			// Initialize meshes with default handles, which the
			// load_gltf_meshes system will replace once Gltf assets load.
			question_mesh: Handle::default(),
			arrow_mesh: Handle::default(),
			unloaded,
		}
	}
}

pub fn load_gltf_meshes(
	mut asset_events: EventReader<AssetEvent<Gltf>>,
	mut models: ResMut<Models>,
	gltf_assets: Res<Assets<Gltf>>,
	gltf_mesh_assets: Res<Assets<GltfMesh>>,
	mut next_state: ResMut<NextState<GameState>>,
) {
	for asset_event in asset_events.iter() {
		if let AssetEvent::Created { handle } = asset_event {
			if let Some(get_mesh_mut) = models.unloaded.remove(handle) {
				let gltf = gltf_assets.get(handle).unwrap();
				let gltf_mesh = gltf_mesh_assets.get(&gltf.meshes[0]).unwrap();
				let mesh = gltf_mesh.primitives[0].mesh.clone();
				*get_mesh_mut(&mut models) = mesh;
			}
		}
	}
	if models.unloaded.is_empty() {
		next_state.set(GameState::CreatingLevel);
	}
}
