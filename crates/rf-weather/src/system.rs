use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rf_ecs::Events;
use rf_math::Vec3;
use rf_particles::ParticleSystem;

use crate::{WeatherKind, WeatherParams};

#[derive(Debug, Clone, Copy)]
pub enum WeatherEvent {
    Lightning { strength: f32 },
    Changed { from: WeatherKind, to: WeatherKind },
}

pub struct WeatherSystem {
    current: WeatherKind,
    target: WeatherKind,
    transition: f32,
    transition_duration: f32,
    hold_timer: f32,
    wind: Vec3,
    wind_target: Vec3,
    wind_retarget_timer: f32,
    lightning_cooldown: f32,
    rng: SmallRng,
    pub events: Events<WeatherEvent>,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self {
            current: WeatherKind::Clear,
            target: WeatherKind::Clear,
            transition: 1.0,
            transition_duration: 1.0,
            hold_timer: 30.0,
            wind: Vec3::ZERO,
            wind_target: Vec3::ZERO,
            wind_retarget_timer: 0.0,
            lightning_cooldown: 5.0,
            rng: SmallRng::from_entropy(),
            events: Events::new(),
        }
    }
}

impl WeatherSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> WeatherKind {
        self.current
    }

    pub fn update(&mut self, dt: f32) {
        self.events.update();
        self.update_wind(dt);
        self.update_transition(dt);
        self.update_lightning(dt);
    }

    fn update_wind(&mut self, dt: f32) {
        self.wind_retarget_timer -= dt;
        if self.wind_retarget_timer <= 0.0 {
            let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let strength = self.rng.gen_range(0.5..6.0);
            self.wind_target = Vec3::new(angle.cos(), 0.0, angle.sin()) * strength;
            self.wind_retarget_timer = self.rng.gen_range(6.0..20.0);
        }
        self.wind = self.wind.lerp(self.wind_target, (dt * 0.3).min(1.0));
    }

    fn update_transition(&mut self, dt: f32) {
        if self.transition < 1.0 {
            self.transition = (self.transition + dt / self.transition_duration).min(1.0);
            if self.transition >= 1.0 {
                self.current = self.target;
            }
            return;
        }

        self.hold_timer -= dt;
        if self.hold_timer > 0.0 {
            return;
        }

        let candidates = self.current.candidates();
        let next = candidates[self.rng.gen_range(0..candidates.len())];
        self.hold_timer = self.rng.gen_range(30.0..90.0);
        if next == self.current {
            return;
        }

        self.events.send(WeatherEvent::Changed {
            from: self.current,
            to: next,
        });
        self.target = next;
        self.transition = 0.0;
        self.transition_duration = self.rng.gen_range(10.0..25.0);
    }

    fn update_lightning(&mut self, dt: f32) {
        let storm_darkness = self.params().storm_darkness;
        if storm_darkness > 0.5 {
            self.lightning_cooldown -= dt;
            if self.lightning_cooldown <= 0.0 {
                let strength = self.rng.gen_range(0.6..1.0);
                self.events.send(WeatherEvent::Lightning { strength });
                self.lightning_cooldown = self.rng.gen_range(4.0..14.0);
            }
        } else {
            self.lightning_cooldown = self.lightning_cooldown.max(3.0);
        }
    }

    pub fn params(&self) -> WeatherParams {
        let (c0, f0, s0, r0, sn0, a0) = self.current.base_params();
        let (c1, f1, s1, r1, sn1, a1) = self.target.base_params();
        let t = self.transition;
        WeatherParams {
            cloud_coverage: rf_math::lerp(c0, c1, t),
            fog_density: rf_math::lerp(f0, f1, t),
            storm_darkness: rf_math::lerp(s0, s1, t),
            rain_intensity: rf_math::lerp(r0, r1, t),
            snow_intensity: rf_math::lerp(sn0, sn1, t),
            ambient_dimming: rf_math::lerp(a0, a1, t),
            wind: self.wind,
        }
    }

    pub fn drive_rain(&self, system: &mut ParticleSystem, base_rate: f32) {
        let params = self.params();
        system.config.rate = base_rate * params.rain_intensity;
        system.enabled = params.rain_intensity > 0.01;
        system.wind = params.wind;
    }

    pub fn drive_snow(&self, system: &mut ParticleSystem, base_rate: f32) {
        let params = self.params();
        system.config.rate = base_rate * params.snow_intensity;
        system.enabled = params.snow_intensity > 0.01;
        system.wind = params.wind;
    }
}
