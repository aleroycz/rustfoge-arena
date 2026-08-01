use rand::Rng;
use rf_math::Vec3;
use rf_net::EventKind;

use super::{util::player_event, ArenaWorld};

impl ArenaWorld {
    pub(crate) fn resolve_abilities(&mut self, dt: f32) {
        for player in self.players.values_mut() {
            player.dash_cooldown = (player.dash_cooldown - dt).max(0.0);
            player.superspeed_cooldown = (player.superspeed_cooldown - dt).max(0.0);
            player.swap_cooldown = (player.swap_cooldown - dt).max(0.0);
            player.ultimate_cooldown = (player.ultimate_cooldown - dt).max(0.0);
            player.superspeed_ability_timer = (player.superspeed_ability_timer - dt).max(0.0);
        }

        struct Trigger {
            player_id: u32,
            dash: bool,
            superspeed: bool,
            swap: bool,
            ultimate: bool,
        }
        let dash_unlock = self.config.dash_unlock_kills;
        let superspeed_unlock = self.config.superspeed_unlock_kills;
        let swap_unlock = self.config.swap_unlock_kills;
        let ultimate_unlock = self.config.ultimate_unlock_kills;

        let mut triggers = Vec::new();
        for (&id, player) in &mut self.players {
            let dash_pressed = player.last_input.ability_dash;
            let superspeed_pressed = player.last_input.ability_superspeed;
            let swap_pressed = player.last_input.ability_swap;
            let ultimate_pressed = player.last_input.ability_ultimate;

            player.last_input.ability_dash = false;
            player.last_input.ability_superspeed = false;
            player.last_input.ability_swap = false;
            player.last_input.ability_ultimate = false;

            if !player.alive {
                continue;
            }

            let dash = dash_pressed
                && player.ability_unlocked(dash_unlock)
                && player.dash_cooldown <= 0.0;
            let superspeed = superspeed_pressed
                && player.ability_unlocked(superspeed_unlock)
                && player.superspeed_cooldown <= 0.0;
            let swap = swap_pressed
                && player.ability_unlocked(swap_unlock)
                && player.swap_cooldown <= 0.0;
            let ultimate = ultimate_pressed
                && player.ability_unlocked(ultimate_unlock)
                && player.ultimate_cooldown <= 0.0;

            if dash || superspeed || swap || ultimate {
                triggers.push(Trigger {
                    player_id: id,
                    dash,
                    superspeed,
                    swap,
                    ultimate,
                });
            }
        }

        for trigger in triggers {
            if trigger.dash {
                self.activate_dash(trigger.player_id);
            }
            if trigger.superspeed {
                self.activate_superspeed(trigger.player_id);
            }
            if trigger.swap {
                self.activate_swap(trigger.player_id);
            }
            if trigger.ultimate {
                self.activate_ultimate(trigger.player_id);
            }
        }
    }

    fn activate_dash(&mut self, player_id: u32) {
        let half_extent = self.config.arena_half_extent;
        let dash_cooldown = self.config.dash_cooldown;
        let dash_distance = self.config.dash_distance;

        let Some(player) = self.players.get_mut(&player_id) else {
            return;
        };
        player.dash_cooldown = dash_cooldown;

        let yaw_rotation = rf_math::Quat::from_rotation_y(player.yaw);
        let move_axis = Vec3::new(player.last_input.move_x, 0.0, player.last_input.move_y);
        let local_dir = if move_axis.length_squared() > 0.01 {
            Vec3::new(move_axis.x, 0.0, -move_axis.z).normalize_or_zero()
        } else {
            Vec3::NEG_Z
        };
        let direction = yaw_rotation * local_dir;
        let new_x = (player.position.x + direction.x * dash_distance)
            .clamp(-half_extent + 0.5, half_extent - 0.5);
        let new_z = (player.position.z + direction.z * dash_distance)
            .clamp(-half_extent + 0.5, half_extent - 0.5);
        player.position.x = new_x;
        player.position.z = new_z;

        let new_position = player.position;
        self.pending_events.push(player_event(
            EventKind::EventAbilityDashUsed,
            new_position,
            direction,
            player_id,
            0.0,
        ));
    }

