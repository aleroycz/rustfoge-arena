use rf_math::Vec3;
use rf_net::PowerupKind;

use super::{util::random_arena_point, ArenaWorld};

pub struct ServerPowerup {
    pub kind: PowerupKind,
    pub position: Vec3,
    pub active: bool,
    pub respawn_timer: f32,
}

impl ArenaWorld {
    pub(crate) fn spawn_powerups(&mut self) {
        self.powerups.clear();
        let mut next_id = 0u32;
        for kind in [
            PowerupKind::PowerupHealth,
            PowerupKind::PowerupSpeed,
            PowerupKind::PowerupDamage,
        ] {
            for _ in 0..2 {
                let position = random_arena_point(&mut self.rng, &self.config, 1.0);
                self.powerups.insert(
                    next_id,
                    ServerPowerup {
                        kind,
                        position,
                        active: true,
                        respawn_timer: 0.0,
                    },
                );
                next_id += 1;
            }
        }
    }

    pub(crate) fn resolve_powerups(&mut self) {
        let pickup_radius = self.config.powerup_pickup_radius;
        let max_health = self.config.max_health;
        let pickups: Vec<(u32, u32, PowerupKind)> = self
            .powerups
            .iter()
            .filter(|(_, p)| p.active)
            .filter_map(|(&powerup_id, powerup)| {
                self.players
                    .iter()
                    .find(|(_, p)| p.alive && p.position.distance(powerup.position) <= pickup_radius)
                    .map(|(&player_id, _)| (powerup_id, player_id, powerup.kind))
            })
            .collect();

        for (powerup_id, player_id, kind) in pickups {
            if let Some(player) = self.players.get_mut(&player_id) {
                match kind {
                    PowerupKind::PowerupHealth => {
                        player.health = (player.health + 40).min(max_health)
                    }
                    PowerupKind::PowerupSpeed => player.speed_boost_timer = 8.0,
                    PowerupKind::PowerupDamage => player.damage_boost_timer = 8.0,
                }
            }
            if let Some(powerup) = self.powerups.get_mut(&powerup_id) {
                powerup.active = false;
                powerup.respawn_timer = self.config.powerup_respawn_delay;
            }
        }

        for powerup in self.powerups.values_mut() {
            if !powerup.active {
                powerup.respawn_timer -= 1.0 / 20.0;
                if powerup.respawn_timer <= 0.0 {
                    powerup.active = true;
                }
            }
        }
    }
}
