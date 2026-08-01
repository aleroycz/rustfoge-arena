use rf_math::Vec3;
use rf_net::EventKind;

use super::{util::player_event, ArenaWorld};

impl ArenaWorld {
    pub(crate) fn resolve_firing(&mut self, _dt: f32) {
        let fire_cooldown = self.config.fire_cooldown;
        let rifle_damage = self.config.rifle_damage;
        let respawn_delay = self.config.respawn_delay;
        let ultimate_unlock_kills = self.config.ultimate_unlock_kills;

        let shooters: Vec<(u32, Vec3, Vec3, f32)> = self
            .players
            .iter_mut()
            .filter_map(|(&id, player)| {
                if !player.alive || !player.last_input.fire || player.fire_cooldown > 0.0 {
                    return None;
                }
                player.fire_cooldown = fire_cooldown;
                let yaw_rotation = rf_math::Quat::from_rotation_y(player.yaw);
                let pitch_rotation = rf_math::Quat::from_rotation_x(player.pitch);
                let direction = yaw_rotation * pitch_rotation * Vec3::NEG_Z;
                let origin = player.position + Vec3::new(0.0, 0.9, 0.0);
                Some((id, origin, direction, player.damage_multiplier()))
            })
            .collect();

        for (shooter_id, origin, direction, damage_mult) in shooters {
            self.pending_events.push(player_event(
                EventKind::EventWeaponFired,
                origin,
                direction,
                shooter_id,
                0.0,
            ));

            if let Some(target_id) = self.raycast_players(shooter_id, origin, direction) {
                let damage = (rifle_damage as f32 * damage_mult) as i32;
                let mut killed = false;
                if let Some(target) = self.players.get_mut(&target_id) {
                    target.health -= damage;
                    if target.health <= 0 && target.alive {
                        target.alive = false;
                        target.deaths += 1;
                        target.just_died = true;
                        target.respawn_timer = respawn_delay;
                        killed = true;
                    }
                }
                if killed {
                    if let Some(shooter) = self.players.get_mut(&shooter_id) {
                        shooter.kills += 1;
                        if shooter.kills == ultimate_unlock_kills {
                            let position = shooter.position;
                            self.pending_events.push(super::util::event(
                                EventKind::EventAbilityUnlocked,
                                position,
                                shooter.kills as f32,
                            ));
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn raycast_players(&self, shooter_id: u32, origin: Vec3, direction: Vec3) -> Option<u32> {
        self.raycast_players_within(
            shooter_id,
            origin,
            direction,
            self.config.hitscan_range,
            self.config.hit_radius,
        )
    }

    pub(crate) fn raycast_players_within(
        &self,
        shooter_id: u32,
        origin: Vec3,
        direction: Vec3,
        max_range: f32,
        base_hit_radius: f32,
    ) -> Option<u32> {
        let hit_radius = if self.giant_mode_timer > 0.0 {
            base_hit_radius * 2.5
        } else {
            base_hit_radius
        };
        let mut best: Option<(u32, f32)> = None;
        for (&id, player) in &self.players {
            if id == shooter_id || !player.alive {
                continue;
            }
            let center = player.position + Vec3::new(0.0, 0.5, 0.0);
            let to_center = center - origin;
            let t = to_center.dot(direction);
            if t < 0.0 || t > max_range {
                continue;
            }
            let closest = origin + direction * t;
            if closest.distance(center) <= hit_radius
                && best.map(|(_, best_t)| t < best_t).unwrap_or(true)
            {
                best = Some((id, t));
            }
        }
        best.map(|(id, _)| id)
    }
}
