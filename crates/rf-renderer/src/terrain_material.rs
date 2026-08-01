use bytemuck::{Pod, Zeroable};

use crate::GpuTexture;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TerrainParamsUniform {
    params: [f32; 4],
}

pub struct TerrainBlendParams {
    pub snow_start_height: f32,
    pub snow_blend_range: f32,
    pub slope_sharpness: f32,
}

pub struct TerrainMaterial {
    pub grass: GpuTexture,
    pub rock: GpuTexture,
    pub snow: GpuTexture,
    pub bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    uniform_buffer: wgpu::Buffer,
}

impl TerrainMaterial {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        grass: GpuTexture,
        rock: GpuTexture,
        snow: GpuTexture,
        blend: TerrainBlendParams,
    ) -> Self {
        let uniform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("terrain params uniform"),
                contents: bytemuck::bytes_of(&TerrainParamsUniform {
                    params: [
                        blend.snow_start_height,
                        blend.snow_blend_range,
                        blend.slope_sharpness,
                        0.0,
                    ],
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain material bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&grass.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&rock.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&snow.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&grass.sampler),
                },
            ],
        });

        Self {
            grass,
            rock,
            snow,
            bind_group,
            uniform_buffer,
        }
    }

    fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain material bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                Self::texture_entry(1),
                Self::texture_entry(2),
                Self::texture_entry(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}
