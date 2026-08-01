use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use hecs::Entity;

use rf_engine::audio::Sound;
use rf_engine::input::{KeyCode, MouseButton};
use rf_engine::math::{Quat, Transform, Vec3};
use rf_engine::particles::{self, ParticleSystem};
use rf_engine::renderer::{GpuTexture, Material, Mesh};
use rf_engine::scene::spawn_spatial;
use rf_engine::EngineContext;
use rf_net::{
    ItemKind, MatchPhase, NetworkClient, PlayerInput, PlayerState,
    WorldSnapshot,
};

use crate::config::AppConfig;
use crate::settings::SettingsPanel;
use crate::Screen;

const ARENA_HALF_EXTENT: f32 = 30.0;
const MOUSE_SENSITIVITY: f32 = 0.0025;
const RIFLE_OFFSET: Vec3 = Vec3::new(0.35, -0.3, -0.6);
const CAT_HELD_OFFSET: Vec3 = Vec3::new(-0.4, -0.25, -0.55);
const RIGHT_WEAPON_OFFSET: Vec3 = Vec3::new(0.55, 0.0, 0.05);
const DEATH_ANIM_DURATION: f32 = 0.8;
const CAT_DANCE_SPIN_SPEED: f32 = 6.0;
const CAT_DANCE_JUMP_HEIGHT: f32 = 0.35;
const CAT_DANCE_JUMP_SPEED: f32 = 10.0;
const FREE_CAM_SPEED: f32 = 9.0;
const ESCAPE_OPTIONS: [&str; 4] = ["RESUME", "SETTINGS", "DISCONNECT", "EXIT GAME"];

const DASH_COOLDOWN_MAX: f32 = 4.0;
const SUPERSPEED_COOLDOWN_MAX: f32 = 14.0;
const SWAP_COOLDOWN_MAX: f32 = 10.0;
const ULTIMATE_COOLDOWN_MAX: f32 = 45.0;

const DASH_ICON: [[u8; 5]; 5] = [
    [0, 0, 1, 1, 0],
    [0, 1, 1, 0, 0],
    [1, 1, 1, 1, 1],
    [0, 0, 1, 1, 0],
    [0, 1, 1, 0, 0],
];
const SUPERSPEED_ICON: [[u8; 5]; 5] = [
    [1, 0, 1, 0, 0],
    [1, 1, 0, 1, 0],
    [1, 1, 1, 0, 1],
    [1, 1, 0, 1, 0],
    [1, 0, 1, 0, 0],
];
const SWAP_ICON: [[u8; 5]; 5] = [
    [1, 0, 0, 0, 1],
    [0, 1, 0, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 1, 0, 1, 0],
    [1, 0, 0, 0, 1],
];
const ULTIMATE_ICON: [[u8; 5]; 5] = [
    [0, 0, 1, 0, 0],
    [1, 0, 1, 0, 1],
    [0, 1, 1, 1, 0],
    [1, 0, 1, 0, 1],
    [0, 0, 1, 0, 0],
];

struct RemotePlayer {
    entity: Entity,
    weapon_entity: Entity,
}

struct CatVisual {
    entities: Vec<Entity>,
    dance_phase: f32,
}

pub struct PlayingScreen {
    network: NetworkClient,
    local_player_id: u32,
    yaw: f32,
    pitch: f32,
    sequence: u32,
    rifle_entities: Vec<Entity>,
    weapon_prop_mesh: Arc<Mesh>,
    weapon_prop_material: Arc<Material>,
    remote_players: HashMap<u32, RemotePlayer>,
    powerup_visuals: HashMap<u32, Entity>,
    heal_item_visuals: HashMap<u32, Entity>,
    prev_heal_active: HashMap<u32, bool>,
    cat_item_id: Option<u32>,
    cat_visual: Option<CatVisual>,
    poi_built: bool,
    poi_entities: Vec<Entity>,
    death_anim: HashMap<u32, f32>,
    latest_snapshot: Option<WorldSnapshot>,
    prev_snapshot: Option<WorldSnapshot>,
    snapshot_interval: f32,
    snapshot_received_at: f32,
    prev_alive: bool,
    prev_kills: u32,
    prev_cat_dancing: bool,
    toasts: Vec<(String, f32)>,
    free_cam_pos: Vec3,
    free_cam_initialized: bool,

