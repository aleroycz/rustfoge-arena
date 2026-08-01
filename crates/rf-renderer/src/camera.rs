use bytemuck::{Pod, Zeroable};
use rf_math::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 3.0, 8.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y_radians: 60_f32.to_radians(),
            aspect: 16.0 / 9.0,
            z_near: 0.1,
            z_far: 500.0,
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y_radians, self.aspect, self.z_near, self.z_far)
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn uniform(&self) -> CameraUniform {
        let forward = (self.target - self.position).normalize_or_zero();
        let right = forward.cross(self.up).normalize_or_zero();
        let true_up = right.cross(forward);
        CameraUniform {
            view_proj: self.view_projection_matrix().to_cols_array_2d(),
            view_pos: [self.position.x, self.position.y, self.position.z, 1.0],
            right: [right.x, right.y, right.z, 0.0],
            up: [true_up.x, true_up.y, true_up.z, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 4],

    pub right: [f32; 4],
    pub up: [f32; 4],
}
