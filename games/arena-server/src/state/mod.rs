mod abilities;
mod combat;
mod events;
mod item;
mod lifecycle;
mod movement;
mod player;
mod poi;
mod powerup;
mod util;

use std::collections::HashMap;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rf_net::{
    ItemState, MatchPhase, PlayerInput, PlayerState, PointOfInterest, PowerupState, WeatherKind,
    WorldSnapshot,
};

use crate::config::ServerConfig;

use events::PendingOrbitalStrike;
use item::ServerItem;
use player::ServerPlayer;
use poi::Poi;
use powerup::ServerPowerup;

use util::to_pb;

pub struct ArenaWorld {
    pub(crate) config: ServerConfig,

    pub players: HashMap<u32, ServerPlayer>,
    pub(crate) powerups: HashMap<u32, ServerPowerup>,
    pub(crate) items: HashMap<u32, ServerItem>,
    pub(crate) pois: Vec<Poi>,

    pub(crate) session_tokens: HashMap<u64, u32>,
    pub(crate) next_player_id: u32,
    pub(crate) next_item_id: u32,
    pub(crate) tick: u64,
    pub(crate) phase: MatchPhase,
    pub(crate) phase_timer: f32,
    pub(crate) winner_id: u32,
    pub(crate) winner_name: String,
    pub(crate) gravity_scale: f32,
    pub(crate) gravity_timer: f32,
    pub(crate) giant_mode_timer: f32,
    pub(crate) event_timer: f32,
    pub(crate) pending_events: Vec<rf_net::GameEvent>,
    pub(crate) pending_strike: Option<PendingOrbitalStrike>,
    pub(crate) time_of_day: f32,
    pub(crate) weather: WeatherKind,
    pub(crate) weather_timer: f32,
    pub(crate) weather_transition: f32,
    pub(crate) rng: SmallRng,
}

impl Default for ArenaWorld {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}

impl ArenaWorld {
    pub fn new(config: ServerConfig) -> Self {
        let mut world = Self {
            config,
            players: HashMap::new(),
            powerups: HashMap::new(),
            items: HashMap::new(),
            pois: Vec::new(),
            session_tokens: HashMap::new(),
            next_player_id: 1,
            next_item_id: 0,
            tick: 0,
            phase: MatchPhase::PhaseLobby,
            phase_timer: 0.0,
            winner_id: 0,
            winner_name: String::new(),
            gravity_scale: 1.0,
            gravity_timer: 0.0,
            giant_mode_timer: 0.0,
            event_timer: 0.0,
            pending_events: Vec::new(),
            pending_strike: None,
            time_of_day: std::f32::consts::FRAC_PI_2,
            weather: WeatherKind::WeatherClear,
            weather_timer: 0.0,
            weather_transition: 1.0,
            rng: SmallRng::from_entropy(),
        };
        world.event_timer = world
            .rng
            .gen_range(world.config.event_min_interval..world.config.event_max_interval);
        world.weather_timer = world
            .rng
            .gen_range(world.config.weather_min_interval..world.config.weather_max_interval);
        world
    }

    pub fn join(&mut self, name: String) -> (u32, u64) {
        let id = self.next_player_id;
        self.next_player_id += 1;
        let spawn = util::random_arena_point(&mut self.rng, &self.config, 0.5);
        self.players
            .insert(id, ServerPlayer::new(name, spawn, &self.config));

        let token = loop {
            let candidate = self.rng.gen::<u64>();
            if candidate != 0 && !self.session_tokens.contains_key(&candidate) {
                break candidate;
            }
        };
        self.session_tokens.insert(token, id);

        log::info!("player {id} joined the lobby");
        (id, token)
    }

    pub fn leave(&mut self, id: u32) {
        self.players.remove(&id);
        self.session_tokens.retain(|_, &mut owner| owner != id);
        for item in self.items.values_mut() {
            if item.held_by == Some(id) {
                item.held_by = None;
                item.active = true;
            }
        }
        log::info!("player {id} left the arena");

        if self.players.is_empty() && self.phase != MatchPhase::PhaseLobby {
            log::info!("all players disconnected -- resetting to lobby");
            self.reset_to_lobby();
        }
    }

