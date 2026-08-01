use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rf_math::Vec3;

use crate::{EmitterConfig, Particle};

pub struct ParticleSystem {
    pub config: EmitterConfig,
    pub origin: Vec3,
    pub wind: Vec3,
    pub enabled: bool,
    particles: Vec<Particle>,
    spawn_accumulator: f32,
    rng: SmallRng,
}

impl ParticleSystem {
    pub fn new(config: EmitterConfig, origin: Vec3) -> Self {
        Self {
            config,
            origin,
            wind: Vec3::ZERO,
            enabled: true,
            particles: Vec::new(),
            spawn_accumulator: 0.0,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn update(&mut self, dt: f32, gravity: Vec3) {
        if self.enabled {
            self.spawn_accumulator += self.config.rate * dt;
            while self.spawn_accumulator >= 1.0 {
                self.spawn_accumulator -= 1.0;
                self.spawn_one();
            }
        }

        let drag_factor = (1.0 - self.config.drag * dt).clamp(0.0, 1.0);
        for particle in &mut self.particles {
            let acceleration =
                gravity * self.config.gravity_scale + self.wind * self.config.wind_influence;
            particle.velocity += acceleration * dt;
            particle.velocity *= drag_factor;
            particle.position += particle.velocity * dt;
            particle.age += dt;
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    pub fn burst(&mut self, count: u32) {
        for _ in 0..count {
            self.spawn_one();
        }
    }

    fn spawn_one(&mut self) {
        let cfg = &self.config;
        let direction = random_cone_direction(&mut self.rng, cfg.direction, cfg.direction_spread);
        let speed = self
            .rng
            .gen_range(cfg.initial_speed.0..=cfg.initial_speed.1);
        let lifetime = self.rng.gen_range(cfg.lifetime.0..=cfg.lifetime.1);
        let size = self.rng.gen_range(cfg.size.0..=cfg.size.1);

        self.particles.push(Particle {
            position: self.origin,
            velocity: direction * speed,
            age: 0.0,
            lifetime,
            size,
            color_start: cfg.color_start,
        });
    }

    pub fn current_color(&self, particle: &Particle) -> [f32; 4] {
        let t = particle.life_fraction();
        let mut out = [0.0; 4];
        for (i, out_channel) in out.iter_mut().enumerate() {
            *out_channel =
                particle.color_start[i] + (self.config.color_end[i] - particle.color_start[i]) * t;
        }
        out
    }
}

fn random_cone_direction(rng: &mut SmallRng, direction: Vec3, spread: f32) -> Vec3 {
    if spread <= 0.0 {
        return direction.normalize_or_zero();
    }
    let base = direction.normalize_or_zero();
    let (tangent, bitangent) = orthonormal_basis(base);

    let theta = rng.gen_range(0.0..spread);
    let phi = rng.gen_range(0.0..std::f32::consts::TAU);
    let offset = (tangent * phi.cos() + bitangent * phi.sin()) * theta.sin();
    (base * theta.cos() + offset).normalize_or_zero()
}

fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    let helper = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let tangent = n.cross(helper).normalize_or_zero();
    let bitangent = n.cross(tangent);
    (tangent, bitangent)
}
