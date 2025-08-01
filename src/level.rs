use std::{
	cmp::Ordering,
	collections::BTreeSet,
	fmt::{Debug, Write},
	ops::{Add, AddAssign, Mul, Neg},
	sync::Arc,
};

use bevy::{
	platform::collections::{HashMap, HashSet},
	prelude::*,
};

use crate::control::Action;

/// Marker component for entities that should be despawned when the level is
/// despawned. Note that level entities are despawned recursively, so it's best
/// to only add this component to root entities.
///
/// TODO: Remove this component in favor of spawning levels as scenes.
#[derive(Component)]
pub struct LevelEntity;

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

impl Coords {
	pub fn transform(&self, z: f32) -> Transform {
		Transform::from_translation(Vec3::new(
			self.col as f32,
			-self.row as f32,
			z,
		))
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

	/// The angle formed by `self` relative to [`Offset::RIGHT`].
	pub fn angle(&self) -> f32 {
		(-self.row as f32).atan2(self.col as f32)
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
	Floor {
		portal_color: Option<CharacterColor>,
	},
	Wall,
	Stairs,
}

/// An object identifier. Enables correlating object animations across frames.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u32);

/// Distinguishes between characters and links them to their return portals.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CharacterColor {
	Green,
	Red,
	Blue,
	Yellow,
	Magenta,
	Cyan,
	Black,
	White,
}

impl CharacterColor {
	// TODO: Replace with std::mem::variant_count when stabilized.
	pub const COUNT: usize = 8;

	pub fn idx(&self) -> usize {
		*self as usize
	}

	pub fn color(&self) -> Color {
		match self {
			CharacterColor::Green => Color::srgb(0.2, 0.7, 0.2),
			CharacterColor::Red => Color::srgb(0.7, 0.2, 0.2),
			CharacterColor::Blue => Color::srgb(0.2, 0.2, 0.7),
			CharacterColor::Yellow => Color::srgb(0.7, 0.7, 0.2),
			CharacterColor::Magenta => Color::srgb(0.7, 0.2, 0.7),
			CharacterColor::Cyan => Color::srgb(0.2, 0.7, 0.7),
			CharacterColor::Black => Color::srgb(0.2, 0.2, 0.2),
			CharacterColor::White => Color::srgb(0.7, 0.7, 0.7),
		}
	}
}

impl<T> From<T> for CharacterColor
where
	T: Into<usize>,
{
	fn from(value: T) -> Self {
		let idx: usize = value.into();
		match idx {
			0 => CharacterColor::Green,
			1 => CharacterColor::Red,
			2 => CharacterColor::Blue,
			3 => CharacterColor::Yellow,
			4 => CharacterColor::Magenta,
			5 => CharacterColor::Cyan,
			6 => CharacterColor::Black,
			7 => CharacterColor::White,
			_ => panic!("color out of bounds: {idx}"),
		}
	}
}

/// A playable character.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Character {
	pub color: CharacterColor,
	pub sliding: bool,
	pub portal_coords: Option<Coords>,
}

impl Character {
	pub fn can_push(&self) -> bool {
		!self.sliding
	}

	pub fn can_summon(&self) -> bool {
		self.portal_coords.is_none()
	}

	pub fn can_return(&self) -> bool {
		self.portal_coords.is_some()
	}
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

impl From<&LevelCharacter> for LevelObject {
	fn from(level_character: &LevelCharacter) -> Self {
		LevelObject {
			id: level_character.id,
			object: Object::Character(level_character.character),
			coords: level_character.coords,
			angle: level_character.angle,
		}
	}
}

/// A [`Character`] along with data relating that character to a [`Level`]. (See
/// also [`LevelObject`].)
#[derive(Clone)]
pub struct LevelCharacter {
	pub id: Id,
	pub character: Character,
	pub coords: Coords,
	pub angle: f32,
}

/// The complete state of a level at a single point in time.
#[derive(Resource)]
pub struct Level {
	width: usize,
	height: usize,
	tiles: Vec<Tile>,
	objects_by_id: HashMap<Id, LevelObject>,
	object_ids_by_coords: HashMap<Coords, Id>,
	character_ids: BTreeSet<Id>,
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

