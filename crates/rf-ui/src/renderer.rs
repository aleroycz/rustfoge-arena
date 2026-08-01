use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{Canvas, FontAtlas};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct RectInstance {
    pos: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
    radius: f32,
    _pad: f32,
}

impl RectInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GlyphInstance {
    pos: [f32; 2],
    size: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
}

impl GlyphInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x4
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlyphInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ScreenUniform {
    size: [f32; 4],
}

pub struct UiRenderer {
    screen_buffer: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    rect_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    atlas_bind_group: wgpu::BindGroup,
    font: FontAtlas,
    rect_instances: Option<(wgpu::Buffer, u32)>,
    glyph_instances: Option<(wgpu::Buffer, u32)>,
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui screen bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ui screen uniform"),
            contents: bytemuck::bytes_of(&ScreenUniform {
                size: [1280.0, 720.0, 0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui screen bind group"),
            layout: &screen_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });

        let blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let color_target = wgpu::ColorTargetState {
            format: surface_format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        };

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui rect shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_rect.wgsl").into()),
        });
        let rect_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui rect pipeline layout"),
            bind_group_layouts: &[&screen_layout],
            push_constant_ranges: &[],
        });
        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui rect pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: "vs_main",
                buffers: &[RectInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: "fs_main",
                targets: &[Some(color_target.clone())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let font = FontAtlas::new(device, queue);
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui atlas bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui atlas bind group"),
            layout: &atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&font.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&font.sampler),
                },
            ],
        });

        let glyph_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui glyph shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui_glyph.wgsl").into()),
        });
        let glyph_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ui glyph pipeline layout"),
                bind_group_layouts: &[&screen_layout, &atlas_layout],
                push_constant_ranges: &[],
            });
        let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui glyph pipeline"),
            layout: Some(&glyph_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &glyph_shader,
                entry_point: "vs_main",
                buffers: &[GlyphInstance::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &glyph_shader,
                entry_point: "fs_main",
                targets: &[Some(color_target)],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            screen_buffer,
            screen_bind_group,
            rect_pipeline,
            glyph_pipeline,
            atlas_bind_group,
            font,
            rect_instances: None,
            glyph_instances: None,
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, canvas: &Canvas) {
        queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::bytes_of(&ScreenUniform {
                size: [canvas.width, canvas.height, 0.0, 0.0],
            }),
        );

        let rect_data: Vec<RectInstance> = canvas
            .rects
            .iter()
            .map(|r| RectInstance {
                pos: r.pos,
                size: r.size,
                color: r.color,
                radius: r.radius,
                _pad: 0.0,
            })
            .collect();
        self.rect_instances = upload_instances(device, &rect_data, "ui rect instances");

        let mut glyph_data = Vec::new();
        for command in &canvas.texts {
            let advance = Canvas::text_advance(command.scale);
            let glyph_height = command.scale;
            let glyph_width = command.scale * self.font.glyph_aspect;
            let mut cursor_x = command.pos[0];
            for ch in command.text.chars() {
                if ch != ' ' {
                    if let Some(uv) = self.font.glyph(ch) {
                        glyph_data.push(GlyphInstance {
                            pos: [cursor_x, command.pos[1]],
                            size: [glyph_width, glyph_height],
                            uv_min: uv.uv_min,
                            uv_max: uv.uv_max,
                            color: command.color,
                        });
                    }
                }
                cursor_x += advance;
            }
        }
        self.glyph_instances = upload_instances(device, &glyph_data, "ui glyph instances");
    }

    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if let Some((buffer, count)) = &self.rect_instances {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..4, 0..*count);
        }
        if let Some((buffer, count)) = &self.glyph_instances {
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_bind_group(0, &self.screen_bind_group, &[]);
            pass.set_bind_group(1, &self.atlas_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..4, 0..*count);
        }
    }
}

fn upload_instances<T: Pod>(
    device: &wgpu::Device,
    data: &[T],
    label: &str,
) -> Option<(wgpu::Buffer, u32)> {
    if data.is_empty() {
        return None;
    }
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Some((buffer, data.len() as u32))
}
