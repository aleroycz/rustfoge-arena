use std::collections::HashMap;

use rf_math::Transform;

use crate::Track;

struct JointTrack {
    translation: Track<rf_math::Vec3>,
    rotation: Track<rf_math::Quat>,
    scale: Track<rf_math::Vec3>,
}

pub struct AnimationClip {
    pub name: String,
    duration: f32,
    joint_tracks: HashMap<usize, JointTrack>,
}

impl AnimationClip {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: 0.0,
            joint_tracks: HashMap::new(),
        }
    }

    pub fn add_joint_tracks(
        &mut self,
        joint_index: usize,
        translation: Track<rf_math::Vec3>,
        rotation: Track<rf_math::Quat>,
        scale: Track<rf_math::Vec3>,
    ) {
        self.duration = self
            .duration
            .max(translation.duration())
            .max(rotation.duration())
            .max(scale.duration());
        self.joint_tracks.insert(
            joint_index,
            JointTrack {
                translation,
                rotation,
                scale,
            },
        );
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn sample_joint(&self, joint_index: usize, time: f32) -> Option<Transform> {
        let track = self.joint_tracks.get(&joint_index)?;
        Some(Transform {
            translation: track.translation.sample(time),
            rotation: track.rotation.sample(time),
            scale: track.scale.sample(time),
        })
    }

    pub fn animated_joints(&self) -> impl Iterator<Item = usize> + '_ {
        self.joint_tracks.keys().copied()
    }
}
