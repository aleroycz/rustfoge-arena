use rand::Rng;
use rf_math::Vec3;

use super::{util::random_arena_point, ArenaWorld};

const POI_NAMES: [&str; 8] = [
    "THE PIT",
    "CRASH SITE",
    "SUPPLY DEPOT",
    "THE THRONE",
    "BUNKER 7",
    "SCRAPYARD",
    "OUTPOST ALPHA",
    "THE HOLLOW",
];

pub struct Poi {
    pub name: &'static str,
    pub position: Vec3,
}

impl ArenaWorld {
    pub(crate) fn generate_pois(&mut self) {
        self.pois.clear();
        let mut names: Vec<&'static str> = POI_NAMES.to_vec();
        for _ in 0..self.config.poi_count.min(names.len()) {
            let index = self.rng.gen_range(0..names.len());
            let name = names.remove(index);
            let position = random_arena_point(&mut self.rng, &self.config, 0.0);
            self.pois.push(Poi { name, position });
        }
    }

    pub(crate) fn spawn_players_at_pois(&mut self) {
        if self.pois.is_empty() {
            for player in self.players.values_mut() {
                player.position = random_arena_point(&mut self.rng, &self.config, 0.5);
            }
            return;
        }
        let poi_positions: Vec<Vec3> = self.pois.iter().map(|p| p.position).collect();
        let half_extent = self.config.arena_half_extent;
        let poi_radius = self.config.poi_radius;
        let max_health = self.config.max_health;
        for player in self.players.values_mut() {
            let poi = poi_positions[self.rng.gen_range(0..poi_positions.len())];
            let offset_angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let offset_radius = self.rng.gen_range(2.0..poi_radius);
            let offset = Vec3::new(offset_angle.cos(), 0.0, offset_angle.sin()) * offset_radius;
            player.position = (poi + offset).clamp(
                Vec3::splat(-half_extent + 1.0),
                Vec3::splat(half_extent - 1.0),
            );
            player.position.y = 0.5;
            player.velocity = Vec3::ZERO;
            player.health = max_health;
            player.alive = true;
        }
    }
}
