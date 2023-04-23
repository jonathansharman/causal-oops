use std::{
	cmp::Ordering,
	f32::consts::FRAC_PI_2,
	fmt::{Debug, Write},
	ops::{Add, AddAssign, Mul, Neg},
	sync::Arc,
};

use bevy::{
	prelude::*,
	utils::{HashMap, HashSet},
};

use crate::control::Action;

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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

	/// The angle formed by `self`.
	pub fn angle(&self) -> f32 {
		f32::atan2(-self.row as f32, self.col as f32)
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tile {
	Floor,
	Wall,
}

/// An object identifier. Enables correlating object animations across frames.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub u32);

/// A playable character.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Character {
	pub idx: usize,
	pub abilities: Abilities,
}

/// Something that can be moved around a level.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Object {
	Character(Character),
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
	pub angle: f32,
}

/// The set of abilities of a character. Determines what actions the character
/// can perform during the next turn.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Abilities {
	pub can_summon: bool,
	pub can_return: bool,
	pub can_push: bool,
}

impl Default for Abilities {
	fn default() -> Self {
		Self {
			can_summon: true,
			can_return: false,
			can_push: true,
		}
	}
}

/// The complete state of a level at a single point in time.
#[derive(Resource)]
pub struct Level {
	width: usize,
	height: usize,
	tiles: Vec<Tile>,
	objects_by_id: HashMap<Id, LevelObject>,
	object_ids_by_coords: HashMap<Coords, Id>,
	characters: Vec<(Id, Character)>,
	next_object_id: Id,
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

	/// The object at `coords`, if any.
	pub fn object(&self, coords: Coords) -> Option<Object> {
		self.object_ids_by_coords
			.get(&coords)
			.and_then(|id| self.objects_by_id.get(id))
			.map(|level_object| level_object.object)
	}

	/// Iterates over all objects in the level.
	pub fn iter_objects(&self) -> impl Iterator<Item = &LevelObject> {
		self.objects_by_id.values()
	}

	/// Characters in the level.
	pub fn characters(&self) -> &[(Id, Character)] {
		&self.characters
	}

