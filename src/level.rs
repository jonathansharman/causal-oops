use bevy::utils::HashMap;

/// Row-column coordinates on a [`Level`] grid.
#[derive(Clone, Copy)]
pub struct Coords {
	row: usize,
	col: usize,
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
pub struct ID(u32);

/// Something that can be moved around a level.
#[derive(Clone, Copy)]
pub enum Object {
	Character { id: ID },
}

/// A tile and the object on top of it, if any.
#[derive(Clone, Copy)]
pub struct Space {
	pub tile: Tile,
	pub object: Option<Object>,
}

/// The complete state of a level at a single point in time.
pub struct Level {
	width: usize,
	height: usize,
	spaces: Vec<Space>,
	/// Allows O(1) position lookup by character ID.
	character_coords: HashMap<ID, Coords>,
}

impl Level {
	pub fn width(&self) -> usize {
		self.width
	}

	pub fn height(&self) -> usize {
		self.height
	}

	pub fn at(&self, coords: Coords) -> Space {
		self.spaces[coords.row * self.height + coords.col]
	}
}

pub fn test_level() -> Level {
	let (width, height) = (5, 5);
	let mut spaces = Vec::with_capacity(width * height);
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
			let object = if row == 1 && col == 1 {
				Some(Object::Character { id: ID(0) })
			} else {
				None
			};
			spaces.push(Space { tile, object })
		}
	}
	Level {
		width,
		height,
		spaces,
		character_coords: HashMap::from([(ID(0), Coords::new(1, 1))]),
	}
}