    pending_jump: bool,
    pending_toggle_ready: bool,
    pending_use_item: bool,
    pending_ability_dash: bool,
    pending_ability_superspeed: bool,
    pending_ability_swap: bool,
    pending_ability_ultimate: bool,
    pending_fire_click: bool,
    config: AppConfig,
    escape_open: bool,
    escape_selected: usize,
    settings_open: bool,
    settings_panel: SettingsPanel,
    muzzle_particles: ParticleSystem,
    death_particles: ParticleSystem,
    rain_particles: ParticleSystem,
    current_cloud_coverage: f32,
    current_storm_darkness: f32,
    current_rain_intensity: f32,
    shoot_sound: Sound,
    hit_marker_sound: Sound,
    death_sound: Sound,
    pickup_sound: Sound,
    siren_sound: Sound,
    thunder_sound: Sound,
    ui_tick_sound: Sound,
    dash_sound: Sound,
    superspeed_sound: Sound,
    swap_sound: Sound,
    ultimate_sound: Sound,
    ui_confirm_sound: Sound,
    music_sink: rf_engine::audio::Sink,
    lobby_music: Sound,
    arena_music: Sound,
    music_is_arena: bool,
    cat_music_sink: Option<rf_engine::audio::Sink>,
    cat_jingle: Sound,
    disconnected_notice_shown: bool,
    address: String,
    player_name: String,
}

mod animation;
mod assets;
mod events;
mod hud;
mod network;
mod util;

use assets::load_rifle_view_model;
pub(crate) use util::load_sound;
use util::push_particle_batch;

