use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        let forward = (target - self.translation).normalize_or_zero();
        if forward != Vec3::ZERO {
            self.rotation = Quat::from_mat3(&glam::Mat3::from_cols(
                forward.cross(up).normalize_or_zero(),
                up,
                -forward,
            ));
        }
        self
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    pub fn mul_transform(&self, child: &Transform) -> Transform {
        let translation = self.translation + self.rotation * (self.scale * child.translation);
        let rotation = self.rotation * child.rotation;
        let scale = self.scale * child.scale;
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    pub fn inverse(&self) -> Transform {
        let rotation = self.rotation.inverse();
        let scale = 1.0 / self.scale;
        let translation = rotation * (-self.translation * scale);
        Transform {
            translation,
            rotation,
            scale,
        }
    }
}

impl From<Transform> for Mat4 {
    fn from(t: Transform) -> Self {
        t.matrix()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_matrix_is_identity() {
        assert_eq!(Transform::IDENTITY.matrix(), Mat4::IDENTITY);
    }

    #[test]
    fn composition_matches_matrix_multiplication() {
        let a = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0))
            .with_rotation(Quat::from_rotation_y(0.7))
            .with_scale(Vec3::splat(2.0));
        let b = Transform::from_translation(Vec3::new(0.5, -1.0, 0.0))
            .with_rotation(Quat::from_rotation_x(0.3));

        let composed = a.mul_transform(&b).matrix();
        let expected = a.matrix() * b.matrix();

        for i in 0..4 {
            for j in 0..4 {
                assert!((composed.col(i)[j] - expected.col(i)[j]).abs() < 1e-4);
            }
        }
    }
}
