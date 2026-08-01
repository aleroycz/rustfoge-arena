use rand::Rng;
use rf_math::Vec3;
use rf_net::{EventKind, ItemKind, MatchPhase, WeatherKind};

use super::{util::event, util::random_arena_point, ArenaWorld};

impl ArenaWorld {
    pub(crate) fn tick_lobby(&mut self) {
        for player in self.players.values_mut() {
            if player.last_input.toggle_ready {
                player.last_input.toggle_ready = false;
                player.ready = !player.ready;
            }
        }

        if !self.players.is_empty() && self.players.values().all(|p| p.ready) {
            log::info!("all players ready -- starting match countdown");
            self.phase = MatchPhase::PhaseStarting;
            self.phase_timer = self.config.starting_countdown;
            self.pending_events.push(event(
                EventKind::EventMatchStarting,
                Vec3::ZERO,
                self.config.starting_countdown,
            ));
        }
    }

    pub(crate) fn tick_starting(&mut self, dt: f32) {
        self.phase_timer -= dt;
        if self.phase_timer > 0.0 {
            return;
        }

        self.generate_pois();
        self.spawn_players_at_pois();
        self.spawn_powerups();
        self.spawn_items();

        self.phase = MatchPhase::PhasePlaying;
        self.phase_timer = self.config.match_duration;
        self.event_timer = self
            .rng
            .gen_range(self.config.event_min_interval..self.config.event_max_interval);
        self.pending_events.push(event(
            EventKind::EventMatchBegin,
            Vec3::ZERO,
            self.config.match_duration,
        ));
        log::info!(
            "match started with {} POIs, {} players",
            self.pois.len(),
            self.players.len()
        );
    }

    pub(crate) fn tick_playing(&mut self, dt: f32) {
        self.phase_timer -= dt;

        self.update_timers(dt);
        self.update_weather_and_time(dt);
        self.update_random_events(dt);
        self.update_movement(dt);
        self.resolve_firing(dt);
        self.resolve_abilities(dt);
        self.resolve_powerups();
        self.resolve_items(dt);
        self.resolve_respawns(dt);

        if self.phase_timer <= 0.0 {
            self.end_match();
        }
    }

    pub(crate) fn tick_ending(&mut self, dt: f32) {
        self.phase_timer -= dt;
        if self.phase_timer <= 0.0 {
            self.reset_to_lobby();
        }
    }

    fn end_match(&mut self) {
        let winner = self.players.iter().max_by(|(a_id, a), (b_id, b)| {
            a.kills
                .cmp(&b.kills)
                .then(b.deaths.cmp(&a.deaths))
                .then(b_id.cmp(a_id))
        });
        if let Some((&id, player)) = winner {
            self.winner_id = id;
            self.winner_name = player.name.clone();
        } else {
            self.winner_id = 0;
            self.winner_name = "NOBODY".to_string();
        }
        self.phase = MatchPhase::PhaseEnding;
        self.phase_timer = self.config.ending_duration;
        self.pending_events
            .push(event(EventKind::EventMatchEnd, Vec3::ZERO, 0.0));
        log::info!(
            "match ended -- winner: {} ({} id={})",
            self.winner_name,
            self.winner_id,
            self.winner_id
        );
    }

    pub(crate) fn reset_to_lobby(&mut self) {
        self.phase = MatchPhase::PhaseLobby;
        self.phase_timer = 0.0;
        self.pois.clear();
        self.powerups.clear();
        self.items.clear();
        self.gravity_scale = 1.0;
        self.gravity_timer = 0.0;
        self.giant_mode_timer = 0.0;
        self.pending_strike = None;
        self.time_of_day = std::f32::consts::FRAC_PI_2;
        self.weather = WeatherKind::WeatherClear;
        self.weather_transition = 1.0;
        self.weather_timer = self
            .rng
            .gen_range(self.config.weather_min_interval..self.config.weather_max_interval);
        let max_health = self.config.max_health;
        let config = self.config.clone();
        let rng = &mut self.rng;
        for player in self.players.values_mut() {
            player.ready = false;
            player.kills = 0;
            player.deaths = 0;
            player.health = max_health;
            player.alive = true;
            player.held_item = ItemKind::ItemNone;
            player.position = random_arena_point(rng, &config, 0.5);
            player.dash_cooldown = 0.0;
            player.superspeed_cooldown = 0.0;
            player.swap_cooldown = 0.0;
            player.ultimate_cooldown = 0.0;
            player.superspeed_ability_timer = 0.0;
        }
        log::info!("returned to lobby -- ready up for another match");
    }

    pub(crate) fn resolve_respawns(&mut self, dt: f32) {
        let max_health = self.config.max_health;
        let config = self.config.clone();
        let rng = &mut self.rng;
        for player in self.players.values_mut() {
            if player.alive {
                continue;
            }
            player.respawn_timer -= dt;
            if player.respawn_timer <= 0.0 {
                player.alive = true;
                player.health = max_health;
                player.position = random_arena_point(rng, &config, 0.5);
                player.velocity = Vec3::ZERO;
            }
        }
    }
}
