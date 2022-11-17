/// The state of the game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GameState {
	/// Receiving player input
	Control,
	/// Animating level change
	Animate,
}
