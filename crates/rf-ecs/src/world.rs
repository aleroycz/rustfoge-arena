use std::ops::{Deref, DerefMut};

#[derive(Default)]
pub struct World {
    inner: hecs::World,
}

impl World {
    pub fn new() -> Self {
        Self {
            inner: hecs::World::new(),
        }
    }

    pub fn inner(&self) -> &hecs::World {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut hecs::World {
        &mut self.inner
    }
}

impl Deref for World {
    type Target = hecs::World;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for World {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
