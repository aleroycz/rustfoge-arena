use rf_math::{Quat, Vec3};

pub trait Interpolate: Copy {
    fn interpolate(a: Self, b: Self, t: f32) -> Self;
}

impl Interpolate for Vec3 {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        a.lerp(b, t)
    }
}

impl Interpolate for Quat {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        a.slerp(b, t)
    }
}

impl Interpolate for f32 {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        rf_math::lerp(a, b, t)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
}

#[derive(Debug, Clone)]
pub struct Track<T> {
    keyframes: Vec<Keyframe<T>>,
}

impl<T: Interpolate> Track<T> {
    pub fn new(mut keyframes: Vec<Keyframe<T>>) -> Self {
        keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self { keyframes }
    }

    pub fn constant(value: T) -> Self {
        Self {
            keyframes: vec![Keyframe { time: 0.0, value }],
        }
    }

    pub fn duration(&self) -> f32 {
        self.keyframes.last().map(|k| k.time).unwrap_or(0.0)
    }

    pub fn sample(&self, time: f32) -> T {
        match self.keyframes.len() {
            0 => panic!("Track has no keyframes"),
            1 => return self.keyframes[0].value,
            _ => {}
        }

        if time <= self.keyframes[0].time {
            return self.keyframes[0].value;
        }
        if time >= self.keyframes[self.keyframes.len() - 1].time {
            return self.keyframes[self.keyframes.len() - 1].value;
        }

        let idx = self
            .keyframes
            .partition_point(|k| k.time <= time)
            .saturating_sub(1);
        let a = &self.keyframes[idx];
        let b = &self.keyframes[idx + 1];
        let span = (b.time - a.time).max(f32::EPSILON);
        let t = (time - a.time) / span;
        T::interpolate(a.value, b.value, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_midpoint() {
        let track = Track::new(vec![
            Keyframe {
                time: 0.0,
                value: 0.0_f32,
            },
            Keyframe {
                time: 2.0,
                value: 10.0,
            },
        ]);
        assert!((track.sample(1.0) - 5.0).abs() < 1e-5);
        assert_eq!(track.sample(-1.0), 0.0);
        assert_eq!(track.sample(5.0), 10.0);
    }
}
