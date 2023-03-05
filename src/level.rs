use std::{
	ops::{Add, AddAssign},
	sync::Arc,
};

use bevy::{prelude::*, utils::HashMap};

use crate::action::{Action, PendingActions};

/// Row-column coordinates on a [`Level`] grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coords {
	pub row: i32,
	pub col: i32,
}

impl Coords {
	pub fn new(row: i32, col: i32) -> Coords {
		Coords { row, col }
	}
}

/// Row-column offset from [`Coords`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Offset {
	pub row: i32,
	pub col: i32,
}

impl Offset {
	pub const UP: Offset = Offset::new(-1, 0);
	pub const DOWN: Offset = Offset::new(1, 0);
	pub const LEFT: Offset = Offset::new(0, -1);
	pub const RIGHT: Offset = Offset::new(0, 1);

	pub const fn new(row: i32, col: i32) -> Offset {
		Offset { row, col }
	}
}

impl AddAssign<Offset> for Coords {
	fn add_assign(&mut self, rhs: Offset) {
		self.row = self.row + rhs.row;
		self.col = self.col + rhs.col;
	}
}

impl Add<Offset> for Coords {
	type Output = Coords;

	fn add(mut self, rhs: Offset) -> Self::Output {
		self += rhs;
		self
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
	Crate { weight: i32 },
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
		self.tiles[coords.row as usize * self.height + coords.col as usize]
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
		let pushers = pending_actions
			.iter()
			.filter_map(|(id, action)| {
				if let Action::Push(offset) = action {
					Some((*id, *offset))
				} else {
					None
				}
			})
			.collect::<HashMap<_, _>>();
		let teams = pushers
			.iter()
			.filter_map(|(id, &offset)| {
				let pusher = &self.objects_by_id[id];
				let mut coords = pusher.coords + offset;
				let mut strength = 1;
				let mut count = 1;
				loop {
					if let Tile::Wall = self.tile(coords) {
						return None;
					}
					let Some(other_id) = self.object_ids_by_coords.get(&coords) else {
						break;
					};
					let level_object = &self.objects_by_id[other_id];
					match level_object.object {
						Object::Character { .. } => {
							if let Some(&other_offset) = pushers.get(other_id) {
								if other_offset != offset {
									return None;
								} else {
									strength += 1;
								}
							} else {
								strength -= 1;
							}
						}
						Object::Crate { weight } => {
							strength -= weight;
						}
					}
					if strength < 0 {
						return None;
					}
					count += 1;
					coords += offset;
				}
				let team = Team {
					start: pusher.coords,
					offset,
					count,
					strength,
				};
				Some((pusher.coords, team))
			})
			.collect::<HashMap<_, _>>();

		// Create and apply the change.
		let mut change = Change {
			moves: HashMap::new(),
		};
		for team in teams.values() {
			let id = self.object_ids_by_coords[&team.start];
			let mv = self.get_move(id, team.offset);
			change.moves.insert(id, mv);
		}
		self.apply(&change);

		// Add the change to the turn history and return it.
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
		// To make sure every target tile is open, first remove all movers.
		for mv in change.moves.values() {
			self.object_ids_by_coords.remove(&mv.from);
		}
		// Now place the movers into their new tiles.
		for (id, mv) in change.moves.iter() {
			let level_object = self.objects_by_id.get_mut(id).unwrap();
			self.object_ids_by_coords.insert(mv.to, level_object.id);
			level_object.coords = mv.to;
		}
	}

	/// Gets a [`Move`] of the object `id` by `offset`.
	fn get_move(&mut self, id: ID, offset: Offset) -> Move {
		let level_object = self.objects_by_id.get_mut(&id).unwrap();
		let from = level_object.coords;
		let to = from + offset;
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

/// A connected line of pushers and passive objects, for use in the resolution
/// of simultaneous movement.
struct Team {
	start: Coords,
	/// The unit offset in the direction of the team.
	offset: Offset,
	count: usize,
	strength: i32,
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
	let (width, height) = (7, 7);
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
		object: Object::Crate { weight: 1 },
		coords: Coords::new(3, 3),
	});
	level.add_object(LevelObject {
		id: ID(2),
		object: Object::Crate { weight: 1 },
		coords: Coords::new(3, 4),
	});
	level.add_object(LevelObject {
		id: ID(3),
		object: Object::Crate { weight: 1 },
		coords: Coords::new(3, 5),
	});
	level.add_object(LevelObject {
		id: ID(4),
		object: Object::Character { index: 1 },
		coords: Coords::new(1, 3),
	});
	level
}