	/// The index of the tile at `coords`.
	fn tile_idx(&self, coords: Coords) -> usize {
		coords.row as usize * self.width + coords.col as usize
	}

	/// The tile at `coords`.
	pub fn tile_at(&self, coords: Coords) -> Tile {
		self.tiles[self.tile_idx(coords)]
	}

	/// Sets the tile at `coords` to `tile`.
	pub fn set_tile_at(&mut self, coords: Coords, tile: Tile) {
		let idx = self.tile_idx(coords);
		self.tiles[idx] = tile;
	}

	/// The object at `coords`, if any.
	pub fn object_at(&self, coords: Coords) -> Option<Object> {
		self.object_ids_by_coords
			.get(&coords)
			.and_then(|id| self.objects_by_id.get(id))
			.map(|level_object| level_object.object)
	}

	// TODO: This method probably won't be necessary if I move the initial
	// entity spawning logic into animation instead of main.
	/// Iterates over all objects in the level.
	pub fn iter_level_objects(&self) -> impl Iterator<Item = &LevelObject> {
		self.objects_by_id.values()
	}

	fn level_character_by_id(&self, id: &Id) -> LevelCharacter {
		let level_object = &self.objects_by_id[id];
		let Object::Character(character) = level_object.object else {
			panic!("object is not a character");
		};
		LevelCharacter {
			id: level_object.id,
			character,
			coords: level_object.coords,
			angle: level_object.angle,
		}
	}

	/// A reference to the character with the given `id`. Panics if there is no
	/// character with that ID.
	pub fn character_by_id(&self, id: &Id) -> &Character {
		let Object::Character(character) = &self.objects_by_id[id].object
		else {
			panic!("character not found");
		};
		character
	}

	/// A mutable reference to the character with the given `id`. Panics if
	/// there is no character with that ID.
	pub fn character_by_id_mut(&mut self, id: &Id) -> &mut Character {
		let Object::Character(character) =
			&mut self.objects_by_id.get_mut(id).unwrap().object
		else {
			panic!("character not found");
		};
		character
	}

	/// Characters in the level, with their IDs.
	pub fn characters_by_id(&self) -> impl Iterator<Item = (&Id, &Character)> {
		self.character_ids
			.iter()
			.map(|id| (id, self.character_by_id(id)))
	}

	/// Number of characters in the level.
	pub fn character_count(&self) -> usize {
		self.character_ids.len()
	}

	/// Updates the level by making the `actors` act, returning the resulting
	/// (possibly trivial) [`Change`].
	///
	/// Actions are resolved in three phases: (1) return, (2) push, and (3)
	/// summon. Actions within each phase are simultaneous.
	///
	/// Any two summoners must summon into disjoint coordinates. This
	/// precondition will generally be trivially satisfied since there should be
	/// at most one summoner per update.
	pub fn update(&mut self, actors: Vec<(Id, Action)>) -> ChangeEvent {
		// Map pushers and summoners to their offsets.
		let (pushers, summoners, returners) = {
			let mut pushers = HashMap::new();
			let mut summoners = HashMap::new();
			let mut returners = HashSet::new();
			for (id, action) in actors {
				match action {
					Action::Push(offset) => {
						pushers.insert(id, offset);
					}
					Action::Summon(offset) => {
						summoners.insert(id, offset);
					}
					Action::Return => {
						returners.insert(id);
					}
					Action::Wait => {}
				}
			}
			(pushers, summoners, returners)
		};

		let returnings = self.get_returnings(returners);
		self.apply_returnings(&returnings);

		let moves = self.get_moves(pushers);
		self.apply_moves(&moves);

		let summonings = self.get_summonings(summoners);
		self.apply_summonings(&summonings);

		// Add the change to the turn history and then return it.
		let change = Change {
			returnings,
			moves,
			summonings,
		};
		let reverse = Arc::new(change.clone().reverse());
		let change = Arc::new(change);
		// Truncate history to remove any future states. This is a no-op if the
		// level is already at the end of its history.
		self.history.truncate(self.turn);
		self.history.push(BiChange {
			forward: change.clone(),
			reverse,
		});
		self.turn += 1;
		ChangeEvent(change)
	}

