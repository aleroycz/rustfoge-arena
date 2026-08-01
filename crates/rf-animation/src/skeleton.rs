use rf_math::{Mat4, Transform};
use std::collections::HashMap;

use crate::AnimationClip;

#[derive(Debug, Clone)]
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,

    pub local_rest_pose: Transform,

    pub inverse_bind_matrix: Mat4,
}

pub struct Skeleton {
    pub joints: Vec<Joint>,
    name_to_index: HashMap<String, usize>,
}

impl Skeleton {
    pub fn new(joints: Vec<Joint>) -> Self {
        let name_to_index = joints
            .iter()
            .enumerate()
            .map(|(i, j)| (j.name.clone(), i))
            .collect();
        Self {
            joints,
            name_to_index,
        }
    }

    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    pub fn rest_pose(&self) -> Pose {
        Pose {
            local_transforms: self.joints.iter().map(|j| j.local_rest_pose).collect(),
        }
    }

    pub fn skinning_matrices(&self, pose: &Pose) -> Vec<Mat4> {
        let mut globals = vec![Mat4::IDENTITY; self.joints.len()];
        for (i, joint) in self.joints.iter().enumerate() {
            let local = pose.local_transforms[i].matrix();
            globals[i] = match joint.parent {
                Some(parent) => globals[parent] * local,
                None => local,
            };
        }
        globals
            .iter()
            .zip(&self.joints)
            .map(|(global, joint)| *global * joint.inverse_bind_matrix)
            .collect()
    }
}

#[derive(Clone)]
pub struct Pose {
    pub local_transforms: Vec<Transform>,
}

impl Pose {
    pub fn sample(skeleton: &Skeleton, clip: &AnimationClip, time: f32) -> Self {
        let local_transforms = skeleton
            .joints
            .iter()
            .enumerate()
            .map(|(i, joint)| clip.sample_joint(i, time).unwrap_or(joint.local_rest_pose))
            .collect();
        Self { local_transforms }
    }

    pub fn blend(a: &Pose, b: &Pose, t: f32) -> Pose {
        let local_transforms = a
            .local_transforms
            .iter()
            .zip(&b.local_transforms)
            .map(|(a, b)| rf_math::Transform {
                translation: a.translation.lerp(b.translation, t),
                rotation: a.rotation.slerp(b.rotation, t),
                scale: a.scale.lerp(b.scale, t),
            })
            .collect();
        Pose { local_transforms }
    }
}
