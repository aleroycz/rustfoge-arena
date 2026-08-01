use std::sync::Arc;


use rf_engine::math::{Quat, Transform, Vec3};
use rf_engine::renderer::{GpuTexture, Material, Mesh, RenderObject};
use rf_engine::scene::spawn_spatial;
use rf_engine::EngineContext;
use rf_net::{
    ItemKind, MatchPhase,
    WorldSnapshot,
};


use super::*;
use super::assets::load_dancing_cat_model;
use super::util::{angle_deg_to_rad, player_color, powerup_color};

impl PlayingScreen {
    pub(super) fn apply_snapshot(&mut self, ctx: &mut EngineContext, snapshot: &WorldSnapshot) {
        let phase = MatchPhase::try_from(snapshot.phase).unwrap_or(MatchPhase::PhaseLobby);

        if phase == MatchPhase::PhaseLobby && self.poi_built {
            for entity in self.poi_entities.drain(..) {
                let _ = ctx.world.despawn(entity);
            }
            self.poi_built = false;
        }
        if !self.poi_built && !snapshot.points_of_interest.is_empty() {
            self.build_pois(ctx, snapshot);
            self.poi_built = true;
        }

        let mut seen_players = std::collections::HashSet::new();
        for player in &snapshot.players {
            seen_players.insert(player.player_id);
            if player.player_id == self.local_player_id {
                if self.prev_alive && !player.alive {
                    if let Ok(sink) = ctx.audio.play(&self.death_sound, self.config.sfx_gain()) {
                        sink.detach();
                    }
                    self.push_toast("YOU DIED");
                    if let Some(pos) = &player.position {
                        self.death_particles.origin = Vec3::new(pos.x, pos.y + 0.5, pos.z);
                        self.death_particles.burst(18);
                    }
                }
                if player.kills > self.prev_kills {
                    if let Ok(sink) = ctx
                        .audio
                        .play(&self.hit_marker_sound, self.config.sfx_gain())
                    {
                        sink.detach();
                    }
                }
                self.prev_alive = player.alive;
                self.prev_kills = player.kills;
                continue;
            }

            if player.just_died {
                self.death_anim.insert(player.player_id, 0.0);
                if let Some(pos) = &player.position {
                    self.death_particles.origin = Vec3::new(pos.x, pos.y + 0.5, pos.z);
                    self.death_particles.burst(18);
                }
            }

            let position = player
                .position
                .as_ref()
                .map(|p| Vec3::new(p.x, p.y, p.z))
                .unwrap_or(Vec3::ZERO);
            let transform = Transform::from_translation(position + Vec3::new(0.0, 0.5, 0.0))
                .with_rotation(Quat::from_rotation_y(player.yaw))
                .with_scale(Vec3::splat(player.scale.max(0.01)));
            let weapon_transform = Transform::from_translation(
                position
                    + Vec3::new(0.0, 0.5, 0.0)
                    + Quat::from_rotation_y(player.yaw)
                        * RIGHT_WEAPON_OFFSET
                        * player.scale.max(0.01),
            )
            .with_rotation(Quat::from_rotation_y(player.yaw))
            .with_scale(Vec3::splat(player.scale.max(0.01)));

            let visible = phase == MatchPhase::PhasePlaying
                && (player.alive || self.death_anim.contains_key(&player.player_id));

            if let Some(remote) = self.remote_players.get(&player.player_id) {
                if !self.death_anim.contains_key(&player.player_id) {
                    let _ = ctx.world.insert_one(remote.entity, transform);
                    let _ = ctx.world.insert_one(remote.weapon_entity, weapon_transform);
                }
                if let Ok(mut render) = ctx.world.get::<&mut RenderObject>(remote.entity) {
                    render.visible = visible;
                }
                if let Ok(mut render) = ctx.world.get::<&mut RenderObject>(remote.weapon_entity) {
                    render.visible = visible;
                }
            } else {
                let device = ctx.renderer.device();
                let queue = ctx.renderer.queue();
                let material_layout = ctx.renderer.material_bind_group_layout();
                let color = player_color(player.player_id);
                let texture = GpuTexture::solid_color(device, queue, color);
                let material = Arc::new(Material::new(
                    device,
                    queue,
                    material_layout,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                    0.1,
                    0.6,
                ));
                let mesh = Arc::new(Mesh::cuboid(device, Vec3::splat(1.0)));
                let render = ctx.renderer.create_render_object(mesh, material);
                let entity = spawn_spatial(ctx.world, transform);
                let _ = ctx.world.insert_one(entity, render);

                let weapon_render = ctx.renderer.create_render_object(
                    self.weapon_prop_mesh.clone(),
                    self.weapon_prop_material.clone(),
                );
                let weapon_entity = spawn_spatial(ctx.world, weapon_transform);
                let _ = ctx.world.insert_one(weapon_entity, weapon_render);

                self.remote_players.insert(
                    player.player_id,
                    RemotePlayer {
                        entity,
                        weapon_entity,
                    },
                );
            }
        }
        let disconnected_ids: Vec<u32> = self
            .remote_players
            .keys()
            .filter(|id| !seen_players.contains(id))
            .copied()
            .collect();
        for id in disconnected_ids {
            if let Some(remote) = self.remote_players.remove(&id) {
                let _ = ctx.world.despawn(remote.entity);
                let _ = ctx.world.despawn(remote.weapon_entity);
            }
            self.death_anim.remove(&id);
        }

        for powerup in &snapshot.powerups {
            let position = powerup
                .position
                .as_ref()
                .map(|p| Vec3::new(p.x, p.y, p.z))
                .unwrap_or(Vec3::ZERO);
            let transform = Transform::from_translation(position + Vec3::new(0.0, 0.5, 0.0));
            if let Some(&entity) = self.powerup_visuals.get(&powerup.powerup_id) {
                let _ = ctx.world.insert_one(entity, transform);
                if let Ok(mut render) = ctx.world.get::<&mut RenderObject>(entity) {
                    render.visible = powerup.active;
                }
            } else {
                let device = ctx.renderer.device();
                let queue = ctx.renderer.queue();
                let material_layout = ctx.renderer.material_bind_group_layout();
                let color = powerup_color(powerup.kind);
                let texture = GpuTexture::solid_color(device, queue, color);
                let material = Arc::new(Material::new(
                    device,
                    queue,
                    material_layout,
                    texture,
                    [1.0, 1.0, 1.0, 1.0],
                    0.5,
                    0.3,
                ));
                let mesh = Arc::new(Mesh::uv_sphere(device, 0.4, 16, 16));
                let render = ctx.renderer.create_render_object(mesh, material);
                let entity = spawn_spatial(ctx.world, transform);
                let _ = ctx.world.insert_one(entity, render);
                self.powerup_visuals.insert(powerup.powerup_id, entity);
            }
        }

        self.apply_items(ctx, snapshot);

        for event in &snapshot.events {
            self.handle_event(ctx, event);
        }
    }