	/// Updates the level by executing `character_actions`, returning the
	/// resulting (possibly trivial) [`Change`].
	pub fn update(
		&mut self,
		character_actions: impl Iterator<Item = (Id, Action)>,
	) -> Arc<Change> {
		// Map pushers to their desired offsets.
		let pushers: HashMap<Id, Offset> = character_actions
			.filter_map(|(id, action)| {
				if let Action::Push(offset) = action {
					Some((id, offset))
				} else {
					None
				}
			})
			.collect();

		// Build the set of teams, keyed by starting coordinates. Teams may not
		// be maximal; i.e. some teams may be subsumed by larger ones.
		let mut teams: HashMap<Coords, Team> = pushers
			.iter()
			.map(|(id, &offset)| {
				let pusher = &self.objects_by_id[id];
				// The team starts with just the backmost pusher.
				let mut team = Team {
					start: pusher.coords,
					offset,
					count: 1,
					strength: 1,
					blocked: false,
				};
				// Consider tiles in the direction of the backmost pusher.
				let mut coords = pusher.coords + offset;
				loop {
					// Block just the starting pusher of teams facing a wall, to
					// allow non-pushers to be claimed by other teams.
					if let Tile::Wall = self.tile(coords) {
						return (
							pusher.coords,
							Team {
								start: pusher.coords,
								offset,
								count: 1,
								strength: -1,
								blocked: true,
							},
						);
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
							// Opposing: block the starting pusher.
							return (
								pusher.coords,
								Team {
									start: pusher.coords,
									offset,
									count: 1,
									strength: -1,
									blocked: true,
								},
							);
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
						return (
							pusher.coords,
							Team {
								start: pusher.coords,
								offset,
								count: 1,
								strength: -1,
								blocked: true,
							},
						);
					}
					// Welcome to the team.
					team.count += 1;
					coords += offset;
				}
				(pusher.coords, team)
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
		let mut move_colliders = Vec::new();
		for team in teams.values() {
			let team_moved = team.moved();
			for other in teams.values() {
				let other_moved = other.moved();
				if team.collides(&other_moved) {
					stay_move_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(*other);
				}
				let move_stay = team_moved.collides(other);
				if move_stay {
					move_stay_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(*other);
				}
				if team_moved.collides(&other_moved) {
					move_move_collisions
						.entry(team.start)
						.or_insert(HashSet::new())
						.insert(*other);
					if move_stay {
						move_colliders.push(team.start);
					}
				}
			}
		}
		// Block teams that, regardless of what other teams do, collide on move.
		for team_start in move_colliders {
			teams.get_mut(&team_start).unwrap().blocked = true;
		}

		// Visit each team in order of increasing priority, resolving collisions
		// by marking teams as blocked (unable to move). This tends to give the
		// right-of-way to stronger teams.
		for team in sorted_teams {
			if team.blocked {
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
				if others.iter().any(|other| !teams[&other.start].blocked) {
					block_queue = vec![team];
				}
			}
			// Block this team if moving it causes a collision with a blocked
			// team.
			if let Some(others) = move_stay_collisions.get(&team.start) {
				if others.iter().any(|other| teams[&other.start].blocked) {
					block_queue = vec![team];
				}
			}
			// Iteratively block teams as needed.
			while let Some(team) = block_queue.pop() {
				if team.blocked {
					// This team was already blocked; nothing more to do.
					continue;
				}
				teams.get_mut(&team.start).unwrap().blocked = true;
				// Blocking this team may block other teams, and so on.
				if let Some(others) = stay_move_collisions.get(&team.start) {
					block_queue.extend(others);
				}
			}
		}

		// Move the objects in unblocked teams.
		let mut moves = HashMap::new();
		for team in teams.values().filter(|team| !team.blocked) {
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
			self.object_ids_by_coords.remove(&mv.from_coords);
		}
		// Now place the movers into their new tiles.
		for (id, mv) in change.moves.iter() {
			let level_object = self.objects_by_id.get_mut(id).unwrap();
			self.object_ids_by_coords
				.insert(mv.to_coords, level_object.id);
			level_object.coords = mv.to_coords;
			level_object.angle = mv.to_angle;
		}
	}

	/// Gets a [`Move`] of the object `id` by `offset`.
	fn get_move(&self, id: Id, offset: Offset) -> Move {
		let object = &self.objects_by_id[&id];
		let from_coords = object.coords;
		let to_coords = from_coords + offset;
		let from_angle = object.angle;
		let to_angle = offset.angle();
		Move {
			from_coords,
			to_coords,
			from_angle,
			to_angle,
		}
	}

	/// Adds `object` to the level at `coords`.
	fn add_object(&mut self, object: Object, coords: Coords, angle: f32) {
		let level_object = LevelObject {
			id: self.next_object_id,
			object,
			coords,
			angle,
		};
		self.next_object_id.0 += 1;

		self.object_ids_by_coords
			.insert(level_object.coords, level_object.id);
		if let Object::Character(c) = level_object.object {
			self.characters.push((level_object.id, c));
		}
		self.objects_by_id.insert(level_object.id, level_object);
	}
}

impl PartialEq<Level> for Level {
	/// Two levels are considered equal if they have the same tiles and objects.
	fn eq(&self, other: &Level) -> bool {
		self.width == other.width
			&& self.tiles == other.tiles
			&& (0..self.height).all(|row| {
				(0..self.width).all(|col| {
					let coords = Coords::new(row as i32, col as i32);
					self.object(coords) == other.object(coords)
				})
			})
	}
}

impl Debug for Level {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Level:")?;
		for row in 0..self.height {
			write!(f, "\n  ")?;
			for col in 0..self.width {
				let coords = Coords::new(row as i32, col as i32);
				let tile = self.tile(coords);
				let object = self.object(coords);
				f.write_char(match tile {
					Tile::Floor => '.',
					Tile::Wall => '#',
				})?;
				f.write_char(match object {
					Some(Object::Character(c)) => match c.idx {
						0 => '0',
						1 => '1',
						2 => '2',
						3 => '3',
						4 => '4',
						5 => '5',
						6 => '6',
						7 => '7',
						8 => '8',
						9 => '9',
						_ => '?',
					},
					Some(Object::WoodenCrate) => 'X',
					Some(Object::SteelCrate) => 'Y',
					Some(Object::StoneBlock) => 'Z',
					None => ' ',
				})?;
			}
		}
		Ok(())
	}
}

/// A movement of an object from one tile to another.
#[derive(Clone, Copy)]
pub struct Move {
	pub from_coords: Coords,
	pub to_coords: Coords,
	pub from_angle: f32,
	pub to_angle: f32,
}

impl Move {
	fn reversed(self) -> Move {
		Move {
			from_coords: self.to_coords,
			to_coords: self.from_coords,
			from_angle: self.to_angle,
			to_angle: self.from_angle,
		}
	}
}

/// A change from one [`Level`] state to another.
#[derive(Clone)]
pub struct Change {
	pub moves: HashMap<Id, Move>,
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

/// A connected line of pushers and passive objects, for use in the resolution
/// of simultaneous movement.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Team {
	start: Coords,
	/// The unit offset in the direction of the team.
	offset: Offset,
	count: usize,
	strength: i32,
	blocked: bool,
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

/// Makes a fresh copy of a simple test level.
pub fn test_level() -> Level {
	make_level(
		r#"# # # # # # # # # 
		   # . .0. . . . . # 
		   # . .1. . . . . # 
		   # . .2. . . . . # 
		   # . .X.Y.Z. . . # 
		   # . .X.Y. . . . # 
		   # . .X. . . . . # 
		   # . . . . . . . # 
		   # # # # # # # # # "#,
	)
}

/// Makes a test level from a string. Each line is a level row, alternating
/// between tiles and objects. Leading whitespace and blank lines are ignored.
fn make_level(map: &str) -> Level {
	let (mut width, mut height) = (0, 0);
	let mut tiles = Vec::new();
	let mut object_coords = Vec::new();
	for (row, line) in map
		.lines()
		.map(|line| line.trim_start())
		.filter(|line| !line.is_empty())
		.enumerate()
	{
		height = height.max(row + 1);
		for (col, tile_object) in line.as_bytes().chunks_exact(2).enumerate() {
			width = width.max(col + 1);
			let (tile, object) = (tile_object[0], tile_object[1]);
			tiles.push(match tile {
				b'#' => Tile::Wall,
				_ => Tile::Floor,
			});
			if let Some(object) = match object {
				b'0' => Some(Object::Character(Character {
					idx: 0,
					abilities: Abilities::default(),
				})),
				b'1' => Some(Object::Character(Character {
					idx: 1,
					abilities: Abilities::default(),
				})),
				b'2' => Some(Object::Character(Character {
					idx: 2,
					abilities: Abilities::default(),
				})),
				b'3' => Some(Object::Character(Character {
					idx: 3,
					abilities: Abilities::default(),
				})),
				b'4' => Some(Object::Character(Character {
					idx: 4,
					abilities: Abilities::default(),
				})),
				b'5' => Some(Object::Character(Character {
					idx: 5,
					abilities: Abilities::default(),
				})),
				b'6' => Some(Object::Character(Character {
					idx: 6,
					abilities: Abilities::default(),
				})),
				b'7' => Some(Object::Character(Character {
					idx: 7,
					abilities: Abilities::default(),
				})),
				b'8' => Some(Object::Character(Character {
					idx: 8,
					abilities: Abilities::default(),
				})),
				b'9' => Some(Object::Character(Character {
					idx: 9,
					abilities: Abilities::default(),
				})),
				b'X' => Some(Object::WoodenCrate),
				b'Y' => Some(Object::SteelCrate),
				b'Z' => Some(Object::StoneBlock),
				_ => None,
			} {
				object_coords
					.push((object, Coords::new(row as i32, col as i32)));
			}
		}
	}
	// Ensure characters are added in index order.
	object_coords.sort_unstable_by(|(o1, c1), (o2, c2)| {
		match (o1, o2) {
			(Object::Character(c1), Object::Character(c2)) => {
				c1.idx.cmp(&c2.idx)
			}
			// Put characters before non-characters.
			(Object::Character { .. }, _) => Ordering::Less,
			(_, Object::Character { .. }) => Ordering::Greater,
			// Otherwise, order doesn't matter.
			_ => c1.row.cmp(&c2.row),
		}
	});
	let mut level = Level {
		width,
		height,
		tiles,
		objects_by_id: HashMap::new(),
		object_ids_by_coords: HashMap::new(),
		characters: Vec::new(),
		next_object_id: Id(0),
		history: Vec::new(),
		turn: 0,
	};
	for (object, coords) in object_coords {
		level.add_object(object, coords, -FRAC_PI_2);
	}
	level
}

#[cfg(test)]
mod tests {
	use super::*;

