use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::Duration;

use hecs::Entity;

use rf_engine::audio::Sound;
use rf_engine::ecs::Events;
use rf_engine::input::{GamepadAxis, GamepadButton, KeyCode, MouseButton};
use rf_engine::math::{Quat, Transform, Vec3};
use rf_engine::particles::{self, ParticleSystem};
use rf_engine::physics::{RigidBody, RigidBodyHandle, RigidBodyType};
use rf_engine::renderer::{GpuTexture, Material, Mesh, ParticleInstance};
use rf_engine::scene::spawn_spatial;
use rf_engine::terrain::{TerrainConfig, TerrainManager};
use rf_engine::weather::{WeatherEvent, WeatherSystem};
use rf_engine::{EngineContext, Game};

const MOVE_SPEED: f32 = 6.0;
const JUMP_IMPULSE: f32 = 6.5;
const PLAYER_HALF_EXTENTS: Vec3 = Vec3::new(0.5, 0.5, 0.5);
const MOUSE_SENSITIVITY: f32 = 0.0025;
const SUN_BASE_INTENSITY: f32 = 1.3;
const RAIN_BASE_RATE: f32 = 900.0;
const SNOW_BASE_RATE: f32 = 250.0;

struct Player {
    entity: Entity,
    body: RigidBodyHandle,
    was_grounded: bool,
}

enum PlayerEvent {
    Jumped,
    Landed,
}

struct Demo {
    player: Option<Player>,
    jump_sound: Sound,
    land_sound: Sound,
    thunder_sound: Sound,
    yaw: f32,
    pitch: f32,
    time_of_day: f32,
    look_locked: bool,
    terrain: Option<TerrainManager>,
    weather: WeatherSystem,
    rain: ParticleSystem,
    snow: ParticleSystem,
    dust: ParticleSystem,
    lightning_flash: f32,
    player_events: Events<PlayerEvent>,
    speed: f32,
    toasts: Vec<(String, f32)>,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            player: None,
            jump_sound: Sound::sine_wave(880.0, Duration::from_millis(140), 44100),
            land_sound: Sound::sine_wave(220.0, Duration::from_millis(90), 44100),
            thunder_sound: Sound::sine_wave(60.0, Duration::from_millis(900), 44100),
            yaw: 0.0,
            pitch: -0.15,
            time_of_day: 1.2,
            look_locked: false,
            terrain: None,
            weather: WeatherSystem::new(),
            rain: particles::rain(Vec3::ZERO),
            snow: particles::snow(Vec3::ZERO),
            dust: particles::dust_burst(Vec3::ZERO),
            lightning_flash: 0.0,
            player_events: Events::new(),
            speed: 0.0,
            toasts: Vec::new(),
        }
    }
}

impl Demo {
    fn push_toast(&mut self, text: impl Into<String>) {
        self.toasts.push((text.into(), 3.0));
    }
}

