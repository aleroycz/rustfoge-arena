use rand::Rng;
use rf_math::Vec3;
use rf_net::{EventKind, WeatherKind};

use super::{
    util::{event, random_arena_point},
    ArenaWorld,
};

pub struct PendingOrbitalStrike {
    pub target: Vec3,
    pub timer: f32,
}

pub enum RandomEvent {
    OrbitalStrike,
    ZeroGravity,
    GiantMode,
}

impl ArenaWorld {
    pub(crate) fn update_timers(&mut self, dt: f32) {
        if self.gravity_timer > 0.0 {
            self.gravity_timer -= dt;
            if self.gravity_timer <= 0.0 {
                self.gravity_scale = 1.0;
                self.pending_events
                    .push(event(EventKind::EventZeroGravityEnd, Vec3::ZERO, 0.0));
            }
        }
        if self.giant_mode_timer > 0.0 {
            self.giant_mode_timer -= dt;
            if self.giant_mode_timer <= 0.0 {
                self.pending_events
                    .push(event(EventKind::EventGiantModeEnd, Vec3::ZERO, 0.0));
            }
        }
        for player in self.players.values_mut() {
            player.fire_cooldown = (player.fire_cooldown - dt).max(0.0);
            player.speed_boost_timer = (player.speed_boost_timer - dt).max(0.0);
            player.damage_boost_timer = (player.damage_boost_timer - dt).max(0.0);
            player.just_died = false;
        }
    }

    pub(crate) fn update_weather_and_time(&mut self, dt: f32) {
        let day_cycle_seconds = self.config.day_cycle_seconds;
        let weather_transition_duration = self.config.weather_transition_duration;
        let weather_min_interval = self.config.weather_min_interval;
        let weather_max_interval = self.config.weather_max_interval;

        self.time_of_day =
            (self.time_of_day + dt * std::f32::consts::TAU / day_cycle_seconds) % std::f32::consts::TAU;

        if self.weather_transition < 1.0 {
            self.weather_transition =
                (self.weather_transition + dt / weather_transition_duration).min(1.0);
        }

        self.weather_timer -= dt;
        if self.weather_timer > 0.0 {
            return;
        }
        self.weather_timer = self
            .rng
            .gen_range(weather_min_interval..weather_max_interval);

        let choices = [
            WeatherKind::WeatherClear,
            WeatherKind::WeatherCloudy,
            WeatherKind::WeatherRain,
            WeatherKind::WeatherStorm,
        ];
        let mut next = choices[self.rng.gen_range(0..choices.len())];
        if next == self.weather {
            next = choices[self.rng.gen_range(0..choices.len())];
        }
        self.weather = next;
        self.weather_transition = 0.0;
        self.pending_events.push(event(
            EventKind::EventWeatherChanged,
            Vec3::ZERO,
            next as i32 as f32,
        ));
        log::info!("weather changing to {next:?}");
    }

    pub(crate) fn update_random_events(&mut self, dt: f32) {
        let orbital_strike_radius = self.config.orbital_strike_radius;
        let orbital_strike_damage = self.config.orbital_strike_damage;
        let orbital_strike_warning_time = self.config.orbital_strike_warning_time;
        let event_min_interval = self.config.event_min_interval;
        let event_max_interval = self.config.event_max_interval;
        let zero_gravity_duration = self.config.zero_gravity_duration;
        let giant_mode_duration = self.config.giant_mode_duration;

        if let Some(strike) = &mut self.pending_strike {
            strike.timer -= dt;
            if strike.timer <= 0.0 {
                let target = strike.target;
                self.pending_strike = None;
                for player in self.players.values_mut() {
                    if player.alive && player.position.distance(target) <= orbital_strike_radius {
                        player.health -= orbital_strike_damage;
                    }
                }
                self.pending_events.push(event(
                    EventKind::EventOrbitalStrikeImpact,
                    target,
                    orbital_strike_radius,
                ));
            }
            return;
        }

        if self.players.is_empty() {
            return;
        }

        self.event_timer -= dt;
        if self.event_timer > 0.0 {
            return;
        }
        self.event_timer = self.rng.gen_range(event_min_interval..event_max_interval);

        match [
            RandomEvent::OrbitalStrike,
            RandomEvent::ZeroGravity,
            RandomEvent::GiantMode,
        ][self.rng.gen_range(0..3)]
        {
            RandomEvent::OrbitalStrike => {
                let target = random_arena_point(&mut self.rng, &self.config, 0.0);
                self.pending_strike = Some(PendingOrbitalStrike {
                    target,
                    timer: orbital_strike_warning_time,
                });
                self.pending_events.push(event(
                    EventKind::EventOrbitalStrikeWarning,
                    target,
                    orbital_strike_warning_time,
                ));
                log::info!("random event: orbital strike incoming at {target:?}");
            }
            RandomEvent::ZeroGravity => {
                self.gravity_scale = 0.08;
                self.gravity_timer = zero_gravity_duration;
                self.pending_events.push(event(
                    EventKind::EventZeroGravityStart,
                    Vec3::ZERO,
                    zero_gravity_duration,
                ));
                log::info!("random event: zero gravity");
            }
            RandomEvent::GiantMode => {
                self.giant_mode_timer = giant_mode_duration;
                self.pending_events.push(event(
                    EventKind::EventGiantModeStart,
                    Vec3::ZERO,
                    giant_mode_duration,
                ));
                log::info!("random event: giant mode -- everyone's huge");
            }
        }
    }
}
