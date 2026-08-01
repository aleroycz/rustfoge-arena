use std::sync::Arc;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;

use rf_math::{Mat4, Transform, Vec3};
use rf_ui::{Canvas, UiRenderer};

use crate::{
    bloom_pipeline::BloomPipeline, context::GpuContext, particles::ParticlePipeline,
    pipeline::ForwardPipeline, shadow_pipeline::ShadowPipeline, sky::SkyPipeline,
    terrain_pipeline::TerrainPipeline, Camera, DirectionalLight, FogSettings, Material, Mesh,
    ParticleInstance, SkySettings, TerrainMaterial,
};

const SHADOW_MAP_SIZE: u32 = 2048;

const SHADOW_HALF_EXTENT: f32 = 50.0;
const SHADOW_DISTANCE: f32 = 80.0;

const BLOOM_THRESHOLD: f32 = 1.0;
const BLOOM_INTENSITY: f32 = 0.6;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ModelUniform {
    model: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ShadowMatrixUniform {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct BloomParamsUniform {
    params: [f32; 4],
}

pub struct RenderObject {
    pub mesh: Arc<Mesh>,
    pub material: Arc<Material>,
    pub visible: bool,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
}

impl RenderObject {
    pub fn set_transform(&self, queue: &wgpu::Queue, transform: &Transform) {
        write_model_uniform(queue, &self.model_buffer, transform);
    }
}

pub struct TerrainRenderObject {
    pub mesh: Arc<Mesh>,
    pub material: Arc<TerrainMaterial>,
    pub visible: bool,
    model_buffer: wgpu::Buffer,
    model_bind_group: wgpu::BindGroup,
}

impl TerrainRenderObject {
    pub fn set_transform(&self, queue: &wgpu::Queue, transform: &Transform) {
        write_model_uniform(queue, &self.model_buffer, transform);
    }
}

fn write_model_uniform(queue: &wgpu::Queue, buffer: &wgpu::Buffer, transform: &Transform) {
    let model = transform.matrix();

    let normal_matrix = model.inverse().transpose();
    let uniform = ModelUniform {
        model: model.to_cols_array_2d(),
        normal_matrix: normal_matrix.to_cols_array_2d(),
    };
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniform));
}

fn shadow_view_proj(camera_position: Vec3, light_direction: Vec3) -> Mat4 {
    let light_dir = if light_direction.length_squared() > 0.0001 {
        light_direction.normalize()
    } else {
        Vec3::new(0.0, -1.0, 0.0)
    };
    let up = if light_dir.y.abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let eye = camera_position - light_dir * SHADOW_DISTANCE;
    let view = Mat4::look_at_rh(eye, camera_position, up);
    let proj = Mat4::orthographic_rh(
        -SHADOW_HALF_EXTENT,
        SHADOW_HALF_EXTENT,
        -SHADOW_HALF_EXTENT,
        SHADOW_HALF_EXTENT,
        0.1,
        SHADOW_DISTANCE * 2.0,
    );
    proj * view
}

pub struct Renderer {
    context: GpuContext,
    pipeline: ForwardPipeline,
    terrain_pipeline: TerrainPipeline,
    particle_pipeline: ParticlePipeline,
    sky_pipeline: SkyPipeline,
    depth_texture: crate::GpuTexture,
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    camera_light_bind_group: wgpu::BindGroup,
    fog_buffer: wgpu::Buffer,
    fog_bind_group: wgpu::BindGroup,
    last_camera: Camera,
    last_light: DirectionalLight,

    shadow_pipeline: ShadowPipeline,
    shadow_map: crate::GpuTexture,
    shadow_light_buffer: wgpu::Buffer,

    shadow_matrix_bind_group: wgpu::BindGroup,

    shadow_sample_bind_group: wgpu::BindGroup,

    bloom_pipeline: BloomPipeline,
    scene_color: crate::GpuTexture,
    bloom_a: crate::GpuTexture,
    bloom_b: crate::GpuTexture,
    scene_color_bind_group: wgpu::BindGroup,
    bloom_a_bind_group: wgpu::BindGroup,
    bloom_b_bind_group: wgpu::BindGroup,
    bloom_h_params_buffer: wgpu::Buffer,
    bloom_h_params_bind_group: wgpu::BindGroup,
    bloom_v_params_buffer: wgpu::Buffer,
    bloom_v_params_bind_group: wgpu::BindGroup,
    bloom_composite_params_bind_group: wgpu::BindGroup,

