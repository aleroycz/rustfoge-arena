use std::collections::VecDeque;

pub struct Events<T> {
    current: VecDeque<T>,
    previous: VecDeque<T>,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Self {
            current: VecDeque::new(),
            previous: VecDeque::new(),
        }
    }
}

impl<T> Events<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send(&mut self, event: T) {
        self.current.push_back(event);
    }

    pub fn update(&mut self) {
        self.previous = std::mem::take(&mut self.current);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.previous.iter().chain(self.current.iter())
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_empty() && self.previous.is_empty()
    }
}
