pub use glam::{EulerRot, Mat3, Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};

mod aabb;
mod ray;
mod transform;

pub use aabb::Aabb;
pub use ray::Ray;
pub use transform::Transform;

pub const WORLD_UP: Vec3 = Vec3::Y;
pub const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
pub const WORLD_RIGHT: Vec3 = Vec3::X;

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn saturate(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}