impl PlayingScreen {
    pub fn new(
        ctx: &mut EngineContext,
        network: NetworkClient,
        address: String,
        player_name: String,
    ) -> Self {
        let config = AppConfig::load();
        let device = ctx.renderer.device();
        let queue = ctx.renderer.queue();
        let material_layout = ctx.renderer.material_bind_group_layout();

        let floor_texture = GpuTexture::checkerboard(device, queue, 256, 16);
        let floor_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            floor_texture,
            [0.6, 0.6, 0.65, 1.0],
            0.0,
            0.9,
        ));
        let floor_mesh = Arc::new(Mesh::plane(device, ARENA_HALF_EXTENT * 2.0, 6.0));
        let floor_render = ctx
            .renderer
            .create_render_object(floor_mesh, floor_material);
        let floor_entity = spawn_spatial(ctx.world, Transform::IDENTITY);
        let _ = ctx.world.insert_one(floor_entity, floor_render);

        let wall_texture = GpuTexture::solid_color(device, queue, [60, 70, 90, 255]);
        let wall_material = Arc::new(Material::new(
            device,
            queue,
            material_layout,
            wall_texture,
            [1.0, 1.0, 1.0, 1.0],
            0.0,
            0.6,
        ));
        let wall_height = 4.0;
        let wall_thickness = 1.0;
        let span = ARENA_HALF_EXTENT * 2.0;
        let wall_mesh_long = Arc::new(Mesh::cuboid(
            device,
            Vec3::new(span, wall_height, wall_thickness),
        ));
        for (x, z, rot) in [
            (0.0, ARENA_HALF_EXTENT, 0.0),
            (0.0, -ARENA_HALF_EXTENT, 0.0),
            (ARENA_HALF_EXTENT, 0.0, std::f32::consts::FRAC_PI_2),
            (-ARENA_HALF_EXTENT, 0.0, std::f32::consts::FRAC_PI_2),
        ] {
            let transform = Transform::from_translation(Vec3::new(x, wall_height * 0.5, z))
                .with_rotation(Quat::from_rotation_y(rot));
            let render = ctx
                .renderer
                .create_render_object(wall_mesh_long.clone(), wall_material.clone());
            let entity = spawn_spatial(ctx.world, transform);
            let _ = ctx.world.insert_one(entity, render);
        }

        let rifle_entities = load_rifle_view_model(ctx);

        let weapon_prop_mesh = Arc::new(Mesh::cuboid(
            ctx.renderer.device(),
            Vec3::new(0.15, 0.15, 0.7),
        ));
        let weapon_prop_material = Arc::new(Material::new(
            ctx.renderer.device(),
            ctx.renderer.queue(),
            ctx.renderer.material_bind_group_layout(),
            GpuTexture::solid_color(
                ctx.renderer.device(),
                ctx.renderer.queue(),
                [35, 35, 40, 255],
            ),
            [1.0, 1.0, 1.0, 1.0],
            0.3,
            0.5,
        ));

        let lobby_music = load_sound("assets/music/lobby_theme.mp3", || {
            Sound::chiptune_loop(44100)
        });

        Self {
            network,
            local_player_id: 0,
            yaw: 0.0,
            pitch: 0.0,
            sequence: 0,
            rifle_entities,
            weapon_prop_mesh,
            weapon_prop_material,
            remote_players: HashMap::new(),
            powerup_visuals: HashMap::new(),
            heal_item_visuals: HashMap::new(),
            prev_heal_active: HashMap::new(),
            cat_item_id: None,
            cat_visual: None,
            poi_built: false,
            poi_entities: Vec::new(),
            death_anim: HashMap::new(),
            latest_snapshot: None,
            prev_snapshot: None,
            snapshot_interval: 1.0 / 20.0,
            snapshot_received_at: 0.0,
            prev_alive: true,
            prev_kills: 0,
            prev_cat_dancing: false,
            toasts: Vec::new(),
            free_cam_pos: Vec3::new(0.0, 1.5, 0.0),
            free_cam_initialized: false,
            pending_jump: false,
            pending_toggle_ready: false,
            pending_use_item: false,
            pending_ability_dash: false,
            pending_ability_superspeed: false,
            pending_ability_swap: false,
            pending_ability_ultimate: false,
            pending_fire_click: false,
            escape_open: false,
            escape_selected: 0,
            settings_open: false,
            settings_panel: SettingsPanel::new(),
            muzzle_particles: particles::sparks(Vec3::ZERO),
            death_particles: particles::dust_burst(Vec3::ZERO),
            rain_particles: particles::rain(Vec3::ZERO),
            current_cloud_coverage: 0.1,
            current_storm_darkness: 0.0,
            current_rain_intensity: 0.0,
            shoot_sound: load_sound("assets/sounds/weapon_fire.mp3", || {
                Sound::sine_wave(1400.0, Duration::from_millis(45), 44100)
            }),
            hit_marker_sound: load_sound("assets/sounds/hit_marker.mp3", || {
                Sound::sine_wave(1900.0, Duration::from_millis(70), 44100)
            }),
            death_sound: load_sound("assets/sounds/player_death.mp3", || {
                Sound::sine_wave(140.0, Duration::from_millis(400), 44100)
            }),
            pickup_sound: load_sound("assets/sounds/item_pickup.mp3", || {
                Sound::sine_wave(1100.0, Duration::from_millis(120), 44100)
            }),
            siren_sound: load_sound("assets/sounds/orbital_strike_warning.mp3", || {
                Sound::sine_wave(700.0, Duration::from_millis(600), 44100)
            }),
            thunder_sound: load_sound("assets/sounds/orbital_strike_impact.mp3", || {
                Sound::sine_wave(55.0, Duration::from_millis(900), 44100)
            }),
            ui_tick_sound: load_sound("assets/sounds/ui_tick.mp3", || {
                Sound::sine_wave(900.0, Duration::from_millis(35), 44100)
            }),
            dash_sound: load_sound("assets/sounds/ability_dash.mp3", || {
                Sound::sine_wave(2200.0, Duration::from_millis(90), 44100)
            }),
            superspeed_sound: load_sound("assets/sounds/ability_superspeed.mp3", || {
                Sound::sine_wave(1600.0, Duration::from_millis(220), 44100)
            }),
            swap_sound: load_sound("assets/sounds/ability_swap.mp3", || {
                Sound::sine_wave(2600.0, Duration::from_millis(140), 44100)
            }),
            ultimate_sound: load_sound("assets/sounds/ability_ultimate.mp3", || {
                Sound::sine_wave(50.0, Duration::from_millis(1400), 44100)
            }),
            ui_confirm_sound: load_sound("assets/sounds/ui_confirm.mp3", || {
                Sound::sine_wave(1300.0, Duration::from_millis(150), 44100)
            }),
            music_sink: ctx
                .audio
                .play_looped(&lobby_music, config.music_gain())
                .expect("start background music"),
            lobby_music,
            arena_music: load_sound("assets/music/arena_theme.mp3", || {
                Sound::chiptune_loop(44100)
            }),
            music_is_arena: false,
            cat_music_sink: None,
            cat_jingle: load_sound("assets/sounds/cat_jingle.mp3", || {
                Sound::chiptune_loop(44100)
            }),
            disconnected_notice_shown: false,
            address,
            player_name,
            config,
        }
    }

    fn phase(&self) -> MatchPhase {
        self.latest_snapshot
            .as_ref()
            .and_then(|s| MatchPhase::try_from(s.phase).ok())
            .unwrap_or(MatchPhase::PhaseLobby)
    }

    fn local_player_state(&self) -> Option<&PlayerState> {
        self.latest_snapshot.as_ref().and_then(|s| {
            s.players
                .iter()
                .find(|p| p.player_id == self.local_player_id)
        })
    }

    fn local_alive(&self) -> bool {
        self.local_player_state().map(|p| p.alive).unwrap_or(true)
    }

    pub fn sync_discord_presence(&self, discord: &mut crate::discord::DiscordPresence) {
        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };
        let total = snapshot.players.len() as u32;
        match self.phase() {
            MatchPhase::PhaseLobby | MatchPhase::PhaseStarting => {
                let ready = snapshot.players.iter().filter(|p| p.ready).count() as u32;
                discord.set_lobby(ready, total);
            }
            MatchPhase::PhasePlaying => {
                let (kills, deaths) = self
                    .local_player_state()
                    .map(|p| (p.kills, p.deaths))
                    .unwrap_or((0, 0));
                let alive = snapshot.players.iter().filter(|p| p.alive).count() as u32;
                discord.set_playing(kills, deaths, alive, total);
            }
            MatchPhase::PhaseEnding => {
                let won = snapshot.winner_player_id == self.local_player_id;
                let kills = self.local_player_state().map(|p| p.kills).unwrap_or(0);
                discord.set_ending(won, kills);
            }
        }
    }

    pub fn fixed_update(&mut self, ctx: &mut EngineContext) -> Option<Screen> {
        self.local_player_id = self.network.player_id;

        let overlay_open = self.escape_open || self.settings_open;
        let phase = self.phase();
        let playing = phase == MatchPhase::PhasePlaying;
        let local_alive = self.local_alive();

        if !overlay_open {
            let mouse_delta = ctx.input.mouse_delta();
            self.yaw -= mouse_delta.x * MOUSE_SENSITIVITY;
            self.pitch = (self.pitch - mouse_delta.y * MOUSE_SENSITIVITY).clamp(-1.4, 1.4);
        }

        *ctx.cursor_locked = playing && !overlay_open;

        let jump_pressed = std::mem::take(&mut self.pending_jump);
        let toggle_ready_pressed = std::mem::take(&mut self.pending_toggle_ready);
        let use_item_pressed = std::mem::take(&mut self.pending_use_item);
        let ability_dash_pressed = std::mem::take(&mut self.pending_ability_dash);
        let ability_superspeed_pressed = std::mem::take(&mut self.pending_ability_superspeed);
        let ability_swap_pressed = std::mem::take(&mut self.pending_ability_swap);
        let ability_ultimate_pressed = std::mem::take(&mut self.pending_ability_ultimate);
        let fire_click_pressed = std::mem::take(&mut self.pending_fire_click);

        self.sequence += 1;
        if playing && local_alive && !overlay_open {
            let move_axis = ctx.input.wasd_axis();
            self.network.send_input(PlayerInput {
                player_id: self.local_player_id,
                move_x: move_axis.x,
                move_y: move_axis.y,
                yaw: self.yaw,
                pitch: self.pitch,
                jump: jump_pressed,
                fire: ctx.input.mouse_held(MouseButton::Left),
                sequence: self.sequence,
                toggle_ready: toggle_ready_pressed,
                use_item: use_item_pressed,
                ability_dash: ability_dash_pressed,
                ability_superspeed: ability_superspeed_pressed,
                ability_swap: ability_swap_pressed,
                ability_ultimate: ability_ultimate_pressed,
                ..Default::default()
            });
            if fire_click_pressed {
                self.fire_muzzle_burst();
                if let Ok(sink) = ctx.audio.play(&self.shoot_sound, self.config.sfx_gain()) {
                    sink.detach();
                }
            }
        } else {
            self.network.send_input(PlayerInput {
                player_id: self.local_player_id,
                move_x: 0.0,
                move_y: 0.0,
                yaw: self.yaw,
                pitch: self.pitch,
                jump: false,
                fire: false,
                sequence: self.sequence,
                toggle_ready: !overlay_open
                    && phase == MatchPhase::PhaseLobby
                    && toggle_ready_pressed,
                use_item: false,
                ..Default::default()
            });
        }

        if playing && !local_alive && !overlay_open {
            let look_rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
            let forward = look_rotation * Vec3::NEG_Z;
            let right = look_rotation * Vec3::X;
            let move_axis = ctx.input.wasd_axis();
            let vertical = if ctx.input.key_held(KeyCode::Space) {
                1.0
            } else if ctx.input.key_held(KeyCode::ShiftLeft) {
                -1.0
            } else {
                0.0
            };
            let dt = ctx.time.fixed_delta.as_secs_f32();
            self.free_cam_pos += (forward * move_axis.y + right * move_axis.x + Vec3::Y * vertical)
                * FREE_CAM_SPEED
                * dt;
        }

        if let Some(snapshot) = self.network.poll_latest_snapshot() {
            let now = ctx.time.elapsed_seconds();
            if self.latest_snapshot.is_some() {
                let gap = now - self.snapshot_received_at;
                if gap > 0.001 {
                    self.snapshot_interval = gap.clamp(0.01, 0.5);
                }
            }
            self.snapshot_received_at = now;
            self.prev_snapshot = self.latest_snapshot.take();
            self.apply_snapshot(ctx, &snapshot);
            self.latest_snapshot = Some(snapshot);
        }

        for (_, remaining) in &mut self.toasts {
            *remaining -= ctx.time.fixed_delta.as_secs_f32();
        }
        self.toasts.retain(|(_, remaining)| *remaining > 0.0);

        for timer in self.death_anim.values_mut() {
            *timer += ctx.time.fixed_delta.as_secs_f32();
        }
        self.death_anim
            .retain(|_, t| *t < DEATH_ANIM_DURATION * 2.0);

        let gravity = Vec3::new(0.0, -9.81, 0.0);
        self.muzzle_particles
            .update(ctx.time.fixed_delta.as_secs_f32(), gravity);
        self.death_particles
            .update(ctx.time.fixed_delta.as_secs_f32(), gravity * 0.3);

        let precip_origin = self
            .local_player_state()
            .and_then(|p| p.position)
            .map(|p| Vec3::new(p.x, p.y + 18.0, p.z))
            .unwrap_or(Vec3::new(0.0, 18.0, 0.0));
        self.rain_particles.origin = precip_origin;
        self.rain_particles.config.rate = 700.0 * self.current_rain_intensity;
        self.rain_particles.enabled = self.current_rain_intensity > 0.02;
        self.rain_particles
            .update(ctx.time.fixed_delta.as_secs_f32(), gravity * 0.4);

        if self.network.is_disconnected() && !self.disconnected_notice_shown {
            self.disconnected_notice_shown = true;
            self.push_toast("DISCONNECTED -- RECONNECTING");
            return Some(Screen::Connecting(crate::Connecting {
                rx: NetworkClient::connect(self.address.clone(), self.player_name.clone()),
                address: self.address.clone(),
                player_name: self.player_name.clone(),
                elapsed: 0.0,
            }));
        }

        None
    }

    pub fn update(&mut self, ctx: &mut EngineContext) -> Option<Screen> {
        self.pending_jump |= ctx.input.key_pressed(KeyCode::Space);
        self.pending_toggle_ready |= ctx.input.key_pressed(KeyCode::KeyR);
        self.pending_use_item |= ctx.input.key_pressed(KeyCode::KeyE);
        self.pending_ability_dash |= ctx.input.key_pressed(KeyCode::Digit1);
        self.pending_ability_superspeed |= ctx.input.key_pressed(KeyCode::Digit2);
        self.pending_ability_swap |= ctx.input.key_pressed(KeyCode::Digit3);
        self.pending_ability_ultimate |= ctx.input.key_pressed(KeyCode::Digit4);
        self.pending_fire_click |= ctx.input.mouse_pressed(MouseButton::Left);

        let phase = self.phase();
        let playing = phase == MatchPhase::PhasePlaying;
        let local_alive = self.local_alive();

        let should_play_arena_music = playing;
        if should_play_arena_music != self.music_is_arena {
            self.music_is_arena = should_play_arena_music;
            let track = if should_play_arena_music {
                &self.arena_music
            } else {
                &self.lobby_music
            };
            if let Ok(sink) = ctx.audio.play_looped(track, self.config.music_gain()) {
                self.music_sink = sink;
            }
        }

        let look_rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
        let eye = if playing {
            if local_alive {
                self.free_cam_initialized = false;
                let local_position = self
                    .local_player_state()
                    .and_then(|p| p.position)
                    .map(|p| Vec3::new(p.x, p.y, p.z))
                    .unwrap_or(Vec3::new(0.0, 1.0, 0.0));
                let eye = local_position + Vec3::new(0.0, 0.9, 0.0);
                ctx.camera.position = eye;
                ctx.camera.target = eye + look_rotation * Vec3::NEG_Z;
                eye
            } else {
                if !self.free_cam_initialized {
                    self.free_cam_initialized = true;
                    self.free_cam_pos = self
                        .local_player_state()
                        .and_then(|p| p.position)
                        .map(|p| Vec3::new(p.x, p.y + 2.0, p.z))
                        .unwrap_or(self.free_cam_pos);
                }
                ctx.camera.position = self.free_cam_pos;
                ctx.camera.target = self.free_cam_pos + look_rotation * Vec3::NEG_Z;
                self.free_cam_pos
            }
        } else {
            self.free_cam_initialized = false;

            let t = ctx.time.elapsed_seconds() * 0.06;
            let radius = 26.0;
            ctx.camera.position = Vec3::new(t.cos() * radius, 20.0, t.sin() * radius);
            ctx.camera.target = Vec3::ZERO;
            Vec3::ZERO
        };

        let show_held = playing && local_alive;
        let rifle_transform = Transform::from_translation(eye + look_rotation * RIFLE_OFFSET)
            .with_rotation(look_rotation)
            .with_scale(if show_held {
                Vec3::splat(0.6)
            } else {
                Vec3::ZERO
            });
        for &entity in &self.rifle_entities {
            let _ = ctx.world.insert_one(entity, rifle_transform);
        }

        let local_held_item = self
            .local_player_state()
            .map(|p| ItemKind::try_from(p.held_item).unwrap_or(ItemKind::ItemNone))
            .unwrap_or(ItemKind::ItemNone);
        if show_held && local_held_item == ItemKind::ItemDancingCat {
            if let Some(cat) = &self.cat_visual {
                let transform = Transform::from_translation(eye + look_rotation * CAT_HELD_OFFSET)
                    .with_rotation(look_rotation)
                    .with_scale(Vec3::splat(0.3));
                for &entity in &cat.entities {
                    let _ = ctx.world.insert_one(entity, transform);
                }
            }
        }

        self.interpolate_remote_players(ctx);
        self.animate_deaths(ctx);
        self.animate_cat_dance(ctx);
        self.update_weather_and_lighting(ctx);
        push_particle_batch(ctx, &self.muzzle_particles);
        push_particle_batch(ctx, &self.death_particles);
        push_particle_batch(ctx, &self.rain_particles);

        match phase {
            MatchPhase::PhaseLobby => self.draw_lobby(ctx),
            MatchPhase::PhaseStarting => self.draw_starting(ctx),
            MatchPhase::PhasePlaying => self.draw_hud(ctx),
            MatchPhase::PhaseEnding => self.draw_ending(ctx),
        }

        let mut transition = None;
        if self.settings_open {
            if self.settings_panel.update(ctx, &mut self.config) {
                self.settings_open = false;
            }
        } else if self.escape_open {
            transition = self.update_escape_menu(ctx);
        } else if ctx.input.key_pressed(KeyCode::Escape) {
            self.escape_open = true;
        }
        transition
    }

    fn fire_muzzle_burst(&mut self) {
        let look_rotation = Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
        self.muzzle_particles.origin = self
            .local_player_state()
            .and_then(|p| p.position)
            .map(|p| Vec3::new(p.x, p.y + 1.5, p.z))
            .unwrap_or(Vec3::ZERO)
            + look_rotation * Vec3::NEG_Z * 0.8;
        self.muzzle_particles.config.direction = look_rotation * Vec3::NEG_Z;
        self.muzzle_particles.burst(6);
    }

    fn update_escape_menu(&mut self, ctx: &mut EngineContext) -> Option<Screen> {
        if ctx.input.key_pressed(KeyCode::ArrowDown) {
            self.escape_selected = (self.escape_selected + 1) % ESCAPE_OPTIONS.len();
            if let Ok(sink) = ctx.audio.play(&self.ui_tick_sound, self.config.sfx_gain()) {
                sink.detach();
            }
        }
        if ctx.input.key_pressed(KeyCode::ArrowUp) {
            self.escape_selected =
                (self.escape_selected + ESCAPE_OPTIONS.len() - 1) % ESCAPE_OPTIONS.len();
            if let Ok(sink) = ctx.audio.play(&self.ui_tick_sound, self.config.sfx_gain()) {
                sink.detach();
            }
        }

        let mut transition = None;
        if ctx.input.key_pressed(KeyCode::Enter) || ctx.input.key_pressed(KeyCode::NumpadEnter) {
            if let Ok(sink) = ctx
                .audio
                .play(&self.ui_confirm_sound, self.config.sfx_gain())
            {
                sink.detach();
            }
            match self.escape_selected {
                0 => self.escape_open = false,
                1 => self.settings_open = true,
                2 => transition = Some(Screen::Menu(crate::menu::MenuScreen::new())),
                3 => std::process::exit(0),
                _ => {}
            }
        }
        if ctx.input.key_pressed(KeyCode::Escape) {
            self.escape_open = false;
        }

        self.draw_escape_menu(ctx);
        transition
    }

}
