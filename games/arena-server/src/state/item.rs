use rand::Rng;
use rf_math::Vec3;
use rf_net::{EventKind, ItemKind};

use super::{
    util::{event, random_arena_point},
    ArenaWorld,
};

#[derive(Default)]
pub struct CatState {
    pub dancing: bool,
    pub dance_timer: f32,
    pub roam_timer: f32,
}

pub struct ServerItem {
    pub kind: ItemKind,
    pub position: Vec3,
    pub held_by: Option<u32>,
    pub active: bool,
    pub respawn_timer: f32,

    pub throw_cooldown: f32,
    pub cat: CatState,
}

impl ArenaWorld {
    pub(crate) fn spawn_items(&mut self) {
        self.items.clear();
        self.next_item_id = 0;
        for _ in 0..self.config.heal_item_count {
            let position = random_arena_point(&mut self.rng, &self.config, 1.0);
            self.spawn_item(ItemKind::ItemHeal, position);
        }
        let cat_position = random_arena_point(&mut self.rng, &self.config, 1.0);
        self.spawn_item(ItemKind::ItemDancingCat, cat_position);
    }

    pub(crate) fn spawn_item(&mut self, kind: ItemKind, position: Vec3) -> u32 {
        let id = self.next_item_id;
        self.next_item_id += 1;
        let (roam_min, roam_max) = self.config.cat_roam_interval();
        self.items.insert(
            id,
            ServerItem {
                kind,
                position,
                held_by: None,
                active: true,
                respawn_timer: 0.0,
                throw_cooldown: 0.0,
                cat: CatState {
                    roam_timer: self.rng.gen_range(roam_min..roam_max),
                    ..Default::default()
                },
            },
        );
        id
    }

    pub(crate) fn resolve_items(&mut self, dt: f32) {
        let half_extent = self.config.arena_half_extent;
        let throw_distance = self.config.item_throw_distance;
        let throw_cooldown = self.config.item_pickup_cooldown;
        let (roam_min, roam_max) = self.config.cat_roam_interval();

        let mut drops = Vec::new();
        for (&player_id, player) in &mut self.players {
            if player.last_input.use_item {
                player.last_input.use_item = false;
                if player.held_item != ItemKind::ItemNone {
                    let yaw_rotation = rf_math::Quat::from_rotation_y(player.yaw);
                    let forward = yaw_rotation * Vec3::NEG_Z;
                    let throw_pos = (player.position + forward * throw_distance).clamp(
                        Vec3::splat(-half_extent + 1.0),
                        Vec3::splat(half_extent - 1.0),
                    );
                    drops.push((player_id, throw_pos, player.held_item));
                    player.held_item = ItemKind::ItemNone;
                }
            }
        }
        for (player_id, throw_pos, kind) in drops {
            if let Some(item) = self
                .items
                .values_mut()
                .find(|i| i.held_by == Some(player_id))
            {
                item.held_by = None;
                item.active = true;
                item.position = Vec3::new(throw_pos.x, 1.0, throw_pos.z);
                item.throw_cooldown = throw_cooldown;
                if kind == ItemKind::ItemDancingCat {
                    item.cat.roam_timer = self.rng.gen_range(roam_min..roam_max);
                    self.pending_events
                        .push(event(EventKind::EventCatDropped, item.position, 0.0));
                }
            }
        }

        let item_pickup_radius = self.config.item_pickup_radius;
        let pickups: Vec<(u32, u32, ItemKind)> = self
            .items
            .iter()
            .filter(|(_, item)| {
                item.held_by.is_none()
                    && item.active
                    && !item.cat.dancing
                    && item.throw_cooldown <= 0.0
            })
            .filter_map(|(&item_id, item)| {
                self.players
                    .iter()
                    .find(|(_, p)| {
                        p.alive
                            && p.held_item == ItemKind::ItemNone
                            && p.position.distance(item.position) <= item_pickup_radius
                    })
                    .map(|(&player_id, _)| (item_id, player_id, item.kind))
            })
            .collect();

        let heal_amount = self.config.heal_item_amount;
        let max_health = self.config.max_health;
        let heal_respawn_delay = self.config.heal_item_respawn_delay;
        for (item_id, player_id, kind) in pickups {
            match kind {
                ItemKind::ItemHeal => {
                    if let Some(player) = self.players.get_mut(&player_id) {
                        player.health = (player.health + heal_amount).min(max_health);
                    }
                    if let Some(item) = self.items.get_mut(&item_id) {
                        item.active = false;
                        item.respawn_timer = heal_respawn_delay;
                    }
                }
                ItemKind::ItemDancingCat => {
                    if let Some(player) = self.players.get_mut(&player_id) {
                        player.held_item = ItemKind::ItemDancingCat;
                    }
                    if let Some(item) = self.items.get_mut(&item_id) {
                        item.held_by = Some(player_id);
                    }
                }
                ItemKind::ItemNone => {}
            }
        }

        let alive_player_positions: Vec<Vec3> = self
            .players
            .values()
            .filter(|p| p.alive)
            .map(|p| p.position)
            .collect();
        let cat_dance_duration = self.config.cat_dance_duration;
        let mut cat_teleports = Vec::new();
        for item in self.items.values_mut() {
            item.throw_cooldown = (item.throw_cooldown - dt).max(0.0);

            if item.kind == ItemKind::ItemHeal && !item.active {
                item.respawn_timer -= dt;
                if item.respawn_timer <= 0.0 {
                    item.active = true;
                }
            }

            if item.kind == ItemKind::ItemDancingCat && item.held_by.is_none() {
                if item.cat.dancing {
                    item.cat.dance_timer -= dt;
                    if item.cat.dance_timer <= 0.0 {
                        item.cat.dancing = false;
                        item.cat.roam_timer = self.rng.gen_range(roam_min..roam_max);
                    }
                } else {
                    item.cat.roam_timer -= dt;
                    if item.cat.roam_timer <= 0.0 && !alive_player_positions.is_empty() {
                        let target = alive_player_positions
                            [self.rng.gen_range(0..alive_player_positions.len())];
                        let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
                        let offset =
                            Vec3::new(angle.cos(), 0.0, angle.sin()) * self.rng.gen_range(1.5..3.5);
                        item.position = (target + offset).clamp(
                            Vec3::splat(-half_extent + 1.0),
                            Vec3::splat(half_extent - 1.0),
                        );
                        item.position.y = 1.0;
                        item.cat.dancing = true;
                        item.cat.dance_timer = cat_dance_duration;
                        cat_teleports.push(item.position);
                    }
                }
            }
        }
        for position in cat_teleports {
            self.pending_events
                .push(event(EventKind::EventCatTeleported, position, 0.0));
        }
    }
}
