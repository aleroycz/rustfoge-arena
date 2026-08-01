use std::sync::Arc;

use crate::{AnimationClip, Pose, Skeleton};

#[derive(Clone)]
struct Playing {
    clip: Arc<AnimationClip>,
    time: f32,
    speed: f32,
    looping: bool,
}

impl Playing {
    fn advance(&mut self, dt: f32) {
        self.time += dt * self.speed;
        let duration = self.clip.duration().max(f32::EPSILON);
        if self.looping {
            self.time = self.time.rem_euclid(duration);
        } else {
            self.time = self.time.clamp(0.0, duration);
        }
    }

    fn finished(&self) -> bool {
        !self.looping && self.time >= self.clip.duration()
    }
}

pub struct AnimationPlayer {
    current: Option<Playing>,
    previous: Option<Playing>,

    blend: f32,
    blend_duration: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Finished,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            current: None,
            previous: None,
            blend: 1.0,
            blend_duration: 0.2,
        }
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn play(&mut self, clip: Arc<AnimationClip>, looping: bool, blend_duration: f32) {
        if let Some(current) = self.current.take() {
            self.previous = Some(current);
            self.blend = 0.0;
        }
        self.blend_duration = blend_duration.max(f32::EPSILON);
        self.current = Some(Playing {
            clip,
            time: 0.0,
            speed: 1.0,
            looping,
        });
    }

    pub fn set_speed(&mut self, speed: f32) {
        if let Some(current) = &mut self.current {
            current.speed = speed;
        }
    }

    pub fn advance(&mut self, dt: f32) {
        if let Some(current) = &mut self.current {
            current.advance(dt);
        }
        if let Some(previous) = &mut self.previous {
            previous.advance(dt);
        }
        if self.blend < 1.0 {
            self.blend = (self.blend + dt / self.blend_duration).min(1.0);
            if self.blend >= 1.0 {
                self.previous = None;
            }
        }
    }

    pub fn state(&self) -> PlaybackState {
        match &self.current {
            None => PlaybackState::Idle,
            Some(c) if c.finished() => PlaybackState::Finished,
            Some(_) => PlaybackState::Playing,
        }
    }

    pub fn sample(&self, skeleton: &Skeleton) -> Pose {
        let current_pose = match &self.current {
            Some(c) => Pose::sample(skeleton, &c.clip, c.time),
            None => skeleton.rest_pose(),
        };
        match &self.previous {
            Some(prev) if self.blend < 1.0 => {
                let previous_pose = Pose::sample(skeleton, &prev.clip, prev.time);
                Pose::blend(&previous_pose, &current_pose, self.blend)
            }
            _ => current_pose,
        }
    }
}
