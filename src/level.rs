use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use crate::action::{Action, PendingActions};

#[derive(Clone, Copy)]
pub enum Direction {
	Up,
	Down,
	Left,
	Right,
}

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

	/// The adjacent coordinates in the given direction.
	pub fn neighbor(&self, direction: Direction) -> Coords {
		match direction {
			Direction::Up => Coords {
				row: self.row - 1,
				col: self.col,
			},
			Direction::Down => Coords {
				row: self.row + 1,
				col: self.col,
			},
			Direction::Left => Coords {
				row: self.row,
				col: self.col - 1,
			},
			Direction::Right => Coords {
				row: self.row,
				col: self.col + 1,
			},
		}
	}
}

/// A level tile.
#[derive(Clone, Copy)]
pub enum Tile {
	Floor,
	Wall,
}

/// An object identifier. Enables correlating object animations across frames.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ID(pub u32);

/// Something that can be moved around a level.
#[derive(Clone, Copy)]
pub enum Object {
	Character { index: usize },
	Crate,
}

/// An [`Object`] along with data relating that object to a [`Level`].
pub struct LevelObject {
	pub id: ID,
	pub object: Object,
	pub coords: Coords,
}

/// The complete state of a level at a single point in time.
#[derive(Resource)]
pub struct Level {
	width: usize,
	height: usize,
	tiles: Vec<Tile>,
	objects_by_id: HashMap<ID, LevelObject>,
	object_ids_by_coords: HashMap<Coords, ID>,
	character_ids: Vec<ID>,
	/// History of the level's state, for seeking backward and forward in time.
	history: Vec<BiChange>,
	turn: usize,
}

impl Level {
	/// The number of columns in the level.
	pub fn width(&self) -> usize {
		self.width
	}

	/// The number of rows in the level.
	pub fn height(&self) -> usize {
		self.height
	}

	/// The tile at `coords`.
	pub fn tile(&self, coords: Coords) -> Tile {
		self.tiles[coords.row * self.height + coords.col]
	}

	/// Iterates over all objects in the level.
	pub fn iter_objects(&self) -> impl Iterator<Item = &LevelObject> {
		self.objects_by_id.values()
	}

	/// IDs of characters in this level, in order of appearance.
	pub fn character_ids(&self) -> &[ID] {
		&self.character_ids
	}

	/// Updates the level by executing the given `pending_actions`, returning
	/// the resulting (possibly trivial) [`Change`].
	pub fn update(&mut self, pending_actions: &PendingActions) -> Arc<Change> {
		let mut change = Change {
			moves: HashMap::new(),
		};
		for (id, action) in pending_actions.iter() {
			match action {
				Action::Wait => (),
				Action::Push(direction) => {
					change.moves.insert(*id, self.push_object(id, *direction));
				}
			}
		}
		let reverse = Arc::new(change.reversed());
		let change = Arc::new(change);
		// Truncate history to remove any future states. This is a no-op if the
		// level is already at the end of its history.
		self.history.truncate(self.turn);
		self.history.push(BiChange {
			forward: change.clone(),
			reverse,
		});
		self.turn += 1;
		change
	}

	/// If possible, moves to the previous level state and returns the applied
	/// [`Change`].
	pub fn undo(&mut self) -> Option<Arc<Change>> {
		if self.turn > 0 {
			let change = self.history[self.turn - 1].reverse.clone();
			self.apply(&change);
			self.turn -= 1;
			Some(change)
		} else {
			None
		}
	}

	/// If possible, moves to the next level state and returns the applied
	/// [`Change`].
	pub fn redo(&mut self) -> Option<Arc<Change>> {
		if self.turn < self.history.len() {
			let change = self.history[self.turn].forward.clone();
			self.apply(&change);
			self.turn += 1;
			Some(change)
		} else {
			None
		}
	}

	/// Applies `change` to the level's state without affecting history.
	fn apply(&mut self, change: &Change) {
		for (id, mv) in change.moves.iter() {
			let level_object = self.objects_by_id.get_mut(id).unwrap();
			Self::move_object(
				level_object,
				&mut self.object_ids_by_coords,
				mv.from,
				mv.to,
			);
		}
	}

	/// Pushes the object with the given ID towards `direction`.
	fn push_object(&mut self, id: &ID, direction: Direction) -> Move {
		let level_object = self.objects_by_id.get_mut(id).unwrap();
		let from = level_object.coords;
		let to = level_object.coords.neighbor(direction);
		Self::move_object(
			level_object,
			&mut self.object_ids_by_coords,
			from,
			to,
		);
		Move { from, to }
	}

	/// Adds `level_object` to the level.
	fn add_object(&mut self, level_object: LevelObject) {
		self.object_ids_by_coords
			.insert(level_object.coords, level_object.id);
		if let Object::Character { .. } = level_object.object {
			self.character_ids.push(level_object.id);
		}
		self.objects_by_id.insert(level_object.id, level_object);
	}

	/// Moves `level_object` from `from` to `to`, updating
	/// `object_ids_by_coords` appropriately.
	fn move_object(
		level_object: &mut LevelObject,
		object_ids_by_coords: &mut HashMap<Coords, ID>,
		from: Coords,
		to: Coords,
	) {
		object_ids_by_coords.remove(&from);
		object_ids_by_coords.insert(to, level_object.id);
		level_object.coords = to;
	}
}

/// A movement of an object from one tile to another.
#[derive(Clone, Copy)]
pub struct Move {
	pub from: Coords,
	pub to: Coords,
}

impl Move {
	fn reversed(self) -> Move {
		Move {
			from: self.to,
			to: self.from,
		}
	}
}

/// A change from one [`Level`] state to another.
#[derive(Clone)]
pub struct Change {
	pub moves: HashMap<ID, Move>,
}

/// A bidirectional change, i.e. a pair inverse changes.
struct BiChange {
	forward: Arc<Change>,
	reverse: Arc<Change>,
}

impl Change {
	fn reversed(&self) -> Change {
		Change {
			moves: self
				.moves
				.iter()
				.map(|(id, mv)| (*id, mv.reversed()))
				.collect(),
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
		history: Vec::new(),
		turn: 0,
		character_ids: Vec::new(),
	};
	level.add_object(LevelObject {
		id: ID(0),
		object: Object::Character { index: 0 },
		coords: Coords::new(1, 1),
	});
	level.add_object(LevelObject {
		id: ID(1),
		object: Object::Crate,
		coords: Coords::new(3, 3),
	});
	level.add_object(LevelObject {
		id: ID(2),
		object: Object::Character { index: 1 },
		coords: Coords::new(1, 3),
	});
	level
}
