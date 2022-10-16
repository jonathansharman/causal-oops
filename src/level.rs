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
#[derive(Clone, Copy)]
pub struct ID(u32);

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
	objects: HashMap<Coords, LevelObject>,
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

	pub fn objects(&self) -> &HashMap<Coords, LevelObject> {
		&self.objects
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
	let objects = HashMap::from([
		(
			Coords::new(1, 1),
			LevelObject {
				id: ID(0),
				object: Object::Character,
				coords: Coords::new(1, 1),
			},
		),
		(
			Coords::new(3, 3),
			LevelObject {
				id: ID(1),
				object: Object::Crate,
				coords: Coords::new(3, 3),
			},
		),
	]);
	Level {
		width,
		height,
		tiles,
		objects,
	}
}
