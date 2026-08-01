use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use rf_math::Vec3;

pub struct HeightSampler {
    fbm: Fbm<Perlin>,
    frequency: f64,
    amplitude: f32,
}

impl HeightSampler {
    pub fn new(seed: u32, frequency: f64, amplitude: f32, octaves: usize) -> Self {
        let fbm = Fbm::<Perlin>::new(seed)
            .set_octaves(octaves)
            .set_persistence(0.5)
            .set_lacunarity(2.0);
        Self {
            fbm,
            frequency,
            amplitude,
        }
    }

    pub fn height(&self, world_x: f32, world_z: f32) -> f32 {
        let n = self.fbm.get([
            world_x as f64 * self.frequency,
            world_z as f64 * self.frequency,
        ]);
        n as f32 * self.amplitude
    }

    pub fn normal(&self, world_x: f32, world_z: f32) -> Vec3 {
        let eps = 0.75;
        let left = self.height(world_x - eps, world_z);
        let right = self.height(world_x + eps, world_z);
        let down = self.height(world_x, world_z - eps);
        let up = self.height(world_x, world_z + eps);
        Vec3::new(left - right, 2.0 * eps, down - up).normalize_or_zero()
    }
}
