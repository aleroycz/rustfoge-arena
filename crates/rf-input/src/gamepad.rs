use gilrs::{Axis, Button, Gilrs};
use std::collections::HashSet;

pub struct GamepadState {
    gilrs: Option<Gilrs>,
    buttons_held: HashSet<Button>,
    buttons_pressed: HashSet<Button>,
    buttons_released: HashSet<Button>,
}

impl Default for GamepadState {
    fn default() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(err) => {
                log::warn!("gamepad support disabled: {err}");
                None
            }
        };
        Self {
            gilrs,
            buttons_held: HashSet::new(),
            buttons_pressed: HashSet::new(),
            buttons_released: HashSet::new(),
        }
    }
}

impl GamepadState {
    pub fn poll(&mut self) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };
        while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _) => {
                    if self.buttons_held.insert(button) {
                        self.buttons_pressed.insert(button);
                    }
                }
                gilrs::EventType::ButtonReleased(button, _) => {
                    self.buttons_held.remove(&button);
                    self.buttons_released.insert(button);
                }
                _ => {}
            }
        }
    }

    pub fn end_frame(&mut self) {
        self.poll();
        self.buttons_pressed.clear();
        self.buttons_released.clear();
    }

    pub fn connected(&self) -> bool {
        self.gilrs
            .as_ref()
            .is_some_and(|g| g.gamepads().next().is_some())
    }

    pub fn button_held(&self, button: Button) -> bool {
        self.buttons_held.contains(&button)
    }

    pub fn button_pressed(&self, button: Button) -> bool {
        self.buttons_pressed.contains(&button)
    }

    pub fn axis(&self, axis: Axis) -> f32 {
        let Some(gilrs) = &self.gilrs else {
            return 0.0;
        };
        gilrs
            .gamepads()
            .next()
            .and_then(|(_, pad)| pad.axis_data(axis).map(|d| d.value()))
            .unwrap_or(0.0)
    }
}