impl Game for Demo {
    fn init(&mut self, ctx: &mut EngineContext) {
        let device = ctx.renderer.device();
        let queue = ctx.renderer.queue();
        let material_layout = ctx.renderer.material_bind_group_layout();

        let mut terrain = TerrainManager::new(ctx.renderer, TerrainConfig::default());
        let spawn_height = terrain.height_at(0.0, 0.0);

        let player_texture = GpuTexture::solid_color(device, queue, [235, 120, 60, 255]);
        let player_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            player_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.1,
            0.5,
        ));
        let player_mesh = Arc::new(Mesh::cuboid(device, Vec3::splat(1.0)));
        let player_render = ctx
            .renderer
            .create_render_object(player_mesh, player_material);

        let start_transform = Transform::from_translation(Vec3::new(0.0, spawn_height + 4.0, 0.0));
        let (player_body, _collider) = ctx.physics.add_box_body(
            RigidBodyType::Dynamic,
            start_transform,
            PLAYER_HALF_EXTENTS,
            1.0,
            0.05,
        );
        ctx.physics.body_mut(player_body).lock_rotations(true, true);

        let player_entity = spawn_spatial(ctx.world, start_transform);
        let _ = ctx.world.insert_one(player_entity, player_render);
        let _ = ctx.world.insert_one(player_entity, RigidBody(player_body));

        self.player = Some(Player {
            entity: player_entity,
            body: player_body,
            was_grounded: false,
        });

        terrain.update(
            ctx.renderer,
            ctx.world,
            ctx.physics,
            start_transform.translation,
        );
        self.terrain = Some(terrain);

        let prop_texture = GpuTexture::solid_color(device, queue, [235, 220, 90, 255]);
        let prop_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            prop_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.6,
            0.2,
        ));
        let prop_mesh = Arc::new(Mesh::uv_sphere(device, 0.6, 24, 24));
        let prop_transform = Transform::from_translation(Vec3::new(3.0, spawn_height + 8.0, 3.0));
        let (prop_body, _prop_collider) = ctx.physics.add_box_body(
            RigidBodyType::Dynamic,
            prop_transform,
            Vec3::splat(0.6),
            0.4,
            0.85,
        );
        let prop_entity = spawn_spatial(ctx.world, prop_transform);
        let prop_render = ctx.renderer.create_render_object(prop_mesh, prop_material);
        let _ = ctx.world.insert_one(prop_entity, prop_render);
        let _ = ctx.world.insert_one(prop_entity, RigidBody(prop_body));

        log::info!(
            "demo scene ready: {} entities, gamepad connected: {} -- Tab toggles mouse-look",
            ctx.world.len(),
            ctx.input.gamepad.connected()
        );
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) {
        let dt = ctx.time.fixed_delta.as_secs_f32();

        if ctx.input.key_pressed(KeyCode::Tab) {
            self.look_locked = !self.look_locked;
            *ctx.cursor_locked = self.look_locked;
        }
        if ctx.input.mouse_pressed(MouseButton::Left) && !self.look_locked {
            self.look_locked = true;
            *ctx.cursor_locked = true;
        }

        if self.look_locked {
            let delta = ctx.input.mouse_delta();
            self.yaw -= delta.x * MOUSE_SENSITIVITY;
            self.pitch = (self.pitch - delta.y * MOUSE_SENSITIVITY).clamp(-1.4, 1.2);
        }
        let turn = ctx.input.gamepad.axis(GamepadAxis::RightStickX);
        self.yaw -= turn * 2.0 * dt;

        self.weather.update(dt);
        self.player_events.update();

        let Some(player) = &mut self.player else {
            return;
        };

        let mut move_axis = ctx.input.wasd_axis();
        move_axis.x += ctx.input.gamepad.axis(GamepadAxis::LeftStickX);
        move_axis.y += ctx.input.gamepad.axis(GamepadAxis::LeftStickY);
        let move_axis = move_axis.clamp_length_max(1.0);

        let yaw_rotation = Quat::from_rotation_y(self.yaw);
        let forward = yaw_rotation * Vec3::NEG_Z;
        let right = yaw_rotation * Vec3::X;
        let world_move = (right * move_axis.x + forward * move_axis.y) * MOVE_SPEED;
        ctx.physics.set_planar_velocity(player.body, world_move);
        self.speed = world_move.length();

        let grounded = ctx.physics.is_grounded(player.body, 0.65);
        let jump_pressed = ctx.input.key_pressed(KeyCode::Space)
            || ctx.input.gamepad.button_pressed(GamepadButton::South);
        if jump_pressed && grounded {
            ctx.physics
                .apply_impulse(player.body, Vec3::new(0.0, JUMP_IMPULSE, 0.0));
            self.player_events.send(PlayerEvent::Jumped);
        }
        if grounded && !player.was_grounded {
            self.player_events.send(PlayerEvent::Landed);
        }
        player.was_grounded = grounded;

        let player_pos = ctx
            .world
            .get::<&Transform>(player.entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        for event in self.player_events.iter() {
            match event {
                PlayerEvent::Jumped => {
                    if let Ok(sink) = ctx.audio.play(&self.jump_sound, 0.5) {
                        sink.detach();
                    }
                }
                PlayerEvent::Landed => {
                    if let Ok(sink) = ctx.audio.play(&self.land_sound, 0.4) {
                        sink.detach();
                    }
                    self.dust.origin = player_pos;
                    self.dust.burst(14);
                }
            }
        }

        let mut new_toasts = Vec::new();
        for event in self.weather.events.iter() {
            match event {
                WeatherEvent::Lightning { strength } => {
                    self.lightning_flash = *strength;
                    if let Ok(sink) = ctx.audio.play(&self.thunder_sound, 0.6) {
                        sink.detach();
                    }
                    new_toasts.push("LIGHTNING STRIKE".to_string());
                }
                WeatherEvent::Changed { to, .. } => {
                    new_toasts.push(format!("WEATHER: {to:?}"));
                }
            }
        }
        for toast in new_toasts {
            self.push_toast(toast);
        }

        for (_, remaining) in &mut self.toasts {
            *remaining -= dt;
        }
        self.toasts.retain(|(_, remaining)| *remaining > 0.0);

        if let Some(terrain) = &mut self.terrain {
            terrain.update(ctx.renderer, ctx.world, ctx.physics, player_pos);
        }

        let params = self.weather.params();
        let precip_origin = player_pos + Vec3::new(0.0, 18.0, 0.0);
        self.rain.origin = precip_origin;
        self.snow.origin = precip_origin;
        self.weather.drive_rain(&mut self.rain, RAIN_BASE_RATE);
        self.weather.drive_snow(&mut self.snow, SNOW_BASE_RATE);

        let gravity = Vec3::new(0.0, -9.81, 0.0);
        self.rain.update(dt, gravity);
        self.snow.update(dt, gravity);
        self.dust.update(dt, gravity * 0.3);

        ctx.light.intensity =
            SUN_BASE_INTENSITY * params.ambient_dimming + self.lightning_flash * 2.5;
        self.lightning_flash = (self.lightning_flash - dt * 2.5).max(0.0);

        ctx.fog.density = params.fog_density;
        ctx.fog.color = if params.storm_darkness > 0.3 {
            [0.35, 0.37, 0.4]
        } else {
            [0.7, 0.75, 0.8]
        };
    }

    fn update(&mut self, ctx: &mut EngineContext) {
        let Some(player) = &self.player else { return };
        let player_pos = ctx
            .world
            .get::<&Transform>(player.entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        let orbit = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
        let camera_offset = orbit * Vec3::new(0.0, 3.5, 9.0);
        ctx.camera.position = player_pos + camera_offset;
        ctx.camera.target = player_pos + Vec3::new(0.0, 1.0, 0.0);

        self.time_of_day = (self.time_of_day + ctx.time.delta_seconds() * 0.05) % TAU;
        let sun_dir = Vec3::new(self.time_of_day.cos(), self.time_of_day.sin(), 0.25).normalize();
        ctx.light.direction = -sun_dir;

        let params = self.weather.params();
        ctx.sky.cloud_coverage = params.cloud_coverage;
        ctx.sky.storm_darkness = (params.storm_darkness + self.lightning_flash * 0.5).min(1.0);
        ctx.sky.star_brightness = 1.0;

        push_particle_batch(ctx, &self.rain);
        push_particle_batch(ctx, &self.snow);
        push_particle_batch(ctx, &self.dust);

        self.draw_hud(ctx);
    }
}

impl Demo {
    fn draw_hud(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;

        const ACCENT: [f32; 4] = [0.35, 0.95, 1.0, 1.0];
        const PANEL: [f32; 4] = [0.04, 0.06, 0.08, 0.55];
        const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.12];
        const TEXT: [f32; 4] = [0.92, 0.97, 1.0, 0.95];
        const MUTED: [f32; 4] = [0.75, 0.85, 0.9, 0.55];

        let center = [w * 0.5, h * 0.5];
        let gap = 5.0 + (self.speed * 0.6).min(6.0);
        let tick = 7.0;
        let thickness = 2.0;
        let crosshair_color = [ACCENT[0], ACCENT[1], ACCENT[2], 0.85];
        ctx.ui.rect(
            [center[0] - thickness / 2.0, center[1] - gap - tick],
            [thickness, tick],
            crosshair_color,
            1.0,
        );
        ctx.ui.rect(
            [center[0] - thickness / 2.0, center[1] + gap],
            [thickness, tick],
            crosshair_color,
            1.0,
        );
        ctx.ui.rect(
            [center[0] - gap - tick, center[1] - thickness / 2.0],
            [tick, thickness],
            crosshair_color,
            1.0,
        );
        ctx.ui.rect(
            [center[0] + gap, center[1] - thickness / 2.0],
            [tick, thickness],
            crosshair_color,
            1.0,
        );

        let panel_pos = [24.0, h - 92.0];
        let panel_size = [220.0, 68.0];
        ctx.ui.rect(panel_pos, panel_size, PANEL, 10.0);

        ctx.ui.text(
            [panel_pos[0] + 14.0, panel_pos[1] + 10.0],
            "SPEED",
            12.0,
            MUTED,
        );
        let speed_fraction = (self.speed / MOVE_SPEED).clamp(0.0, 1.0);
        ctx.ui.bar(
            [panel_pos[0] + 14.0, panel_pos[1] + 26.0],
            [panel_size[0] - 28.0, 8.0],
            speed_fraction,
            TRACK,
            ACCENT,
            4.0,
        );

        let weather = self.weather.current();
        let weather_color = match weather {
            rf_engine::weather::WeatherKind::Clear => [0.4, 0.9, 1.0, 1.0],
            rf_engine::weather::WeatherKind::Cloudy => [0.75, 0.8, 0.85, 1.0],
            rf_engine::weather::WeatherKind::Rain => [0.35, 0.6, 1.0, 1.0],
            rf_engine::weather::WeatherKind::Snow => [0.9, 0.95, 1.0, 1.0],
            rf_engine::weather::WeatherKind::Storm => [1.0, 0.45, 0.3, 1.0],
        };
        ctx.ui.rect(
            [panel_pos[0] + 14.0, panel_pos[1] + 46.0],
            [8.0, 8.0],
            weather_color,
            4.0,
        );
        ctx.ui.text(
            [panel_pos[0] + 30.0, panel_pos[1] + 44.0],
            format!("{weather:?}"),
            12.0,
            TEXT,
        );

        let compass_width = 260.0;
        let compass_pos = [w * 0.5 - compass_width * 0.5, 20.0];
        ctx.ui.rect(compass_pos, [compass_width, 22.0], PANEL, 6.0);
        let heading_deg = self.yaw.to_degrees().rem_euclid(360.0);
        for (deg, label) in [(0.0, "N"), (90.0, "E"), (180.0, "S"), (270.0, "W")] {
            let mut delta = deg - heading_deg;
            delta = ((delta + 180.0).rem_euclid(360.0)) - 180.0;
            let pixels_per_degree = compass_width / 180.0;
            let x = compass_pos[0] + compass_width * 0.5 + delta * pixels_per_degree;
            if x < compass_pos[0] || x > compass_pos[0] + compass_width {
                continue;
            }
            let color = if label == "N" { ACCENT } else { MUTED };
            ctx.ui
                .text([x - 3.0, compass_pos[1] + 5.0], label, 12.0, color);
        }
        ctx.ui.rect(
            [w * 0.5 - 1.0, compass_pos[1] + 22.0],
            [2.0, 6.0],
            ACCENT,
            0.0,
        );

        let fps = if ctx.time.delta_seconds() > 0.0001 {
            1.0 / ctx.time.delta_seconds()
        } else {
            0.0
        };
        let chunk_count = self
            .terrain
            .as_ref()
            .map(|t| t.loaded_chunk_count())
            .unwrap_or(0);
        let debug_text = format!("{:.0} FPS / {} CHUNKS", fps, chunk_count);
        let debug_width = rf_engine::ui::Canvas::text_width(&debug_text, 12.0);
        ctx.ui
            .text([w - debug_width - 20.0, 20.0], debug_text, 12.0, MUTED);

        let mut toast_y = h * 0.32;
        for (text, remaining) in self.toasts.iter().rev() {
            let alpha = (*remaining / 3.0).min(1.0);
            let width = rf_engine::ui::Canvas::text_width(text, 16.0) + 28.0;
            let x = w * 0.5 - width * 0.5;
            ctx.ui.rect(
                [x, toast_y],
                [width, 30.0],
                [PANEL[0], PANEL[1], PANEL[2], PANEL[3] * alpha],
                8.0,
            );
            ctx.ui.text(
                [x + 14.0, toast_y + 8.0],
                text.clone(),
                16.0,
                [TEXT[0], TEXT[1], TEXT[2], alpha],
            );
            toast_y += 38.0;
        }
    }
}

fn push_particle_batch(ctx: &mut EngineContext, system: &ParticleSystem) {
    if system.is_empty() {
        return;
    }
    let batch: Vec<ParticleInstance> = system
        .particles()
        .iter()
        .map(|p| ParticleInstance {
            world_position: p.position.to_array(),
            size: p.size,
            color: system.current_color(p),
        })
        .collect();
    ctx.particle_batches.push(batch);
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    rf_engine::run("RustForge Engine Demo", Demo::default())
}
