use std::collections::HashMap;
use std::sync::Arc;

use hecs::Entity;
use rf_ecs::World;
use rf_math::{Transform, Vec3};
use rf_physics::{PhysicsWorld, RigidBodyHandle};
use rf_renderer::{GpuTexture, Renderer, TerrainMaterial};

use crate::chunk::build_chunk;
use crate::heightmap::HeightSampler;

#[derive(Debug, Clone, Copy)]
pub struct TerrainConfig {
    pub chunk_size: f32,

    pub resolution: u32,
    pub height_scale: f32,
    pub noise_frequency: f64,
    pub noise_octaves: usize,
    pub seed: u32,

    pub view_distance_chunks: i32,

    pub texture_tiling: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64.0,
            resolution: 33,
            height_scale: 14.0,
            noise_frequency: 0.008,
            noise_octaves: 5,
            seed: 1,
            view_distance_chunks: 3,
            texture_tiling: 8.0,
        }
    }
}

fn mottled_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    base: [u8; 3],
    variation: u8,
) -> GpuTexture {
    const SIZE: u32 = 64;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let n = hash2(x, y);
            let offset = (n as i32 % (2 * variation as i32 + 1)) - variation as i32;
            let idx = ((y * SIZE + x) * 4) as usize;
            for c in 0..3 {
                pixels[idx + c] = (base[c] as i32 + offset).clamp(0, 255) as u8;
            }
            pixels[idx + 3] = 255;
        }
    }
    GpuTexture::from_rgba8(
        device,
        queue,
        &pixels,
        SIZE,
        SIZE,
        "terrain mottled texture",
    )
}

fn hash2(x: u32, y: u32) -> u32 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

struct LoadedChunk {
    entity: Entity,
    body: RigidBodyHandle,
}

pub struct TerrainManager {
    config: TerrainConfig,
    sampler: HeightSampler,
    material: Arc<TerrainMaterial>,
    loaded: HashMap<(i32, i32), LoadedChunk>,
}

impl TerrainManager {
    pub fn new(renderer: &Renderer, config: TerrainConfig) -> Self {
        let device = renderer.device();
        let queue = renderer.queue();
        let grass = mottled_texture(device, queue, [70, 110, 45], 24);
        let rock = mottled_texture(device, queue, [110, 105, 100], 20);
        let snow = mottled_texture(device, queue, [225, 230, 235], 10);
        let material = Arc::new(TerrainMaterial::new(
            device,
            renderer.terrain_material_bind_group_layout(),
            grass,
            rock,
            snow,
            rf_renderer::TerrainBlendParams {
                snow_start_height: config.height_scale * 0.55,
                snow_blend_range: config.height_scale * 0.2,
                slope_sharpness: 4.0,
            },
        ));
        let sampler = HeightSampler::new(
            config.seed,
            config.noise_frequency,
            config.height_scale,
            config.noise_octaves,
        );
        Self {
            config,
            sampler,
            material,
            loaded: HashMap::new(),
        }
    }

    pub fn height_at(&self, world_x: f32, world_z: f32) -> f32 {
        self.sampler.height(world_x, world_z)
    }

    pub fn update(
        &mut self,
        renderer: &Renderer,
        world: &mut World,
        physics: &mut PhysicsWorld,
        focus: Vec3,
    ) {
        let center_x = (focus.x / self.config.chunk_size).floor() as i32;
        let center_z = (focus.z / self.config.chunk_size).floor() as i32;
        let radius = self.config.view_distance_chunks;

        let mut desired = std::collections::HashSet::new();
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                desired.insert((center_x + dx, center_z + dz));
            }
        }

        let to_remove: Vec<(i32, i32)> = self
            .loaded
            .keys()
            .filter(|coord| !desired.contains(coord))
            .copied()
            .collect();
        for coord in to_remove {
            if let Some(chunk) = self.loaded.remove(&coord) {
                let _ = world.despawn(chunk.entity);
                physics.remove_body(chunk.body);
            }
        }

        for coord in desired {
            if self.loaded.contains_key(&coord) {
                continue;
            }
            let geometry = build_chunk(renderer.device(), &self.sampler, coord, &self.config);
            let render_object = renderer
                .create_terrain_render_object(Arc::new(geometry.mesh), self.material.clone());
            render_object.set_transform(renderer.queue(), &Transform::IDENTITY);

            let entity = world.spawn((Transform::IDENTITY, render_object));

            let (body, _collider) = physics.add_heightfield_body(
                geometry.center,
                &geometry.heights,
                Vec3::new(self.config.chunk_size, 1.0, self.config.chunk_size),
            );

            self.loaded.insert(coord, LoadedChunk { entity, body });
        }
    }

    pub fn loaded_chunk_count(&self) -> usize {
        self.loaded.len()
    }
}
