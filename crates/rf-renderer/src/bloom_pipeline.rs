pub struct BloomPipeline {
    pub texture_sampler_layout: wgpu::BindGroupLayout,
    pub params_layout: wgpu::BindGroupLayout,
    pub extract_blur_h_pipeline: wgpu::RenderPipeline,
    pub blur_v_pipeline: wgpu::RenderPipeline,
    pub composite_pipeline: wgpu::RenderPipeline,
}

impl BloomPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bloom.wgsl").into()),
        });

        let texture_sampler_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloom texture+sampler layout"),
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

        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom params layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let single_input_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom single-input pipeline layout"),
            bind_group_layouts: &[&texture_sampler_layout, &params_layout],
            push_constant_ranges: &[],
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom composite pipeline layout"),
            bind_group_layouts: &[
                &texture_sampler_layout,
                &params_layout,
                &texture_sampler_layout,
            ],
            push_constant_ranges: &[],
        });

        let color_target = |blend| {
            Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };

        let make_pipeline = |label: &str,
                             layout: &wgpu::PipelineLayout,
                             entry_point: &'static str,
                             blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point,
                    targets: &[color_target(blend)],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        let extract_blur_h_pipeline = make_pipeline(
            "bloom extract+blur-h pipeline",
            &single_input_layout,
            "fs_extract_blur_h",
            Some(wgpu::BlendState::REPLACE),
        );
        let blur_v_pipeline = make_pipeline(
            "bloom blur-v pipeline",
            &single_input_layout,
            "fs_blur_v",
            Some(wgpu::BlendState::REPLACE),
        );
        let composite_pipeline = make_pipeline(
            "bloom composite pipeline",
            &composite_layout,
            "fs_composite",
            Some(wgpu::BlendState::REPLACE),
        );

        Self {
            texture_sampler_layout,
            params_layout,
            extract_blur_h_pipeline,
            blur_v_pipeline,
            composite_pipeline,
        }
    }
}