    ui: UiRenderer,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let context = GpuContext::new(window)?;
        let pipeline = ForwardPipeline::new(&context.device, context.surface_format);
        let particle_pipeline = ParticlePipeline::new(
            &context.device,
            context.surface_format,
            &pipeline.camera_light_layout,
        );
        let sky_pipeline = SkyPipeline::new(&context.device, context.surface_format);
        let terrain_pipeline = TerrainPipeline::new(
            &context.device,
            context.surface_format,
            &pipeline.camera_light_layout,
            &pipeline.model_layout,
            &pipeline.fog_layout,
        );
        let depth_texture = crate::GpuTexture::depth_texture(
            &context.device,
            context.config.width,
            context.config.height,
        );

        let camera_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera uniform"),
                contents: bytemuck::bytes_of(&Camera::default().uniform()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let light_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("light uniform"),
                contents: bytemuck::bytes_of(&DirectionalLight::default().uniform()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_light_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("camera+light bind group"),
                    layout: &pipeline.camera_light_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: camera_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: light_buffer.as_entire_binding(),
                        },
                    ],
                });

        let fog_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("fog uniform"),
                contents: bytemuck::bytes_of(&FogSettings::default().uniform()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let fog_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fog bind group"),
                layout: &pipeline.fog_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: fog_buffer.as_entire_binding(),
                }],
            });

        let shadow_pipeline = ShadowPipeline::new(&context.device, &pipeline.model_layout);
        let shadow_map =
            crate::GpuTexture::depth_texture(&context.device, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE);
        let shadow_light_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("shadow light matrix uniform"),
                    contents: bytemuck::bytes_of(&ShadowMatrixUniform {
                        view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                    }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let shadow_matrix_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shadow matrix bind group"),
                    layout: &shadow_pipeline.light_matrix_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: shadow_light_buffer.as_entire_binding(),
                    }],
                });
        let shadow_sample_bind_group =
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shadow sample bind group"),
                    layout: &pipeline.shadow_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&shadow_map.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: shadow_light_buffer.as_entire_binding(),
                        },
                    ],
                });

        let bloom_pipeline = BloomPipeline::new(&context.device, context.surface_format);
        let (
            scene_color,
            bloom_a,
            bloom_b,
            scene_color_bind_group,
            bloom_a_bind_group,
            bloom_b_bind_group,
        ) = create_bloom_targets(
            &context.device,
            &bloom_pipeline,
            context.surface_format,
            context.config.width,
            context.config.height,
        );

        let bloom_h_params_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bloom h-blur params"),
                    contents: bytemuck::bytes_of(&BloomParamsUniform { params: [0.0; 4] }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let bloom_v_params_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bloom v-blur params"),
                    contents: bytemuck::bytes_of(&BloomParamsUniform { params: [0.0; 4] }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let bloom_composite_params_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bloom composite params"),
                    contents: bytemuck::bytes_of(&BloomParamsUniform {
                        params: [0.0, 0.0, 0.0, BLOOM_INTENSITY],
                    }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let make_params_bind_group = |buffer: &wgpu::Buffer, label: &str| {
            context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(label),
                    layout: &bloom_pipeline.params_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
        };
        let bloom_h_params_bind_group =
            make_params_bind_group(&bloom_h_params_buffer, "bloom h params bind group");
        let bloom_v_params_bind_group =
            make_params_bind_group(&bloom_v_params_buffer, "bloom v params bind group");
        let bloom_composite_params_bind_group = make_params_bind_group(
            &bloom_composite_params_buffer,
            "bloom composite params bind group",
        );

        let ui = UiRenderer::new(&context.device, &context.queue, context.surface_format);

        let renderer = Self {
            context,
            pipeline,
            terrain_pipeline,
            particle_pipeline,
            sky_pipeline,
            depth_texture,
            camera_buffer,
            light_buffer,
            camera_light_bind_group,
            fog_buffer,
            fog_bind_group,
            last_camera: Camera::default(),
            last_light: DirectionalLight::default(),
            shadow_pipeline,
            shadow_map,
            shadow_light_buffer,
            shadow_matrix_bind_group,
            shadow_sample_bind_group,
            bloom_pipeline,
            scene_color,
            bloom_a,
            bloom_b,
            scene_color_bind_group,
            bloom_a_bind_group,
            bloom_b_bind_group,
            bloom_h_params_buffer,
            bloom_h_params_bind_group,
            bloom_v_params_buffer,
            bloom_v_params_bind_group,
            bloom_composite_params_bind_group,
            ui,
        };
        renderer.write_bloom_texel_params();
        Ok(renderer)
    }

    fn write_bloom_texel_params(&self) {
        let full_w = self.context.config.width.max(1) as f32;
        let full_h = self.context.config.height.max(1) as f32;
        let half_w = (self.context.config.width.max(1) / 2).max(1) as f32;
        let half_h = (self.context.config.height.max(1) / 2).max(1) as f32;
        self.context.queue.write_buffer(
            &self.bloom_h_params_buffer,
            0,
            bytemuck::bytes_of(&BloomParamsUniform {
                params: [1.0 / full_w, 1.0 / full_h, BLOOM_THRESHOLD, 0.0],
            }),
        );
        self.context.queue.write_buffer(
            &self.bloom_v_params_buffer,
            0,
            bytemuck::bytes_of(&BloomParamsUniform {
                params: [1.0 / half_w, 1.0 / half_h, 0.0, 0.0],
            }),
        );
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.context.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.context.queue
    }

    pub fn material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.pipeline.material_layout
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.context.surface_format
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.context.resize(width, height);
        self.depth_texture = crate::GpuTexture::depth_texture(&self.context.device, width, height);

        let (
            scene_color,
            bloom_a,
            bloom_b,
            scene_color_bind_group,
            bloom_a_bind_group,
            bloom_b_bind_group,
        ) = create_bloom_targets(
            &self.context.device,
            &self.bloom_pipeline,
            self.context.surface_format,
            width,
            height,
        );
        self.scene_color = scene_color;
        self.bloom_a = bloom_a;
        self.bloom_b = bloom_b;
        self.scene_color_bind_group = scene_color_bind_group;
        self.bloom_a_bind_group = bloom_a_bind_group;
        self.bloom_b_bind_group = bloom_b_bind_group;
        self.write_bloom_texel_params();
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.context.config.width as f32 / self.context.config.height.max(1) as f32
    }

    fn create_model_binding(&self) -> (wgpu::Buffer, wgpu::BindGroup) {
        let model_buffer =
            self.context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("model uniform"),
                    contents: bytemuck::bytes_of(&ModelUniform {
                        model: rf_math::Mat4::IDENTITY.to_cols_array_2d(),
                        normal_matrix: rf_math::Mat4::IDENTITY.to_cols_array_2d(),
                    }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let model_bind_group = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("model bind group"),
                layout: &self.pipeline.model_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_buffer.as_entire_binding(),
                }],
            });
        (model_buffer, model_bind_group)
    }

    pub fn create_render_object(&self, mesh: Arc<Mesh>, material: Arc<Material>) -> RenderObject {
        let (model_buffer, model_bind_group) = self.create_model_binding();
        RenderObject {
            mesh,
            material,
            visible: true,
            model_buffer,
            model_bind_group,
        }
    }

    pub fn create_terrain_render_object(
        &self,
        mesh: Arc<Mesh>,
        material: Arc<TerrainMaterial>,
    ) -> TerrainRenderObject {
        let (model_buffer, model_bind_group) = self.create_model_binding();
        TerrainRenderObject {
            mesh,
            material,
            visible: true,
            model_buffer,
            model_bind_group,
        }
    }

    pub fn terrain_material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.terrain_pipeline.material_layout
    }

    pub fn set_camera(&mut self, camera: &Camera) {
        self.last_camera = *camera;
        self.context.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&camera.uniform()),
        );
    }

    pub fn set_light(&mut self, light: &DirectionalLight) {
        self.last_light = *light;
        self.context.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::bytes_of(&light.uniform()),
        );
    }

    pub fn set_sky(
        &self,
        camera: &Camera,
        light: &DirectionalLight,
        settings: &SkySettings,
        elapsed_seconds: f32,
    ) {
        self.sky_pipeline.write(
            &self.context.queue,
            camera,
            light,
            settings,
            elapsed_seconds,
        );
    }

    pub fn set_fog(&self, fog: &FogSettings) {
        self.context
            .queue
            .write_buffer(&self.fog_buffer, 0, bytemuck::bytes_of(&fog.uniform()));
    }

    pub fn prepare_ui(&mut self, canvas: &Canvas) {
        self.ui
            .prepare(&self.context.device, &self.context.queue, canvas);
    }

    pub fn render<'a>(
        &mut self,
        objects: impl IntoIterator<Item = &'a RenderObject>,
        terrain_objects: impl IntoIterator<Item = &'a TerrainRenderObject>,
        particle_batches: &[Vec<ParticleInstance>],
    ) -> Result<(), wgpu::SurfaceError> {
        let objects: Vec<&RenderObject> = objects.into_iter().collect();
        let terrain_objects: Vec<&TerrainRenderObject> = terrain_objects.into_iter().collect();

        let output = self.context.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("frame encoder"),
                });

        let shadow_matrix = shadow_view_proj(self.last_camera.position, self.last_light.direction);
        self.context.queue.write_buffer(
            &self.shadow_light_buffer,
            0,
            bytemuck::bytes_of(&ShadowMatrixUniform {
                view_proj: shadow_matrix.to_cols_array_2d(),
            }),
        );

        let particle_instance_buffers: Vec<wgpu::Buffer> = particle_batches
            .iter()
            .filter(|batch| !batch.is_empty())
            .map(|batch| {
                self.context
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("particle instance buffer"),
                        contents: bytemuck::cast_slice(batch),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            })
            .collect();
        let particle_counts: Vec<u32> = particle_batches
            .iter()
            .filter(|batch| !batch.is_empty())
            .map(|batch| batch.len() as u32)
            .collect();

        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            shadow_pass.set_pipeline(&self.shadow_pipeline.pipeline);
            shadow_pass.set_bind_group(0, &self.shadow_matrix_bind_group, &[]);
            for object in objects.iter().filter(|o| o.visible) {
                shadow_pass.set_bind_group(1, &object.model_bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, object.mesh.vertex_buffer.slice(..));
                shadow_pass.set_index_buffer(
                    object.mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                shadow_pass.draw_indexed(0..object.mesh.index_count, 0, 0..1);
            }
            for chunk in terrain_objects.iter().filter(|o| o.visible) {
                shadow_pass.set_bind_group(1, &chunk.model_bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, chunk.mesh.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(chunk.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..chunk.mesh.index_count, 0, 0..1);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("forward pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_color.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.07,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.sky_pipeline.draw(&mut pass);

            pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
            pass.set_bind_group(3, &self.fog_bind_group, &[]);
            pass.set_bind_group(4, &self.shadow_sample_bind_group, &[]);

            pass.set_pipeline(&self.terrain_pipeline.pipeline);
            for chunk in terrain_objects.iter().filter(|o| o.visible) {
                pass.set_bind_group(1, &chunk.material.bind_group, &[]);
                pass.set_bind_group(2, &chunk.model_bind_group, &[]);
                pass.set_vertex_buffer(0, chunk.mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(chunk.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..chunk.mesh.index_count, 0, 0..1);
            }

            pass.set_pipeline(&self.pipeline.pipeline);
            for object in objects.iter().filter(|o| o.visible) {
                pass.set_bind_group(1, &object.material.bind_group, &[]);
                pass.set_bind_group(2, &object.model_bind_group, &[]);
                pass.set_vertex_buffer(0, object.mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(
                    object.mesh.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..object.mesh.index_count, 0, 0..1);
            }

            pass.set_bind_group(0, &self.camera_light_bind_group, &[]);
            for (buffer, &count) in particle_instance_buffers.iter().zip(&particle_counts) {
                self.particle_pipeline.draw(&mut pass, buffer, count);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom extract+blur-h pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_a.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.bloom_pipeline.extract_blur_h_pipeline);
            pass.set_bind_group(0, &self.scene_color_bind_group, &[]);
            pass.set_bind_group(1, &self.bloom_h_params_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom blur-v pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_b.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.bloom_pipeline.blur_v_pipeline);
            pass.set_bind_group(0, &self.bloom_a_bind_group, &[]);
            pass.set_bind_group(1, &self.bloom_v_params_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom composite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.bloom_pipeline.composite_pipeline);
            pass.set_bind_group(0, &self.scene_color_bind_group, &[]);
            pass.set_bind_group(1, &self.bloom_composite_params_bind_group, &[]);
            pass.set_bind_group(2, &self.bloom_b_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        {
            let mut ui_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.ui.render(&mut ui_render_pass);
        }

        self.context.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }
}

#[allow(clippy::type_complexity)]
fn create_bloom_targets(
    device: &wgpu::Device,
    bloom_pipeline: &BloomPipeline,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (
    crate::GpuTexture,
    crate::GpuTexture,
    crate::GpuTexture,
    wgpu::BindGroup,
    wgpu::BindGroup,
    wgpu::BindGroup,
) {
    let half_w = (width.max(1) / 2).max(1);
    let half_h = (height.max(1) / 2).max(1);

    let scene_color =
        crate::GpuTexture::render_target(device, width, height, format, "scene color");
    let bloom_a = crate::GpuTexture::render_target(device, half_w, half_h, format, "bloom a");
    let bloom_b = crate::GpuTexture::render_target(device, half_w, half_h, format, "bloom b");

    let make_bind_group = |texture: &crate::GpuTexture, label: &str| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &bloom_pipeline.texture_sampler_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    };
    let scene_color_bind_group = make_bind_group(&scene_color, "scene color bind group");
    let bloom_a_bind_group = make_bind_group(&bloom_a, "bloom a bind group");
    let bloom_b_bind_group = make_bind_group(&bloom_b, "bloom b bind group");

    (
        scene_color,
        bloom_a,
        bloom_b,
        scene_color_bind_group,
        bloom_a_bind_group,
        bloom_b_bind_group,
    )
}
