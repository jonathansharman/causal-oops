use bevy::{
	gltf::{Gltf, GltfMesh},
	platform::collections::HashMap,
	prelude::*,
};

use crate::states::GameState;

type GetMeshMut = fn(&mut Models) -> &mut Handle<Mesh>;

#[derive(Resource)]
pub struct Models {
	pub wall: Handle<WorldAsset>,
	pub floor: Handle<WorldAsset>,
	pub wooden_crate: Handle<WorldAsset>,
	pub steel_crate: Handle<WorldAsset>,
	pub stone_block: Handle<WorldAsset>,
	pub stairs: Handle<WorldAsset>,

	pub question_mesh: Handle<Mesh>,
	pub wait_mesh: Handle<Mesh>,
	pub arrow_mesh: Handle<Mesh>,
	pub summon_mesh: Handle<Mesh>,
	pub return_mesh: Handle<Mesh>,
	pub descend_mesh: Handle<Mesh>,

	// Used to track which Gltf assets haven't finished loading yet and to
	// determine which mesh their contents should be loaded into.
	unloaded: HashMap<Handle<Gltf>, GetMeshMut>,
}

impl Models {
	pub fn load(asset_server: &mut AssetServer) -> Self {
		let load = |path, getter: GetMeshMut| (asset_server.load(path), getter);
		let unloaded = HashMap::from([
			load("models/question.glb", |models| &mut models.question_mesh),
			load("models/wait.glb", |models| &mut models.wait_mesh),
			load("models/arrow.glb", |models| &mut models.arrow_mesh),
			load("models/summon.glb", |models| &mut models.summon_mesh),
			load("models/return.glb", |models| &mut models.return_mesh),
			load("models/descend.glb", |models| &mut models.descend_mesh),
		]);

		let load = |path| {
			let scene = GltfAssetLabel::Scene(0);
			asset_server.load(scene.from_asset(path))
		};
		Self {
			wall: load("models/wall.glb"),
			floor: load("models/stone.glb"),
			wooden_crate: load("models/wooden-crate.glb"),
			steel_crate: load("models/steel-crate.glb"),
			stone_block: load("models/sandstone-block.glb"),
			stairs: load("models/stairs.glb"),
			// Initialize meshes with default handles, which the
			// load_gltf_meshes system will replace once Gltf assets load.
			question_mesh: Handle::default(),
			wait_mesh: Handle::default(),
			arrow_mesh: Handle::default(),
			summon_mesh: Handle::default(),
			return_mesh: Handle::default(),
			descend_mesh: Handle::default(),
			unloaded,
		}
	}
}

pub fn load_gltf_meshes(
	mut asset_events: MessageReader<AssetEvent<Gltf>>,
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
