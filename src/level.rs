use std::{
	ops::{Add, AddAssign, Mul, Neg},
	sync::Arc,
};

use bevy::{
	prelude::*,
	utils::{HashMap, HashSet},
};

use crate::action::{Action, PendingActions};

/// Row-column coordinates on a [`Level`] grid.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Coords {
	pub row: i32,
	pub col: i32,
}

impl Coords {
	pub fn new(row: i32, col: i32) -> Coords {
		Coords { row, col }
	}
}

impl From<Coords> for Transform {
	fn from(value: Coords) -> Self {
		Transform::from_xyz(value.col as f32, 0.5, value.row as f32)
	}
}

/// Row-column offset from [`Coords`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

impl Ord for Offset {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.row
			.cmp(&other.row)
			.then_with(|| self.col.cmp(&other.col))
	}
}

impl PartialOrd for Offset {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Neg for Offset {
	type Output = Self;

	fn neg(self) -> Self {
		Self {
			row: -self.row,
			col: -self.col,
		}
	}
}

impl Mul<i32> for Offset {
	type Output = Self;

	fn mul(self, rhs: i32) -> Self {
		Self {
			row: self.row * rhs,
			col: self.col * rhs,
		}
	}
}

impl Mul<Offset> for i32 {
	type Output = Offset;

	fn mul(self, rhs: Offset) -> Offset {
		Offset {
			row: self * rhs.row,
			col: self * rhs.col,
		}
	}
}

impl AddAssign<Offset> for Coords {
	fn add_assign(&mut self, rhs: Offset) {
		self.row = self.row + rhs.row;
		self.col = self.col + rhs.col;
	}
}

impl Add<Offset> for Coords {
	type Output = Self;

