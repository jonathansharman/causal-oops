use bevy::{
	gltf::{Gltf, GltfMesh},
	platform::collections::HashMap,
	prelude::*,
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
	pub stairs: Handle<Scene>,

	pub question_mesh: Handle<Mesh>,
	pub wait_mesh: Handle<Mesh>,
	pub arrow_mesh: Handle<Mesh>,
	pub summon_mesh: Handle<Mesh>,
	pub return_mesh: Handle<Mesh>,

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
		unloaded.insert(asset_server.load("models/wait.glb"), |models| {
			&mut models.wait_mesh
		});
		unloaded.insert(asset_server.load("models/arrow.glb"), |models| {
			&mut models.arrow_mesh
		});
		unloaded.insert(asset_server.load("models/summon.glb"), |models| {
			&mut models.summon_mesh
		});
		unloaded.insert(asset_server.load("models/return.glb"), |models| {
			&mut models.return_mesh
		});
		let scene0 = GltfAssetLabel::Scene(0);
		Self {
			wall: asset_server.load(scene0.from_asset("models/wall.glb")),
			floor: asset_server.load(scene0.from_asset("models/stone.glb")),
			wooden_crate: asset_server
				.load(scene0.from_asset("models/wooden-crate.glb")),
			steel_crate: asset_server
				.load(scene0.from_asset("models/steel-crate.glb")),
			stone_block: asset_server
				.load(scene0.from_asset("models/sandstone-block.glb")),
			stairs: asset_server.load(scene0.from_asset("models/stairs.glb")),
			// Initialize meshes with default handles, which the
			// load_gltf_meshes system will replace once Gltf assets load.
			question_mesh: Handle::default(),
			wait_mesh: Handle::default(),
			arrow_mesh: Handle::default(),
			summon_mesh: Handle::default(),
			return_mesh: Handle::default(),
			unloaded,
		}
	}
}

pub fn load_gltf_meshes(
	mut asset_events: EventReader<AssetEvent<Gltf>>,
	mut models: ResMut<Models>,
	mut gltf_assets: ResMut<Assets<Gltf>>,
	gltf_mesh_assets: Res<Assets<GltfMesh>>,
	mut next_state: ResMut<NextState<GameState>>,
) {
	for asset_event in asset_events.read() {
		if let AssetEvent::Added { id } = asset_event {
			let Some(handle) = gltf_assets.get_strong_handle(*id) else {
				continue;
			};
			if let Some(get_mesh_mut) = models.unloaded.remove(&handle) {
				let gltf = gltf_assets.get(*id).unwrap();
				let gltf_mesh = gltf_mesh_assets.get(&gltf.meshes[0]).unwrap();
				let mesh = gltf_mesh.primitives[0].mesh.clone();
				*get_mesh_mut(&mut models) = mesh;
			}
		}
	}
	if models.unloaded.is_empty() {
		next_state.set(GameState::SpawningLevel);
	}
}