    fn activate_superspeed(&mut self, player_id: u32) {
        let superspeed_cooldown = self.config.superspeed_cooldown;
        let superspeed_duration = self.config.superspeed_duration;

        let Some(player) = self.players.get_mut(&player_id) else {
            return;
        };
        player.superspeed_cooldown = superspeed_cooldown;
        player.superspeed_ability_timer = superspeed_duration;

        let position = player.position;
        self.pending_events.push(player_event(
            EventKind::EventAbilitySuperspeedUsed,
            position,
            Vec3::ZERO,
            player_id,
            superspeed_duration,
        ));
    }

    fn activate_swap(&mut self, player_id: u32) {
        let swap_cooldown = self.config.swap_cooldown;
        let swap_range = self.config.swap_range;
        let swap_target_radius = self.config.swap_target_radius;

        let Some(caster) = self.players.get(&player_id) else {
            return;
        };
        let origin = caster.position + Vec3::new(0.0, 0.9, 0.0);
        let yaw_rotation = rf_math::Quat::from_rotation_y(caster.yaw);
        let pitch_rotation = rf_math::Quat::from_rotation_x(caster.pitch);
        let direction = yaw_rotation * pitch_rotation * Vec3::NEG_Z;

        if let Some(player) = self.players.get_mut(&player_id) {
            player.swap_cooldown = swap_cooldown;
        }

        let Some(target_id) = self.raycast_players_within(
            player_id,
            origin,
            direction,
            swap_range,
            swap_target_radius,
        ) else {
            return;
        };

        let caster_pos = self.players.get(&player_id).map(|p| p.position);
        let target_pos = self.players.get(&target_id).map(|p| p.position);
        if let (Some(caster_pos), Some(target_pos)) = (caster_pos, target_pos) {
            if let Some(p) = self.players.get_mut(&player_id) {
                p.position = target_pos;
            }
            if let Some(p) = self.players.get_mut(&target_id) {
                p.position = caster_pos;
            }

            self.pending_events.push(player_event(
                EventKind::EventAbilitySwapUsed,
                target_pos,
                direction,
                player_id,
                0.0,
            ));
            self.pending_events.push(player_event(
                EventKind::EventAbilitySwapUsed,
                caster_pos,
                direction,
                target_id,
                0.0,
            ));
        }
    }

    fn activate_ultimate(&mut self, player_id: u32) {
        let ultimate_cooldown = self.config.ultimate_cooldown;
        let ultimate_radius = self.config.ultimate_radius;
        let ultimate_damage = self.config.ultimate_damage;
        let ultimate_launch_up = self.config.ultimate_launch_up;
        let ultimate_scatter_radius = self.config.ultimate_scatter_radius;
        let half_extent = self.config.arena_half_extent;
        let respawn_delay = self.config.respawn_delay;

        let Some(caster_pos) = self.players.get(&player_id).map(|p| p.position) else {
            return;
        };
        if let Some(player) = self.players.get_mut(&player_id) {
            player.ultimate_cooldown = ultimate_cooldown;
        }

        let victim_ids: Vec<u32> = self
            .players
            .iter()
            .filter(|(&id, p)| {
                id != player_id && p.alive && p.position.distance(caster_pos) <= ultimate_radius
            })
            .map(|(&id, _)| id)
            .collect();

        let mut kills_scored = 0;
        for victim_id in victim_ids {
            let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let scatter = Vec3::new(angle.cos(), 0.0, angle.sin())
                * self.rng.gen_range(0.5..ultimate_scatter_radius);
            if let Some(victim) = self.players.get_mut(&victim_id) {
                victim.health -= ultimate_damage;
                victim.velocity.y = ultimate_launch_up;
                let new_pos = victim.position + scatter;
                victim.position.x = new_pos.x.clamp(-half_extent + 1.0, half_extent - 1.0);
                victim.position.z = new_pos.z.clamp(-half_extent + 1.0, half_extent - 1.0);
                if victim.health <= 0 && victim.alive {
                    victim.alive = false;
                    victim.deaths += 1;
                    victim.just_died = true;
                    victim.respawn_timer = respawn_delay;
                    kills_scored += 1;
                }
            }
        }
        if kills_scored > 0 {
            if let Some(caster) = self.players.get_mut(&player_id) {
                caster.kills += kills_scored;
            }
        }

        self.pending_events.push(player_event(
            EventKind::EventUltimateUsed,
            caster_pos,
            Vec3::ZERO,
            player_id,
            ultimate_radius,
        ));
        log::info!("player {player_id} used Quantum Breakdown");
    }
}
