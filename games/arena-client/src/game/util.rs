

use rf_engine::audio::Sound;
use rf_engine::math::Vec3;
use rf_engine::particles::ParticleSystem;
use rf_engine::renderer::{Camera, ParticleInstance};
use rf_engine::EngineContext;



pub(super) fn draw_pixel_icon(
    ctx: &mut EngineContext,
    pos: [f32; 2],
    cell: f32,
    pattern: &[[u8; 5]; 5],
    color: [f32; 4],
) {
    for (row, cells) in pattern.iter().enumerate() {
        for (col, &v) in cells.iter().enumerate() {
            if v == 1 {
                ctx.ui.rect(
                    [pos[0] + col as f32 * cell, pos[1] + row as f32 * cell],
                    [cell, cell],
                    color,
                    0.0,
                );
            }
        }
    }
}

pub(super) fn push_particle_batch(ctx: &mut EngineContext, system: &ParticleSystem) {
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

pub(crate) fn load_sound(path: &str, fallback: impl FnOnce() -> Sound) -> Sound {
    match Sound::load_from_file(path) {
        Ok(sound) => sound,
        Err(err) => {
            log::warn!("{path}: {err} -- using placeholder tone; see assets/sounds/README.md");
            fallback()
        }
    }
}

pub(super) fn angle_deg_to_rad(deg: f32) -> f32 {
    deg.to_radians()
}

pub(super) fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let diff =
        (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + diff * t
}

pub(super) fn player_color(id: u32) -> [u8; 4] {
    const PALETTE: [[u8; 4]; 6] = [
        [235, 120, 60, 255],
        [90, 170, 235, 255],
        [235, 90, 150, 255],
        [130, 220, 120, 255],
        [235, 200, 70, 255],
        [170, 120, 235, 255],
    ];
    PALETTE[(id as usize) % PALETTE.len()]
}

pub(super) fn powerup_color(kind: i32) -> [u8; 4] {
    match kind {
        1 => [90, 220, 255, 255],
        2 => [255, 90, 90, 255],
        _ => [90, 255, 130, 255],
    }
}

pub(super) fn world_to_screen(camera: &Camera, width: f32, height: f32, world: Vec3) -> Option<[f32; 2]> {
    let clip = camera.view_projection_matrix()
        * rf_engine::math::Vec4::new(world.x, world.y, world.z, 1.0);
    if clip.w <= 0.01 {
        return None;
    }
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    if !(-1.2..=1.2).contains(&ndc_x) || !(-1.2..=1.2).contains(&ndc_y) {
        return None;
    }
    Some([
        (ndc_x * 0.5 + 0.5) * width,
        (1.0 - (ndc_y * 0.5 + 0.5)) * height,
    ])
}

