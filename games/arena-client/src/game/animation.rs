

use rf_engine::math::{Quat, Transform, Vec3};
use rf_engine::EngineContext;
use rf_net::WeatherKind;


use super::*;
use super::util::lerp_angle;

impl PlayingScreen {
    pub(super) fn interpolate_remote_players(&mut self, ctx: &mut EngineContext) {
        let Some(latest) = &self.latest_snapshot else {
            return;
        };
        let t = if self.prev_snapshot.is_some() {
            ((ctx.time.elapsed_seconds() - self.snapshot_received_at) / self.snapshot_interval)
                .clamp(0.0, 1.0)
        } else {
            1.0
        };

        for player in &latest.players {
            if player.player_id == self.local_player_id {
                continue;
            }
            if self.death_anim.contains_key(&player.player_id) {
                continue;
            }
            let Some(remote) = self.remote_players.get(&player.player_id) else {
                continue;
            };

            let cur_pos = player
                .position
                .as_ref()
                .map(|p| Vec3::new(p.x, p.y, p.z))
                .unwrap_or(Vec3::ZERO);

            let (from_pos, from_yaw) = self
                .prev_snapshot
                .as_ref()
                .and_then(|prev| {
                    prev.players
                        .iter()
                        .find(|p| p.player_id == player.player_id)
                })
                .map(|prev| {
                    let pos = prev
                        .position
                        .as_ref()
                        .map(|p| Vec3::new(p.x, p.y, p.z))
                        .unwrap_or(cur_pos);
                    (pos, prev.yaw)
                })
                .unwrap_or((cur_pos, player.yaw));

            let position = from_pos.lerp(cur_pos, t);
            let yaw = lerp_angle(from_yaw, player.yaw, t);
            let scale = player.scale.max(0.01);

            let transform = Transform::from_translation(position + Vec3::new(0.0, 0.5, 0.0))
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::splat(scale));
            let weapon_transform = Transform::from_translation(
                position
                    + Vec3::new(0.0, 0.5, 0.0)
                    + Quat::from_rotation_y(yaw) * RIGHT_WEAPON_OFFSET * scale,
            )
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale));

            let _ = ctx.world.insert_one(remote.entity, transform);
            let _ = ctx.world.insert_one(remote.weapon_entity, weapon_transform);
        }
    }

    pub(super) fn animate_deaths(&mut self, ctx: &mut EngineContext) {
        for (&player_id, &t) in &self.death_anim {
            let Some(remote) = self.remote_players.get(&player_id) else {
                continue;
            };
            let Some(player) = self
                .latest_snapshot
                .as_ref()
                .and_then(|s| s.players.iter().find(|p| p.player_id == player_id))
            else {
                continue;
            };
            let position = player
                .position
                .as_ref()
                .map(|p| Vec3::new(p.x, p.y, p.z))
                .unwrap_or(Vec3::ZERO);

            let fall_progress = (t / DEATH_ANIM_DURATION).min(1.0);

            let tilt = fall_progress * std::f32::consts::FRAC_PI_2;
            let sink = fall_progress * 0.4;
            let transform = Transform::from_translation(position + Vec3::new(0.0, 0.5 - sink, 0.0))
                .with_rotation(Quat::from_rotation_z(tilt) * Quat::from_rotation_y(player.yaw))
                .with_scale(Vec3::splat(player.scale.max(0.01)));
            let _ = ctx.world.insert_one(remote.entity, transform);
            let _ = ctx.world.insert_one(remote.weapon_entity, transform);
        }
    }

    pub(super) fn animate_cat_dance(&mut self, ctx: &mut EngineContext) {
        let Some(cat) = &mut self.cat_visual else {
            return;
        };
        if !self.prev_cat_dancing {
            return;
        }
        cat.dance_phase += ctx.time.delta_seconds();
        let spin = Quat::from_rotation_y(cat.dance_phase * CAT_DANCE_SPIN_SPEED);
        let bob = (cat.dance_phase * CAT_DANCE_JUMP_SPEED).sin().abs() * CAT_DANCE_JUMP_HEIGHT;

        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };
        let Some(item) = snapshot
            .items
            .iter()
            .find(|i| Some(i.item_id) == self.cat_item_id)
        else {
            return;
        };
        let Some(pos) = &item.position else { return };
        let base = Vec3::new(pos.x, pos.y, pos.z);
        let transform = Transform::from_translation(base + Vec3::new(0.0, 0.4 + bob, 0.0))
            .with_rotation(spin)
            .with_scale(Vec3::splat(0.4));
        for &entity in &cat.entities {
            let _ = ctx.world.insert_one(entity, transform);
        }
    }

    pub(super) fn update_weather_and_lighting(&mut self, ctx: &mut EngineContext) {
        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };

        let time_of_day = snapshot.time_of_day;
        let sun_dir = Vec3::new(time_of_day.cos(), time_of_day.sin(), 0.25).normalize();
        ctx.light.direction = -sun_dir;

        let weather = WeatherKind::try_from(snapshot.weather).unwrap_or(WeatherKind::WeatherClear);
        let (target_cloud, target_storm, target_rain) = match weather {
            WeatherKind::WeatherClear => (0.1, 0.0, 0.0),
            WeatherKind::WeatherCloudy => (0.6, 0.05, 0.0),
            WeatherKind::WeatherRain => (0.8, 0.2, 1.0),
            WeatherKind::WeatherStorm => (0.95, 0.85, 1.0),
        };

        let smoothing = (ctx.time.delta_seconds() * 1.0).min(1.0);
        self.current_cloud_coverage += (target_cloud - self.current_cloud_coverage) * smoothing;
        self.current_storm_darkness += (target_storm - self.current_storm_darkness) * smoothing;
        self.current_rain_intensity += (target_rain - self.current_rain_intensity) * smoothing;

        ctx.sky.cloud_coverage = self.current_cloud_coverage;
        ctx.sky.storm_darkness = self.current_storm_darkness;
        ctx.sky.star_brightness = 1.0;

        ctx.fog.density = 0.004 + self.current_storm_darkness * 0.03;
        ctx.fog.color = if self.current_storm_darkness > 0.3 {
            [0.35, 0.37, 0.4]
        } else {
            [0.7, 0.75, 0.8]
        };

        ctx.light.intensity = 1.3 * (1.0 - self.current_storm_darkness * 0.6);
    }

}