    fn apply_items(&mut self, ctx: &mut EngineContext, snapshot: &WorldSnapshot) {
        for item in &snapshot.items {
            let position = item
                .position
                .as_ref()
                .map(|p| Vec3::new(p.x, p.y, p.z))
                .unwrap_or(Vec3::ZERO);

            match ItemKind::try_from(item.kind).unwrap_or(ItemKind::ItemNone) {
                ItemKind::ItemHeal => {
                    let was_active = self
                        .prev_heal_active
                        .get(&item.item_id)
                        .copied()
                        .unwrap_or(true);
                    if was_active && !item.active {
                        if let Ok(sink) = ctx.audio.play(&self.pickup_sound, self.config.sfx_gain())
                        {
                            sink.detach();
                        }
                    }
                    self.prev_heal_active.insert(item.item_id, item.active);

                    let transform =
                        Transform::from_translation(position + Vec3::new(0.0, 0.5, 0.0));
                    if let Some(&entity) = self.heal_item_visuals.get(&item.item_id) {
                        let _ = ctx.world.insert_one(entity, transform);
                        if let Ok(mut render) = ctx.world.get::<&mut RenderObject>(entity) {
                            render.visible = item.active;
                        }
                    } else {
                        let device = ctx.renderer.device();
                        let queue = ctx.renderer.queue();
                        let material_layout = ctx.renderer.material_bind_group_layout();
                        let texture = GpuTexture::solid_color(device, queue, [90, 255, 130, 255]);
                        let material = Arc::new(Material::new(
                            device,
                            queue,
                            material_layout,
                            texture,
                            [1.0, 1.0, 1.0, 1.0],
                            0.0,
                            0.4,
                        ));
                        let mesh = Arc::new(Mesh::cuboid(device, Vec3::splat(0.5)));
                        let render = ctx.renderer.create_render_object(mesh, material);
                        let entity = spawn_spatial(ctx.world, transform);
                        let _ = ctx.world.insert_one(entity, render);
                        self.heal_item_visuals.insert(item.item_id, entity);
                    }
                }
                ItemKind::ItemDancingCat => {
                    self.cat_item_id = Some(item.item_id);
                    if self.cat_visual.is_none() {
                        let entities = load_dancing_cat_model(ctx);
                        self.cat_visual = Some(CatVisual {
                            entities,
                            dance_phase: 0.0,
                        });
                    }

                    if !item.dancing && self.prev_cat_dancing {
                        self.cat_music_sink = None;
                        self.push_toast("THE CAT HAS STOPPED DANCING");
                    }
                    if item.dancing && !self.prev_cat_dancing {
                        self.push_toast("THE CAT IS DANCING");
                        if let Ok(sink) = ctx
                            .audio
                            .play_looped(&self.cat_jingle, self.config.music_gain() * 1.5)
                        {
                            self.cat_music_sink = Some(sink);
                        }
                    }
                    self.prev_cat_dancing = item.dancing;

                    if let Some(cat) = &self.cat_visual {
                        let held_offset = if item.held {
                            Vec3::new(0.3, -0.2, -0.5)
                        } else {
                            Vec3::ZERO
                        };
                        let base = if item.held {
                            if item.holder_id == self.local_player_id {
                                None
                            } else {
                                self.remote_players.get(&item.holder_id).map(|_| position)
                            }
                        } else {
                            Some(position)
                        };
                        if let Some(base) = base {
                            let transform = Transform::from_translation(
                                base + Vec3::new(0.0, 0.4, 0.0) + held_offset,
                            )
                            .with_scale(Vec3::splat(0.4));
                            for &entity in &cat.entities {
                                let _ = ctx.world.insert_one(entity, transform);
                            }
                        }
                    }
                }
                ItemKind::ItemNone => {}
            }
        }
    }

