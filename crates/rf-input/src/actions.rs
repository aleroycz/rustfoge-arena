use crate::InputState;
use gilrs::{Axis as GamepadAxis, Button as GamepadButton};
use std::collections::HashMap;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    GamepadButton(GamepadButton),
}

#[derive(Debug, Clone, Copy)]
pub enum AxisBinding {
    Keys {
        positive: KeyCode,
        negative: KeyCode,
    },
    GamepadAxis(GamepadAxis),
}

#[derive(Default)]
pub struct ActionMap {
    digital: HashMap<String, Vec<Binding>>,
    axes: HashMap<String, Vec<AxisBinding>>,
}

impl ActionMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_action(&mut self, action: impl Into<String>, binding: Binding) -> &mut Self {
        self.digital.entry(action.into()).or_default().push(binding);
        self
    }

    pub fn bind_axis(&mut self, axis: impl Into<String>, binding: AxisBinding) -> &mut Self {
        self.axes.entry(axis.into()).or_default().push(binding);
        self
    }

    pub fn action_held(&self, input: &InputState, action: &str) -> bool {
        self.digital
            .get(action)
            .into_iter()
            .flatten()
            .any(|b| binding_held(input, *b))
    }

    pub fn action_pressed(&self, input: &InputState, action: &str) -> bool {
        self.digital
            .get(action)
            .into_iter()
            .flatten()
            .any(|b| binding_pressed(input, *b))
    }

    pub fn axis_value(&self, input: &InputState, axis: &str) -> f32 {
        let value: f32 = self
            .axes
            .get(axis)
            .into_iter()
            .flatten()
            .map(|b| axis_binding_value(input, *b))
            .sum();
        value.clamp(-1.0, 1.0)
    }
}

fn binding_held(input: &InputState, binding: Binding) -> bool {
    match binding {
        Binding::Key(key) => input.key_held(key),
        Binding::Mouse(button) => input.mouse_held(button),
        Binding::GamepadButton(button) => input.gamepad.button_held(button),
    }
}

fn binding_pressed(input: &InputState, binding: Binding) -> bool {
    match binding {
        Binding::Key(key) => input.key_pressed(key),
        Binding::Mouse(button) => input.mouse_pressed(button),
        Binding::GamepadButton(button) => input.gamepad.button_pressed(button),
    }
}

fn axis_binding_value(input: &InputState, binding: AxisBinding) -> f32 {
    match binding {
        AxisBinding::Keys { positive, negative } => {
            let mut v = 0.0;
            if input.key_held(positive) {
                v += 1.0;
            }
            if input.key_held(negative) {
                v -= 1.0;
            }
            v
        }
        AxisBinding::GamepadAxis(axis) => input.gamepad.axis(axis),
    }
}
