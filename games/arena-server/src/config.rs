use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind_addr_default: String,
    pub tick_rate_hz: f32,

    pub arena_half_extent: f32,

    pub player_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub max_health: i32,

    pub hitscan_range: f32,
    pub hit_radius: f32,
    pub rifle_damage: i32,
    pub fire_cooldown: f32,
    pub respawn_delay: f32,

    pub powerup_respawn_delay: f32,
    pub powerup_pickup_radius: f32,

    pub event_min_interval: f32,
    pub event_max_interval: f32,
    pub orbital_strike_warning_time: f32,
    pub orbital_strike_radius: f32,
    pub orbital_strike_damage: i32,
    pub zero_gravity_duration: f32,
    pub giant_mode_duration: f32,

    pub starting_countdown: f32,
    pub match_duration: f32,
    pub ending_duration: f32,
    pub poi_count: usize,
    pub poi_radius: f32,

    pub item_pickup_radius: f32,
    pub heal_item_count: usize,
    pub heal_item_amount: i32,
    pub heal_item_respawn_delay: f32,
    pub cat_dance_duration: f32,
    pub cat_roam_interval_min: f32,
    pub cat_roam_interval_max: f32,
    pub item_throw_distance: f32,
    pub item_pickup_cooldown: f32,

    pub dash_unlock_kills: u32,
    pub superspeed_unlock_kills: u32,
    pub swap_unlock_kills: u32,
    pub ultimate_unlock_kills: u32,

    pub dash_cooldown: f32,
    pub dash_distance: f32,
    pub superspeed_cooldown: f32,
    pub superspeed_duration: f32,
    pub superspeed_multiplier: f32,
    pub swap_cooldown: f32,
    pub swap_range: f32,
    pub swap_target_radius: f32,
    pub ultimate_cooldown: f32,
    pub ultimate_radius: f32,
    pub ultimate_damage: i32,
    pub ultimate_launch_up: f32,
    pub ultimate_scatter_radius: f32,

    pub day_cycle_seconds: f32,
    pub weather_min_interval: f32,
    pub weather_max_interval: f32,
    pub weather_transition_duration: f32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr_default: "0.0.0.0:50051".to_string(),
            tick_rate_hz: 20.0,

            arena_half_extent: 30.0,

            player_speed: 9.0,
            jump_speed: 9.5,
            gravity: -18.0,
            max_health: 100,

            hitscan_range: 60.0,
            hit_radius: 1.0,
            rifle_damage: 18,
            fire_cooldown: 0.15,
            respawn_delay: 15.0,

            powerup_respawn_delay: 15.0,
            powerup_pickup_radius: 1.5,

            event_min_interval: 25.0,
            event_max_interval: 45.0,
            orbital_strike_warning_time: 3.0,
            orbital_strike_radius: 8.0,
            orbital_strike_damage: 60,
            zero_gravity_duration: 12.0,
            giant_mode_duration: 15.0,

            starting_countdown: 5.0,
            match_duration: 30.0 * 60.0,
            ending_duration: 15.0,
            poi_count: 5,
            poi_radius: 7.0,

            item_pickup_radius: 1.5,
            heal_item_count: 2,
            heal_item_amount: 50,
            heal_item_respawn_delay: 20.0,
            cat_dance_duration: 5.0,
            cat_roam_interval_min: 8.0,
            cat_roam_interval_max: 16.0,
            item_throw_distance: 3.0,
            item_pickup_cooldown: 0.5,

            dash_unlock_kills: 0,
            superspeed_unlock_kills: 0,
            swap_unlock_kills: 0,
            ultimate_unlock_kills: 5,

            dash_cooldown: 4.0,
            dash_distance: 9.0,
            superspeed_cooldown: 14.0,
            superspeed_duration: 5.0,
            superspeed_multiplier: 2.4,
            swap_cooldown: 10.0,
            swap_range: 45.0,
            swap_target_radius: 3.0,
            ultimate_cooldown: 45.0,
            ultimate_radius: 12.0,
            ultimate_damage: 25,
            ultimate_launch_up: 13.0,
            ultimate_scatter_radius: 3.0,

            day_cycle_seconds: 300.0,
            weather_min_interval: 45.0,
            weather_max_interval: 90.0,
            weather_transition_duration: 6.0,
        }
    }
}

impl ServerConfig {
    fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join("server_config.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(err) => {
                    log::warn!(
                        "failed to parse {path:?} ({err}) -- using default config"
                    );
                    Self::default()
                }
            },
            Err(_) => {
                let config = Self::default();
                config.write_default_template(&path);
                config
            }
        }
    }

    fn write_default_template(&self, path: &PathBuf) {
        if let Ok(toml_string) = toml::to_string_pretty(self) {
            if std::fs::write(path, toml_string).is_ok() {
                log::info!("wrote default server config to {path:?}");
            }
        }
    }

    pub fn cat_roam_interval(&self) -> (f32, f32) {
        (self.cat_roam_interval_min, self.cat_roam_interval_max)
    }
}