	const U: Action = Action::Push(Offset::UP);
	const D: Action = Action::Push(Offset::DOWN);
	const L: Action = Action::Push(Offset::LEFT);
	const R: Action = Action::Push(Offset::RIGHT);
	const Z: Action = Action::Wait;

	/// Performs `actions` on `level`. The number of actions should match the
	/// number of characters in the level. Actions will be performed in
	/// character index order.
	fn perform<const N: usize>(level: &mut Level, actions: [Action; N]) {
		let character_actions: Vec<_> = level
			.characters
			.iter()
			.zip(actions)
			.map(|((id, _), action)| (*id, action))
			.collect();
		level.update(character_actions.into_iter());
	}

	/// Performs `actions` on `start` and asserts the result is equal to `end`.
	fn test<const N: usize>(actions: [Action; N], start: &str, end: &str) {
		let mut actual = make_level(start);
		perform(&mut actual, actions);
		let expected = make_level(end);
		assert_eq!(actual, expected);
	}

	// Push strength

	#[test]
	fn one_can_push_wooden_crate() {
		test([R], ".0.X. ", ". .0.X");
	}

	#[test]
	fn one_can_push_passive_character() {
		test([R, Z], ".0.1. ", ". .0.1");
	}

	#[test]
	fn one_cannot_push_two_wooden_crates() {
		test([R], ".0.X.X. ", ".0.X.X. ");
	}

