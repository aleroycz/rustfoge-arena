use glam::Vec2;
use std::collections::HashSet;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::GamepadState;

#[derive(Default)]
pub struct InputState {
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,

    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_released: HashSet<MouseButton>,

    mouse_position: Vec2,
    mouse_delta: Vec2,
    scroll_delta: f32,
    text_input: String,

    pub gamepad: GamepadState,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                match event.state {
                    ElementState::Pressed => {
                        if self.keys_held.insert(code) {
                            self.keys_pressed.insert(code);
                        }

                        if let Some(text) = &event.text {
                            self.text_input.push_str(text);
                        }
                    }
                    ElementState::Released => {
                        self.keys_held.remove(&code);
                        self.keys_released.insert(code);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if self.mouse_buttons_held.insert(*button) {
                        self.mouse_buttons_pressed.insert(*button);
                    }
                }
                ElementState::Released => {
                    self.mouse_buttons_held.remove(button);
                    self.mouse_buttons_released.insert(*button);
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll_delta += match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 16.0) as f32,
                };
            }
            _ => {}
        }
    }

    pub fn process_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.mouse_delta += Vec2::new(delta.0 as f32, delta.1 as f32);
        }
    }

    pub fn end_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = Vec2::ZERO;
        self.scroll_delta = 0.0;
        self.text_input.clear();
        self.gamepad.end_frame();
    }

    pub fn typed_text(&self) -> &str {
        &self.text_input
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn scroll_delta(&self) -> f32 {
        self.scroll_delta
    }

    pub fn wasd_axis(&self) -> Vec2 {
        let mut v = Vec2::ZERO;
        if self.key_held(KeyCode::KeyW) || self.key_held(KeyCode::ArrowUp) {
            v.y += 1.0;
        }
        if self.key_held(KeyCode::KeyS) || self.key_held(KeyCode::ArrowDown) {
            v.y -= 1.0;
        }
        if self.key_held(KeyCode::KeyD) || self.key_held(KeyCode::ArrowRight) {
            v.x += 1.0;
        }
        if self.key_held(KeyCode::KeyA) || self.key_held(KeyCode::ArrowLeft) {
            v.x -= 1.0;
        }
        v
    }
}