	fn add(mut self, rhs: Offset) -> Self {
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
pub struct Id(pub u32);

/// Something that can be moved around a level.
#[derive(Clone, Copy)]
pub enum Object {
	Character { idx: usize },
	WoodenCrate,
	SteelCrate,
	StoneBlock,
}

impl Object {
	fn weight(&self) -> i32 {
		match self {
			Object::Character { .. } => 1,
			Object::WoodenCrate => 1,
			Object::SteelCrate => 2,
			Object::StoneBlock => 3,
		}
	}
}

/// An [`Object`] along with data relating that object to a [`Level`].
pub struct LevelObject {
	pub id: Id,
	pub object: Object,
	pub coords: Coords,
}

/// The complete state of a level at a single point in time.
#[derive(Resource)]
pub struct Level {
	width: usize,
	height: usize,
	tiles: Vec<Tile>,
	objects_by_id: HashMap<Id, LevelObject>,
	object_ids_by_coords: HashMap<Coords, Id>,
	character_ids: Vec<Id>,
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
		self.tiles[coords.row as usize * self.width + coords.col as usize]
	}

	/// Iterates over all objects in the level.
	pub fn iter_objects(&self) -> impl Iterator<Item = &LevelObject> {
		self.objects_by_id.values()
	}

	/// IDs of characters in this level, in order of appearance.
	pub fn character_ids(&self) -> &[Id] {
		&self.character_ids
	}

	/// Updates the level by executing `pending_actions`, returning the
	/// resulting (possibly trivial) [`Change`].
	pub fn update(&mut self, pending_actions: &PendingActions) -> Arc<Change> {
		// Map pushers to their desired offsets.
		let pushers: HashMap<Id, Offset> = pending_actions
			.iter()
			.filter_map(|(id, action)| {
				if let Action::Push(offset) = action {
					Some((*id, *offset))
				} else {
					None
				}
			})
			.collect();

		// Build the set of teams, keyed by starting coordinates. Teams may not
		// be maximal; i.e. some teams may be subsumed by larger ones.
		let mut teams: HashMap<Coords, Team> = pushers
			.iter()
			.filter_map(|(id, &offset)| {
				let pusher = &self.objects_by_id[id];
				// The team starts with just the backmost pusher.
				let mut team = Team {
					start: pusher.coords,
					offset,
					count: 1,
					strength: 1,
				};
				// Consider tiles in the direction of the backmost pusher.
				let mut coords = pusher.coords + offset;
				loop {
					// Nullify teams facing a wall.
					if let Tile::Wall = self.tile(coords) {
						return None;
					}
					// Check for the next object in line.
					let other_id = self.object_ids_by_coords.get(&coords);
					let Some(other_id) = other_id else { break };
					// If the object is a pusher, it may contribute to, oppose,
					// or be orthogonal to the current team.
					if let Some(&other_offset) = pushers.get(other_id) {
						if other_offset == offset {
							// Contributing; add strength.
							team.strength += 2;
						} else if other_offset == -offset {
							// Opposing; nullify this team.
							return None;
						} else {
							// Part of an orthogonal team - may be able to get
							// out of the way later.
							break;
						}
					}
					// The team's strength must remain at or above zero for its
					// entire length.
					let other = &self.objects_by_id[other_id].object;
					team.strength -= other.weight();
					if team.strength < 0 {
						return None;
					}
					// Welcome to the team.
					team.count += 1;
					coords += offset;
				}
				Some((pusher.coords, team))
			})
			.collect();

		// Sort the teams by priority.
		let mut sorted_teams: Vec<Team> = teams.values().copied().collect();
		sorted_teams.sort();
		let sorted_teams = sorted_teams;

		// Visit teams in order of decreasing priority, cutting any overlapping
		// non-subteams. Don't discard subteams yet because they could become
		// maximal if superteams are discarded.
		let mut cut_teams = HashSet::new();
		for team in sorted_teams.iter().rev() {
			if cut_teams.contains(&team.start) {
				continue;
			}
			cut_teams.extend(teams.values().filter_map(|other| {
				team.collides(other).then_some(other.start)
			}));
		}
		for team_start in cut_teams {
			teams.remove(&team_start);
		}

		// Now that actual collisions are resolved, discard subteams. Each
		// subteam starts within the "tail" of another team's coordinates set.
		let subteams: HashSet<Coords> = teams
			.values()
			.flat_map(|team| team.coords().skip(1))
			.collect();
		teams.retain(|team_start, _| !subteams.contains(team_start));

		// For each team, precompute the collisions with other teams given that
		// either/both teams move this turn.
		let mut stay_move_collisions = HashMap::new();
		let mut move_stay_collisions = HashMap::new();
		let mut move_move_collisions = HashMap::new();
		for team in teams.values() {
			let team_moved = team.moved();
			for other in teams.values() {
				let other_moved = other.moved();
				if team.collides(&other_moved) {
					stay_move_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(other.start);
				}
				if team_moved.collides(other) {
					move_stay_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(other.start);
				}
				if team_moved.collides(&other_moved) {
					move_move_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(other.start);
				}
			}
		}

		// Visit each team in order of increasing priority, resolving collisions
		// by marking teams as blocked (unable to move). This tends to give the
		// right-of-way to stronger teams.
		let mut blocked_teams = HashSet::new();
		for team in sorted_teams {
			if blocked_teams.contains(&team.start) {
				// This team was already blocked; nothing more to do.
				continue;
			}
			// Blocking a team can cause other teams to become blocked, which we
			// track with an iterative work queue.
			let mut block_queue = Vec::new();
			// Block this team if moving it may cause a collision with an
			// unblocked team. These other teams could become blocked later, so
			// this algorithm may not always block the fewest possible teams.
			if let Some(others) = move_move_collisions.get(&team.start) {
				if others.iter().any(|other| !blocked_teams.contains(other)) {
					block_queue = vec![team.start];
				}
			}
			// Block this team if moving it causes a collision with a blocked
			// team.
			if let Some(others) = move_stay_collisions.get(&team.start) {
				if others.iter().any(|other| blocked_teams.contains(other)) {
					block_queue = vec![team.start];
				}
			}
			// Iteratively block teams as needed.
			while let Some(team_start) = block_queue.pop() {
				if !blocked_teams.insert(team_start) {
					// This team was already blocked; nothing more to do.
					continue;
				}
				// Blocking this team may block other teams, and so on.
				if let Some(others) = stay_move_collisions.get(&team_start) {
					block_queue.extend(others);
				}
			}
		}

		// Move the objects in unblocked teams.
		let mut moves = HashMap::new();
		for team in teams
			.values()
			.filter(|team| !blocked_teams.contains(&team.start))
		{
			for coords in team.coords() {
				let id = self.object_ids_by_coords[&coords];
				let mv = self.get_move(id, team.offset);
				moves.insert(id, mv);
			}
		}

		// Create and apply the change.
		let change = Change { moves };
		self.apply(&change);

		// Add the change to the turn history.
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
	fn get_move(&self, id: Id, offset: Offset) -> Move {
		let from = self.objects_by_id[&id].coords;
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
	pub moves: HashMap<Id, Move>,
}

/// A connected line of pushers and passive objects, for use in the resolution
/// of simultaneous movement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Team {
	start: Coords,
	/// The unit offset in the direction of the team.
	offset: Offset,
	count: usize,
	strength: i32,
}

impl Team {
	/// A copy of this team after applying `offset` to `start`.
	fn moved(&self) -> Team {
		Team {
			start: self.start + self.offset,
			..*self
		}
	}

	/// An iterator over the coordinates occupied by objects in this team.
	fn coords(&self) -> TeamCoordsIterator {
		TeamCoordsIterator {
			team: *self,
			idx: 0,
		}
	}

	/// Whether `self` and `other` collide. Subteams are not considered to
	/// collide with superteams.
	fn collides(&self, other: &Team) -> bool {
		if self.offset == other.offset {
			// Teams can only be in collision if one is a subteam of the other.
			return false;
		}
		// Could check this in constant time, but this is simpler/good enough.
		self.coords().any(|c1| other.coords().any(|c2| c1 == c2))
	}
}

struct TeamCoordsIterator {
	team: Team,
	idx: usize,
}

impl Iterator for TeamCoordsIterator {
	type Item = Coords;

	fn next(&mut self) -> Option<Self::Item> {
		if self.idx < self.team.count {
			let result = self.team.start + self.idx as i32 * self.team.offset;
			self.idx += 1;
			Some(result)
		} else {
			None
		}
	}
}

impl Ord for Team {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		// Prioritize teams by strength, breaking ties by offset for the sake of
		// determinism.
		self.strength
			.cmp(&other.strength)
			.then_with(|| self.offset.cmp(&other.offset))
	}
}

impl PartialOrd for Team {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
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
	let (width, height) = (9, 9);
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

	let mut id = Id(0);
	let mut add_object = |object: Object, coords: Coords| {
		level.add_object(LevelObject { id, object, coords });
		id.0 += 1;
	};

	// Add characters.
	for idx in 0..3 {
		add_object(Object::Character { idx }, Coords::new(1 + idx as i32, 2));
	}

	// Add three wooden crates.
	for row in 4..7 {
		add_object(Object::WoodenCrate, Coords::new(row, 2));
	}
	// Add two steel crates.
	for row in 4..6 {
		add_object(Object::SteelCrate, Coords::new(row, 3));
	}
	// Add one stone block.
	add_object(Object::StoneBlock, Coords::new(4, 4));

	level
}
