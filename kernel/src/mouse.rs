//! General mouse definitions and methods

use exclusive_cell::ExclusiveCell;

/// Represents the current state of the mouse
pub struct MouseState {
	left_button_down: bool,
	right_button_down: bool,
	middle_button_down: bool,
	fourth_button_down: bool,
	fifth_button_down: bool,
	x: f32,
	y: f32,
}

impl MouseState {
	/// Constructs a mouse state object that represents the state of the mouse on start-up
	const fn new() -> Self {
		Self {
			left_button_down: false,
			right_button_down: false,
			middle_button_down: false,
			fourth_button_down: false,
			fifth_button_down: false,
			x: 0.0,
			y: 0.0,
		}
	}
}

/// The global mouse state. Access should be exclusive: we do not expect to recieve two mouse
/// events simultaneously
static MOUSE_STATE: ExclusiveCell<MouseState> = ExclusiveCell::new(MouseState::new());

pub fn mouse_event(left_down: bool, right_down: bool, middle_down: bool, fourth_down: bool,
	fifth_down: bool, x_delta: i32, y_delta: i32, z_delta: i32) {
	let mut mouse_state = MOUSE_STATE.acquire();

	mouse_state.left_button_down = left_down;
	mouse_state.right_button_down = right_down;
	mouse_state.middle_button_down = middle_down;
	mouse_state.fourth_button_down = fourth_down;
	mouse_state.fifth_button_down = fifth_down;
	if left_down {
		mouse_state.x += (x_delta as f32)/20.;
	}
	mouse_state.y += (y_delta as f32)/20.;

	crate::screen::set_cursor_offset(mouse_state.x as usize);
	// crate::println!("X: {} ({})", mouse_state.x as usize, mouse_state.x);
}