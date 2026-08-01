use bytemuck::{Pod, Zeroable};

use crate::GpuTexture;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],

    pub metallic_roughness: [f32; 4],

    pub emissive: [f32; 4],
}

pub struct Material {
    pub base_color_factor: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub texture: GpuTexture,
    pub bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        texture: GpuTexture,
        base_color_factor: [f32; 4],
        metallic: f32,
        roughness: f32,
    ) -> Self {
        let uniform = MaterialUniform {
            base_color: base_color_factor,
            metallic_roughness: [metallic, roughness, 0.0, 0.0],
            emissive: [0.0, 0.0, 0.0, 0.0],
        };
        let uniform_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("material uniform"),
                contents: bytemuck::bytes_of(&uniform),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Self {
            base_color_factor,
            metallic,
            roughness,
            emissive: [0.0, 0.0, 0.0],
            texture,
            bind_group,
            uniform_buffer,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue) {
        let uniform = MaterialUniform {
            base_color: self.base_color_factor,
            metallic_roughness: [self.metallic, self.roughness, 0.0, 0.0],
            emissive: [self.emissive[0], self.emissive[1], self.emissive[2], 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn set_emissive(&mut self, queue: &wgpu::Queue, color: [f32; 3]) {
        self.emissive = color;
        self.write(queue);
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material bind group layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}
