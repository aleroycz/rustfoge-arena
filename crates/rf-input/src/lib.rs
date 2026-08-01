mod actions;
mod gamepad;
mod state;

pub use actions::{ActionMap, AxisBinding, Binding};
pub use gamepad::GamepadState;
pub use state::InputState;

pub use gilrs::{Axis as GamepadAxis, Button as GamepadButton};
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;