	#[test]
	fn two_can_push_two_wooden_crates() {
		test([R, R], ".0.1.X.X. ", ". .0.1.X.X");
	}

	#[test]
	fn one_cannot_push_steel_crate() {
		test([R], ".0.Y. ", ".0.Y. ");
	}

	#[test]
	fn two_can_push_steel_crate() {
		test([R, R], ".0.1.Y. ", ". .0.1.Y");
	}

	// Blocking

	#[test]
	fn opposing_teams_block() {
		test([R, R, L], r#".0.1.2"#, r#".0.1.2"#);
	}

	#[test]
	fn orthogonal_team_blocks() {
		// Although the rightward team is stronger, it's blocked regardless of
		// whether the downward team moves.
		test(
			[D, D, R, R, R],
			r#". . . .0. 
			   .2.3.4.1. 
			   . . . . . "#,
			r#". . . . . 
			   .2.3.4.0. 
			   . . . .1. "#,
		);
	}

	#[test]
	fn blocked_orthogonal_pusher_blocks() {
		test(
			[R, D],
			r#".0.1
			   . # "#,
			r#".0.1
			   . # "#,
		);
	}

	#[test]
	fn loops_do_not_block() {
		test(
			[R, D, L, U],
			r#".0.1
			   .3.2"#,
			r#".3.0
			   .2.1"#,
		);
	}

	// Broken teams

	#[test]
	fn strong_cuts_weak() {
		// Down normally cuts right, but the rightward team is stronger.
		test(
			[D, R, R],
			r#". . .0. 
			   .1.2.X. 
			   . . . . "#,
			r#". . .0. 
			   . .1.2.X
			   . . . . "#,
		);
	}

	#[test]
	fn can_steal_from_blocked_team() {
		// With 0 blocked, the crate unambiguously belongs to 1's team.
		test(
			[D, R],
			r#". .0. 
			   .1.X. 
			   . # . "#,
			r#". .0. 
			   . .1.X
			   . # . "#,
		);
	}

	#[test]
	fn strong_uncut_subteam_continues_on() {
		// 3 has enough strength by itself to push the crate.
		test(
			[D, D, R, R],
			r#". .0. . . 
			   .2.X.3.X. 
			   . .1. . . 
			   . .X. . . 
			   . . . . . "#,
			r#". . . . . 
			   .2.0. .3.X
			   . .X. . . 
			   . .1. . . 
			   . .X. . . "#,
		);
	}

	#[test]
	fn weak_uncut_subteam_is_blocked() {
		// With 3 and 4 blocked, 5 can't push two crates.
		test(
			[D, D, D, R, R, R],
			r#". . .0. . . 
			   . . .1. . . 
			   .3.4.X.5.X.X
			   . . .2. . . 
			   . . .X. . . 
			   . . .X. . . 
			   . . . . . . "#,
			r#". . . . . . 
			   . . .0. . . 
			   .3.4.1.5.X.X
			   . . .X. . . 
			   . . .2. . . 
			   . . .X. . . 
			   . . .X. . . "#,
		);
	}

	// Collision resolution

	#[test]
	fn down_beats_right_left_up() {
		test(
			[D, U],
			r#".0
			   . 
			   .1"#,
			r#". 
			   .0
			   .1"#,
		);
		test(
			[D, R],
			r#". .0
			   .1. "#,
			r#". . 
			   .1.0"#,
		);
		test(
			[D, L],
			r#".0. 
			   . .1"#,
			r#". . 
			   .0.1"#,
		);
	}

	#[test]
	fn right_beats_left_up() {
		test(
			[R, U],
			r#".0. 
			   . .1"#,
			r#". .0
			   . .1"#,
		);
		test([R, L], r#".0. .1"#, r#". .0.1"#);
	}

	#[test]
	fn left_beats_up() {
		test(
			[L, U],
			r#". .0
			   .1. "#,
			r#".0. 
			   .1. "#,
		);
	}

	#[test]
	fn strong_blocks_weak() {
		// Down normally beats right, but the rightward team is stronger.
		test(
			[D, R],
			r#". .0
			   . .X
			   .1. "#,
			r#". .0
			   . .X
			   . .1"#,
		);
	}
}
