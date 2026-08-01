use wgpu::util::DeviceExt;

use crate::Vertex;
use rf_math::Vec3;

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertex buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh index buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    pub fn cuboid(device: &wgpu::Device, size: Vec3) -> Self {
        let h = size * 0.5;
        let faces: [(Vec3, Vec3, Vec3); 6] = [
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
            (Vec3::X, Vec3::NEG_Z, Vec3::Y),
            (Vec3::NEG_X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::X, Vec3::NEG_Z),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (normal, right, up) in faces {
            let center = normal * h;
            let base = vertices.len() as u32;
            let corners = [
                center - right * h.x - up * h.y,
                center + right * h.x - up * h.y,
                center + right * h.x + up * h.y,
                center - right * h.x + up * h.y,
            ];
            let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            for (corner, uv) in corners.into_iter().zip(uvs) {
                vertices.push(Vertex {
                    position: corner.to_array(),
                    normal: normal.to_array(),
                    uv,
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Self::new(device, &vertices, &indices)
    }

    pub fn plane(device: &wgpu::Device, size: f32, tiling: f32) -> Self {
        let h = size * 0.5;
        let tiles = size / tiling.max(0.01);
        let vertices = [
            Vertex {
                position: [-h, 0.0, h],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, tiles],
            },
            Vertex {
                position: [h, 0.0, h],
                normal: [0.0, 1.0, 0.0],
                uv: [tiles, tiles],
            },
            Vertex {
                position: [h, 0.0, -h],
                normal: [0.0, 1.0, 0.0],
                uv: [tiles, 0.0],
            },
            Vertex {
                position: [-h, 0.0, -h],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
        ];
        let indices = [0u32, 1, 2, 0, 2, 3];
        Self::new(device, &vertices, &indices)
    }

    pub fn uv_sphere(device: &wgpu::Device, radius: f32, stacks: u32, slices: u32) -> Self {
        let mut vertices = Vec::new();
        for i in 0..=stacks {
            let v = i as f32 / stacks as f32;
            let phi = v * std::f32::consts::PI;
            for j in 0..=slices {
                let u = j as f32 / slices as f32;
                let theta = u * std::f32::consts::TAU;
                let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                vertices.push(Vertex {
                    position: (normal * radius).to_array(),
                    normal: normal.to_array(),
                    uv: [u, v],
                });
            }
        }

        let mut indices = Vec::new();
        let row = slices + 1;
        for i in 0..stacks {
            for j in 0..slices {
                let a = i * row + j;
                let b = a + row;
                indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }

        Self::new(device, &vertices, &indices)
    }
}
