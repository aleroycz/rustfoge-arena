use crate::{Time, World};

pub trait System: 'static {
    fn run(&mut self, world: &mut World, time: &Time);
}

impl<F> System for F
where
    F: FnMut(&mut World, &Time) + 'static,
{
    fn run(&mut self, world: &mut World, time: &Time) {
        (self)(world, time)
    }
}

#[derive(Default)]
pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: impl System) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    pub fn run(&mut self, world: &mut World, time: &Time) {
        for system in &mut self.systems {
            system.run(world, time);
        }
    }
}
