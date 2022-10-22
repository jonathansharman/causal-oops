use bevy::utils::HashMap;

/// Row-column coordinates on a [`Level`] grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coords {
	pub row: usize,
	pub col: usize,
}

impl Coords {
	pub fn new(row: usize, col: usize) -> Coords {
		Coords { row, col }
	}
}

/// A level tile.
#[derive(Clone, Copy)]
pub enum Tile {
	Floor,
	Wall,
}

/// A character or portal identifier. Enables correlating characters with
/// portals and character animations across frames.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ID(pub u32);

/// Something that can be moved around a level.
#[derive(Clone, Copy)]
pub enum Object {
	Character,
	Crate,
}

/// An [`Object`] along with data relating that object to a [`Level`].
pub struct LevelObject {
	pub id: ID,
	pub object: Object,
	pub coords: Coords,
}

/// The complete state of a level at a single point in time.
pub struct Level {
	width: usize,
	height: usize,
	tiles: Vec<Tile>,
	objects_by_id: HashMap<ID, LevelObject>,
	object_ids_by_coords: HashMap<Coords, ID>,
}

impl Level {
	pub fn width(&self) -> usize {
		self.width
	}

	pub fn height(&self) -> usize {
		self.height
	}

	pub fn tile(&self, coords: Coords) -> Tile {
		self.tiles[coords.row * self.height + coords.col]
	}

	/// Iterates over all objects in the level.
	pub fn iter_objects(&self) -> impl Iterator<Item = &LevelObject> {
		self.objects_by_id.values()
	}

	/// Gets a reference to the object with the given ID, if it exists.
	pub fn get_object(&self, id: &ID) -> Option<&LevelObject> {
		self.objects_by_id.get(id)
	}

	/// Gets a mutable reference to the object with the given ID, if it exists.
	pub fn get_object_mut(&mut self, id: &ID) -> Option<&mut LevelObject> {
		self.objects_by_id.get_mut(id)
	}

	/// Adds `level_object` to the level.
	fn add_object(&mut self, level_object: LevelObject) {
		self.object_ids_by_coords
			.insert(level_object.coords, level_object.id);
		self.objects_by_id.insert(level_object.id, level_object);
	}

	/// Moves the object with the given ID from `from` to `to`.
	pub fn move_object(&mut self, id: &ID, from: Coords, to: Coords) {
		if let Some(level_object) = self.objects_by_id.get_mut(id) {
			self.object_ids_by_coords.remove(&from);
			self.object_ids_by_coords.insert(to, *id);
			level_object.coords = to;
		}
	}
}

pub fn test_level() -> Level {
	let (width, height) = (5, 5);
	let mut tiles = Vec::with_capacity(width * height);
	for row in 0..height {
		for col in 0..width {
			let tile = if row == 0
				|| row == height - 1
				|| col == 0 || col == width - 1
			{
				Tile::Wall
			} else {
				Tile::Floor
			};
			tiles.push(tile)
		}
	}
	let mut level = Level {
		width,
		height,
		tiles,
		object_ids_by_coords: HashMap::new(),
		objects_by_id: HashMap::new(),
	};
	level.add_object(LevelObject {
		id: ID(0),
		object: Object::Character,
		coords: Coords::new(1, 1),
	});
	level.add_object(LevelObject {
		id: ID(1),
		object: Object::Crate,
		coords: Coords::new(3, 3),
	});
	level
}
