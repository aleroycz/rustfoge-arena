use rf_math::Vec3;

use crate::{EmitterConfig, ParticleSystem};

pub fn rain(origin: Vec3) -> ParticleSystem {
    ParticleSystem::new(
        EmitterConfig {
            rate: 400.0,
            lifetime: (1.5, 2.5),
            initial_speed: (14.0, 18.0),
            direction: Vec3::NEG_Y,
            direction_spread: 0.05,
            size: (0.03, 0.05),
            color_start: [0.75, 0.8, 0.9, 0.6],
            color_end: [0.75, 0.8, 0.9, 0.15],
            gravity_scale: 0.4,
            drag: 0.02,
            wind_influence: 1.5,
        },
        origin,
    )
}

pub fn snow(origin: Vec3) -> ParticleSystem {
    ParticleSystem::new(
        EmitterConfig {
            rate: 120.0,
            lifetime: (4.0, 7.0),
            initial_speed: (0.5, 1.2),
            direction: Vec3::NEG_Y,
            direction_spread: 0.4,
            size: (0.06, 0.14),
            color_start: [1.0, 1.0, 1.0, 0.9],
            color_end: [1.0, 1.0, 1.0, 0.5],
            gravity_scale: 0.05,
            drag: 0.3,
            wind_influence: 2.5,
        },
        origin,
    )
}

pub fn dust_burst(origin: Vec3) -> ParticleSystem {
    ParticleSystem::new(
        EmitterConfig {
            rate: 0.0,
            lifetime: (0.3, 0.6),
            initial_speed: (1.0, 3.0),
            direction: Vec3::Y,
            direction_spread: std::f32::consts::FRAC_PI_3,
            size: (0.1, 0.25),
            color_start: [0.55, 0.5, 0.42, 0.5],
            color_end: [0.55, 0.5, 0.42, 0.0],
            gravity_scale: 0.6,
            drag: 0.5,
            wind_influence: 0.3,
        },
        origin,
    )
}

pub fn sparks(origin: Vec3) -> ParticleSystem {
    ParticleSystem::new(
        EmitterConfig {
            rate: 0.0,
            lifetime: (0.2, 0.5),
            initial_speed: (3.0, 7.0),
            direction: Vec3::Y,
            direction_spread: std::f32::consts::FRAC_PI_2,
            size: (0.02, 0.04),
            color_start: [1.0, 0.85, 0.4, 1.0],
            color_end: [1.0, 0.3, 0.1, 0.0],
            gravity_scale: 2.0,
            drag: 0.1,
            wind_influence: 0.0,
        },
        origin,
    )
}
