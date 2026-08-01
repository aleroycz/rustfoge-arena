use rf_math::Vec3;

use super::ArenaWorld;

impl ArenaWorld {
    pub(crate) fn update_movement(&mut self, dt: f32) {
        let half_extent = self.config.arena_half_extent;
        let player_speed = self.config.player_speed;
        let jump_speed = self.config.jump_speed;
        let gravity = self.config.gravity;
        let gravity_scale = self.gravity_scale;

        for player in self.players.values_mut() {
            if !player.alive {
                continue;
            }
            let yaw = player.last_input.yaw;
            let pitch = player.last_input.pitch;
            let move_x = player.last_input.move_x;
            let move_y = player.last_input.move_y;
            let jump = player.last_input.jump;
            player.last_input.jump = false;

            player.yaw = yaw;
            player.pitch = pitch;

            let yaw_rotation = rf_math::Quat::from_rotation_y(yaw);
            let forward = yaw_rotation * Vec3::NEG_Z;
            let right = yaw_rotation * Vec3::X;
            let move_axis = Vec3::new(move_x, 0.0, move_y).clamp_length_max(1.0);
            let planar = (right * move_axis.x + forward * move_axis.z)
                * player_speed
                * player.speed_multiplier(&self.config);

            player.velocity.x = planar.x;
            player.velocity.z = planar.z;
            player.velocity.y += gravity * gravity_scale * dt;

            let grounded = player.position.y <= 0.001 && player.velocity.y <= 0.0;
            if grounded && jump {
                player.velocity.y = jump_speed;
            }

            player.position += player.velocity * dt;
            if player.position.y < 0.0 {
                player.position.y = 0.0;
                player.velocity.y = 0.0;
            }
            player.position.x = player.position.x.clamp(-half_extent, half_extent);
            player.position.z = player.position.z.clamp(-half_extent, half_extent);
        }
    }
}
