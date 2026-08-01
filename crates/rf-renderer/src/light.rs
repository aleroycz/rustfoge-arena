use bytemuck::{Pod, Zeroable};
use rf_math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub ambient: Vec3,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.4, -1.0, -0.3).normalize(),
            color: Vec3::new(1.0, 0.97, 0.92),
            intensity: 1.3,
            ambient: Vec3::splat(0.12),
        }
    }
}

impl DirectionalLight {
    pub fn uniform(&self) -> LightUniform {
        LightUniform {
            direction: [self.direction.x, self.direction.y, self.direction.z, 0.0],
            color: [
                self.color.x * self.intensity,
                self.color.y * self.intensity,
                self.color.z * self.intensity,
                0.0,
            ],
            ambient: [self.ambient.x, self.ambient.y, self.ambient.z, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LightUniform {
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub ambient: [f32; 4],
}
