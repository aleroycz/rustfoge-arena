use bytemuck::{Pod, Zeroable};

#[derive(Debug, Clone, Copy)]
pub struct FogSettings {
    pub color: [f32; 3],

    pub density: f32,

    pub height_falloff: f32,
}

impl Default for FogSettings {
    fn default() -> Self {
        Self {
            color: [0.7, 0.75, 0.8],
            density: 0.008,
            height_falloff: 0.05,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct FogUniform {
    pub color_density: [f32; 4],
    pub params: [f32; 4],
}

impl FogSettings {
    pub(crate) fn uniform(&self) -> FogUniform {
        FogUniform {
            color_density: [self.color[0], self.color[1], self.color[2], self.density],
            params: [self.height_falloff, 0.0, 0.0, 0.0],
        }
    }
}
