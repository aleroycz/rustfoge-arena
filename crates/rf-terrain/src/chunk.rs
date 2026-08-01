use nalgebra::DMatrix;
use rf_math::Vec3;
use rf_renderer::{Mesh, Vertex};

use crate::heightmap::HeightSampler;
use crate::manager::TerrainConfig;

pub struct ChunkGeometry {
    pub mesh: Mesh,
    pub heights: DMatrix<f32>,

    pub center: Vec3,
}

pub fn build_chunk(
    device: &wgpu::Device,
    sampler: &HeightSampler,
    coord: (i32, i32),
    config: &TerrainConfig,
) -> ChunkGeometry {
    let resolution = config.resolution as usize;
    let half = config.chunk_size * 0.5;
    let origin_x = coord.0 as f32 * config.chunk_size;
    let origin_z = coord.1 as f32 * config.chunk_size;

    let mut vertices = Vec::with_capacity(resolution * resolution);
    let mut heights = DMatrix::<f32>::zeros(resolution, resolution);

    for i in 0..resolution {
        let fx = i as f32 / (resolution - 1) as f32;
        let world_x = origin_x - half + fx * config.chunk_size;
        for j in 0..resolution {
            let fz = j as f32 / (resolution - 1) as f32;
            let world_z = origin_z - half + fz * config.chunk_size;

            let height = sampler.height(world_x, world_z);
            let normal = sampler.normal(world_x, world_z);
            heights[(i, j)] = height;

            vertices.push(Vertex {
                position: [world_x, height, world_z],
                normal: normal.to_array(),
                uv: [
                    world_x / config.texture_tiling,
                    world_z / config.texture_tiling,
                ],
            });
        }
    }

    let mut indices = Vec::with_capacity((resolution - 1) * (resolution - 1) * 6);
    for i in 0..resolution - 1 {
        for j in 0..resolution - 1 {
            let a = (i * resolution + j) as u32;
            let b = a + resolution as u32;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    ChunkGeometry {
        mesh: Mesh::new(device, &vertices, &indices),
        heights,
        center: Vec3::new(origin_x, 0.0, origin_z),
    }
}