	/// Computes the set of [`Returning`]s resulting from the given `returners`.
	fn get_returnings(
		&mut self,
		returners: HashSet<Id>,
	) -> HashMap<Id, Returning> {
		returners
			.into_iter()
			.filter_map(|id| {
				let returner = self.level_character_by_id(&id);
				returner.character.portal_coords.and_then(|portal_coords| {
					(portal_coords == returner.coords).then_some((
						returner.id,
						Returning {
							returner,
							linked_id: id,
						},
					))
				})
			})
			.collect()
	}

	/// Computes the set of [`Move`]s resulting from the given `pushers`.
	fn get_moves(&self, pushers: HashMap<Id, Offset>) -> HashMap<Id, Move> {
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
					if let Tile::Wall = self.tile_at(coords) {
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
		moves
	}

	/// Computes the list of colors not yet taken by any character. The results
	/// are deterministic.
	fn get_available_colors(&self) -> Vec<CharacterColor> {
		let character_colors = HashSet::<CharacterColor>::from_iter(
			self.characters_by_id()
				.map(|(_, character)| character.color),
		);
		(0..CharacterColor::COUNT)
			.filter_map(|idx| {
				let color = idx.into();
				(!character_colors.contains(&color)).then_some(color)
			})
			.collect()
	}

	/// Computes the set of [`Summoning`]s resulting from the given `summoners`.
	///
	/// Any two summoners must summon into disjoint coordinates. This
	/// precondition will generally be trivially satisfied since there should be
	/// at most one summoner per update.
	fn get_summonings(
		&mut self,
		summoners: HashMap<Id, Offset>,
	) -> HashMap<Id, Summoning> {
		summoners
			.into_iter()
			.zip(self.get_available_colors())
			.filter_map(|((summoner_id, offset), summon_color)| {
				let summon_id = self.new_object_id();
				let level_summoner = self.level_character_by_id(&summoner_id);
				self.farthest_open_tile(level_summoner.coords, offset).map(
					|coords| {
						(
							summoner_id,
							Summoning {
								summon: LevelCharacter {
									id: summon_id,
									character: Character {
										color: summon_color,
										sliding: false,
										portal_coords: None,
									},
									coords,
									angle: 0.0,
								},
								linked_id: summoner_id,
								portal_color: level_summoner.character.color,
							},
						)
					},
				)
			})
			.collect()
	}

	/// The empty floor tile most distant from `start` incrementing by `offset`.
	fn farthest_open_tile(
		&self,
		start: Coords,
		offset: Offset,
	) -> Option<Coords> {
		let mut result = None;
		let mut coords = start;
		loop {
			coords += offset;
			if coords.row < 0
				|| coords.row >= self.height() as i32
				|| coords.col < 0
				|| coords.col >= self.width() as i32
			{
				break;
			}
			if let (Tile::Floor { portal_color: None }, None) =
				(self.tile_at(coords), self.object_at(coords))
			{
				result = Some(coords);
			}
		}
		result
	}

	/// If possible, moves to the previous level state and returns the resulting
	/// [`ChangeEvent`].
	pub fn undo(&mut self) -> Option<ChangeEvent> {
		if self.turn > 0 {
			let change = self.history[self.turn - 1].reverse.clone();
			self.apply(&change);
			self.turn -= 1;
			Some(ChangeEvent(change))
		} else {
			None
		}
	}

	/// If possible, moves to the next level state and returns the resulting
	/// [`ChangeEvent`].
	pub fn redo(&mut self) -> Option<ChangeEvent> {
		if self.turn < self.history.len() {
			let change = self.history[self.turn].forward.clone();
			self.apply(&change);
			self.turn += 1;
			Some(ChangeEvent(change))
		} else {
			None
		}
	}

	/// Applies `change` to the level's state without affecting history.
	fn apply(&mut self, change: &Change) {
		self.apply_returnings(&change.returnings);
		self.apply_moves(&change.moves);
		self.apply_summonings(&change.summonings);
	}

	/// Applies `returnings` to the level's state without affecting history.
	fn apply_returnings(&mut self, returnings: &HashMap<Id, Returning>) {
		for returning in returnings.values() {
			// Unlink linked character from portal.
			self.character_by_id_mut(&returning.linked_id).portal_coords = None;
			// Remove returning character.
			self.remove_at(returning.returner.coords);
			// Close portal.
			self.set_tile_at(
				returning.returner.coords,
				Tile::Floor { portal_color: None },
			);
		}
	}

	/// Applies `moves` to the level's state without affecting history.
	fn apply_moves(&mut self, moves: &HashMap<Id, Move>) {
		// To make sure every target tile is open, first remove all movers.
		for mv in moves.values() {
			self.object_ids_by_coords.remove(&mv.from_coords);
		}
		// Now place the movers into their new tiles.
		for (id, mv) in moves.iter() {
			let level_object = self.objects_by_id.get_mut(id).unwrap();
			self.object_ids_by_coords
				.insert(mv.to_coords, level_object.id);
			level_object.coords = mv.to_coords;
			level_object.angle = mv.to_angle;
		}
	}

	/// Applies `summonings` to the level's state without affecting history.
	fn apply_summonings(&mut self, summonings: &HashMap<Id, Summoning>) {
		for (summoner_id, summoning) in summonings {
			// Open portal.
			self.set_tile_at(
				summoning.summon.coords,
				Tile::Floor {
					portal_color: Some(summoning.portal_color),
				},
			);
			// Summon character from the future.
			self.spawn((&summoning.summon).into());
			// Link summoner to portal.
			self.character_by_id_mut(summoner_id).portal_coords =
				Some(summoning.summon.coords);
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

	/// A fresh object ID.
	fn new_object_id(&mut self) -> Id {
		let id = self.next_object_id;
		self.next_object_id.0 += 1;
		id
	}

	/// Spawns `level_object` into the level. The caller is responsible for
	/// ensuring `level_object`'s ID is currently available.
	fn spawn(&mut self, level_object: LevelObject) {
		self.object_ids_by_coords
			.insert(level_object.coords, level_object.id);
		if let Object::Character(..) = level_object.object {
			self.character_ids.insert(level_object.id);
		}
		self.objects_by_id.insert(level_object.id, level_object);
	}

	/// Removes the object at `coords`, if there is one.
	fn remove_at(&mut self, coords: Coords) {
		if let Some(removed_id) = self.object_ids_by_coords.remove(&coords) {
			self.objects_by_id.remove(&removed_id);
			self.character_ids.remove(&removed_id);
		}
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
					self.object_at(coords) == other.object_at(coords)
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
				let tile = self.tile_at(coords);
				let object = self.object_at(coords);
				f.write_char(match tile {
					Tile::Floor { portal_color } => {
						if portal_color.is_some() {
							'o'
						} else {
							'.'
						}
					}
					Tile::Wall => '#',
					Tile::Stairs => '>',
				})?;
				f.write_char(match object {
					Some(Object::Character(c)) => {
						(b'0' + c.color.idx() as u8) as char
					}
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

/// A character's return to the past.
#[derive(Clone)]
pub struct Returning {
	pub returner: LevelCharacter,
	pub linked_id: Id,
}

impl Returning {
	fn reverse(self) -> Summoning {
		let portal_color = self.returner.character.color;
		Summoning {
			summon: self.returner,
			linked_id: self.linked_id,
			portal_color,
		}
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
	fn reverse(self) -> Move {
		Move {
			from_coords: self.to_coords,
			to_coords: self.from_coords,
			from_angle: self.to_angle,
			to_angle: self.from_angle,
		}
	}
}

/// A character's summoning from the future.
#[derive(Clone)]
pub struct Summoning {
	pub summon: LevelCharacter,
	pub linked_id: Id,
	pub portal_color: CharacterColor,
}

impl Summoning {
	fn reverse(self) -> Returning {
		Returning {
			returner: self.summon,
			linked_id: self.linked_id,
		}
	}
}

/// A change from one [`Level`] state to another.
#[derive(Clone)]
pub struct Change {
	pub returnings: HashMap<Id, Returning>,
	pub moves: HashMap<Id, Move>,
	pub summonings: HashMap<Id, Summoning>,
}

impl Change {
	fn reverse(self) -> Change {
		Change {
			returnings: self
				.summonings
				.into_iter()
				.map(|(id, returning)| (id, returning.reverse()))
				.collect(),
			moves: self
				.moves
				.into_iter()
				.map(|(id, mv)| (id, mv.reverse()))
				.collect(),
			summonings: self
				.returnings
				.into_iter()
				.map(|(id, summon)| (id, summon.reverse()))
				.collect(),
		}
	}
}

/// A [`Change`] event. Note that `Change` itself can't be an [`Event`] because
/// it's not [`Sync`].
#[derive(Event, Deref)]
pub struct ChangeEvent(Arc<Change>);

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
		   # . .0. . . . > # 
		   # . . . . . . . # 
		   # . . . . . . . # 
		   # . .X.Y.Z. . . # 
		   # . .X.Y. . . . # 
		   # . .X. . . . . # 
		   # . . . . . . > # 
		   # # # # # # # # # "#,
	)
}

/// Makes a fresh copy of a flat test level.
pub fn test_level_short() -> Level {
	make_level(
		r#"# # # # # # # # # 
		   # . .0. . . . . # 
		   # # # # # # # # # "#,
	)
}

/// Makes a fresh copy of a thin test level.
pub fn test_level_thin() -> Level {
	make_level(
		r#"# # # 
		   # .0# 
		   # . # 
		   # . # 
		   # .X# 
		   # .X# 
		   # . # 
		   # . # 
		   # # # "#,
	)
}

/// Makes a fresh copy of a large test level.
pub fn test_level_large() -> Level {
	make_level(
		r#"# # # # # # # # # # # # # # # # # # # # # # 
		   # . .0. . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . .X.Y.Z. . . . . . . . . . . . . . . . # 
		   # . .X.Y. . . . . . . . . . . . . . . . . # 
		   # . .X. . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # . . . . . . . . . . . . . . . . . . . . # 
		   # # # # # # # # # # # # # # # # # # # # # # "#,
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
		.map(str::trim_start)
		.filter(|line| !line.is_empty())
		.enumerate()
	{
		height = height.max(row + 1);
		for (col, tile_object) in line.as_bytes().chunks_exact(2).enumerate() {
			width = width.max(col + 1);
			let (tile, object) = (tile_object[0], tile_object[1]);
			tiles.push(match tile {
				b'#' => Tile::Wall,
				b'>' => Tile::Stairs,
				_ => Tile::Floor { portal_color: None },
			});
			if let Some(object) = match object {
				b'0'..=b'7' => Some(Object::Character(Character {
					color: CharacterColor::from(object - b'0'),
					sliding: false,
					portal_coords: None,
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
				c1.color.cmp(&c2.color)
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
		character_ids: BTreeSet::new(),
		next_object_id: Id(0),
		history: Vec::new(),
		turn: 0,
	};
	for (object, coords) in object_coords {
		let id = level.new_object_id();
		level.spawn(LevelObject {
			id,
			object,
			coords,
			angle: 0.0,
		});
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
		let character_actions =
			level.character_ids.iter().copied().zip(actions).collect();
		level.update(character_actions);
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
