use rf_math::Vec3;
use rf_net::{ItemKind, PlayerInput};

use crate::config::ServerConfig;

pub struct ServerPlayer {
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: i32,
    pub alive: bool,
    pub kills: u32,
    pub deaths: u32,
    pub respawn_timer: f32,
    pub fire_cooldown: f32,
    pub speed_boost_timer: f32,
    pub damage_boost_timer: f32,
    pub ready: bool,
    pub held_item: ItemKind,
    pub just_died: bool,
    pub last_input: PlayerInput,
    pub(crate) dash_cooldown: f32,
    pub(crate) superspeed_cooldown: f32,
    pub(crate) swap_cooldown: f32,
    pub(crate) ultimate_cooldown: f32,
    pub(crate) superspeed_ability_timer: f32,
}

impl ServerPlayer {
    pub(crate) fn new(name: String, spawn: Vec3, config: &ServerConfig) -> Self {
        Self {
            name,
            position: spawn,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            health: config.max_health,
            alive: true,
            kills: 0,
            deaths: 0,
            respawn_timer: 0.0,
            fire_cooldown: 0.0,
            speed_boost_timer: 0.0,
            damage_boost_timer: 0.0,
            ready: false,
            held_item: ItemKind::ItemNone,
            just_died: false,
            last_input: PlayerInput::default(),
            dash_cooldown: 0.0,
            superspeed_cooldown: 0.0,
            swap_cooldown: 0.0,
            ultimate_cooldown: 0.0,
            superspeed_ability_timer: 0.0,
        }
    }

    pub(crate) fn speed_multiplier(&self, config: &ServerConfig) -> f32 {
        let ability = if self.superspeed_ability_timer > 0.0 {
            config.superspeed_multiplier
        } else {
            1.0
        };
        let powerup = if self.speed_boost_timer > 0.0 { 1.6 } else { 1.0 };
        ability.max(powerup)
    }

    pub(crate) fn ability_unlocked(&self, threshold: u32) -> bool {
        self.kills >= threshold
    }

    pub(crate) fn damage_multiplier(&self) -> f32 {
        if self.damage_boost_timer > 0.0 {
            1.75
        } else {
            1.0
        }
    }
}