    pub fn apply_input(&mut self, input: PlayerInput) -> Option<u32> {
        let &player_id = self.session_tokens.get(&input.session_token)?;
        if let Some(player) = self.players.get_mut(&player_id) {
            let merged = PlayerInput {
                jump: player.last_input.jump || input.jump,
                toggle_ready: player.last_input.toggle_ready || input.toggle_ready,
                use_item: player.last_input.use_item || input.use_item,
                ability_dash: player.last_input.ability_dash || input.ability_dash,
                ability_superspeed: player.last_input.ability_superspeed
                    || input.ability_superspeed,
                ability_swap: player.last_input.ability_swap || input.ability_swap,
                ability_ultimate: player.last_input.ability_ultimate || input.ability_ultimate,
                ..input
            };
            player.last_input = merged;
        }
        Some(player_id)
    }

    pub fn tick(&mut self, dt: f32) -> WorldSnapshot {
        self.pending_events.clear();
        self.tick += 1;

        match self.phase {
            MatchPhase::PhaseLobby => self.tick_lobby(),
            MatchPhase::PhaseStarting => self.tick_starting(dt),
            MatchPhase::PhasePlaying => self.tick_playing(dt),
            MatchPhase::PhaseEnding => self.tick_ending(dt),
        }

        self.snapshot()
    }

    fn snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            players: self
                .players
                .iter()
                .map(|(&id, p)| PlayerState {
                    player_id: id,
                    name: p.name.clone(),
                    position: Some(to_pb(p.position)),
                    yaw: p.yaw,
                    health: p.health,
                    alive: p.alive,
                    kills: p.kills,
                    deaths: p.deaths,
                    scale: if self.giant_mode_timer > 0.0 {
                        2.5
                    } else {
                        1.0
                    },
                    ready: p.ready,
                    held_item: p.held_item as i32,
                    just_died: p.just_died,
                    respawn_time_remaining: if p.alive {
                        0.0
                    } else {
                        p.respawn_timer.max(0.0)
                    },
                    dash_cooldown_remaining: p.dash_cooldown.max(0.0),
                    superspeed_cooldown_remaining: p.superspeed_cooldown.max(0.0),
                    swap_cooldown_remaining: p.swap_cooldown.max(0.0),
                    ultimate_cooldown_remaining: p.ultimate_cooldown.max(0.0),
                })
                .collect(),
            powerups: self
                .powerups
                .iter()
                .map(|(&id, p)| PowerupState {
                    powerup_id: id,
                    kind: p.kind as i32,
                    position: Some(to_pb(p.position)),
                    active: p.active,
                })
                .collect(),
            events: self.pending_events.clone(),
            tick: self.tick,
            gravity_scale: self.gravity_scale,
            phase: self.phase as i32,
            phase_time_remaining: self.phase_timer.max(0.0),
            points_of_interest: self
                .pois
                .iter()
                .enumerate()
                .map(|(i, poi)| PointOfInterest {
                    poi_id: i as u32,
                    name: poi.name.to_string(),
                    position: Some(to_pb(poi.position)),
                    radius: self.config.poi_radius,
                })
                .collect(),
            items: self
                .items
                .iter()
                .map(|(&id, item)| ItemState {
                    item_id: id,
                    kind: item.kind as i32,
                    position: Some(to_pb(item.position)),
                    held: item.held_by.is_some(),
                    holder_id: item.held_by.unwrap_or(0),
                    dancing: item.cat.dancing,
                    active: item.active,
                })
                .collect(),
            winner_player_id: self.winner_id,
            winner_name: self.winner_name.clone(),
            time_of_day: self.time_of_day,
            weather: self.weather as i32,
            weather_transition: self.weather_transition,
        }
    }
}