    fn build_pois(&mut self, ctx: &mut EngineContext, snapshot: &WorldSnapshot) {
        let device = ctx.renderer.device();
        let queue = ctx.renderer.queue();
        let material_layout = ctx.renderer.material_bind_group_layout();

        let pad_texture = GpuTexture::solid_color(device, queue, [80, 90, 110, 255]);
        let pad_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            pad_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.0,
            0.8,
        ));
        let crate_texture = GpuTexture::solid_color(device, queue, [150, 110, 70, 255]);
        let crate_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            crate_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.0,
            0.7,
        ));
        let tower_texture = GpuTexture::solid_color(device, queue, [100, 100, 110, 255]);
        let tower_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            tower_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.2,
            0.5,
        ));

        let pad_mesh = Arc::new(Mesh::cuboid(device, Vec3::new(10.0, 0.1, 10.0)));
        let crate_mesh = Arc::new(Mesh::cuboid(device, Vec3::splat(1.2)));
        let tower_mesh = Arc::new(Mesh::cuboid(device, Vec3::new(1.0, 5.0, 1.0)));

        let mut beacon_material = Material::new(
            device,
            queue,
            material_layout,
            GpuTexture::solid_color(device, queue, [255, 255, 255, 255]),
            [1.0, 1.0, 1.0, 1.0],
            0.0,
            0.5,
        );
        beacon_material.set_emissive(queue, [0.25, 0.85, 1.0]);
        let beacon_material = Arc::new(beacon_material);
        let beacon_mesh = Arc::new(Mesh::uv_sphere(device, 0.35, 12, 12));

        for poi in &snapshot.points_of_interest {
            let Some(pos) = &poi.position else { continue };
            let center = Vec3::new(pos.x, 0.0, pos.z);

            let pad_render = ctx
                .renderer
                .create_render_object(pad_mesh.clone(), pad_material.clone());
            let pad_entity = spawn_spatial(
                ctx.world,
                Transform::from_translation(center + Vec3::new(0.0, 0.05, 0.0)),
            );
            let _ = ctx.world.insert_one(pad_entity, pad_render);
            self.poi_entities.push(pad_entity);

            let tower_render = ctx
                .renderer
                .create_render_object(tower_mesh.clone(), tower_material.clone());
            let tower_entity = spawn_spatial(
                ctx.world,
                Transform::from_translation(center + Vec3::new(0.0, 2.5, 0.0)),
            );
            let _ = ctx.world.insert_one(tower_entity, tower_render);
            self.poi_entities.push(tower_entity);

            let beacon_render = ctx
                .renderer
                .create_render_object(beacon_mesh.clone(), beacon_material.clone());
            let beacon_entity = spawn_spatial(
                ctx.world,
                Transform::from_translation(center + Vec3::new(0.0, 5.3, 0.0)),
            );
            let _ = ctx.world.insert_one(beacon_entity, beacon_render);
            self.poi_entities.push(beacon_entity);

            for angle_deg in [0.0, 90.0, 180.0, 270.0] {
                let angle = angle_deg_to_rad(angle_deg);
                let offset = Vec3::new(angle.cos(), 0.0, angle.sin()) * 3.5;
                let crate_render = ctx
                    .renderer
                    .create_render_object(crate_mesh.clone(), crate_material.clone());
                let crate_entity = spawn_spatial(
                    ctx.world,
                    Transform::from_translation(center + offset + Vec3::new(0.0, 0.6, 0.0))
                        .with_rotation(Quat::from_rotation_y(angle)),
                );
                let _ = ctx.world.insert_one(crate_entity, crate_render);
                self.poi_entities.push(crate_entity);
            }
        }
    }

}
