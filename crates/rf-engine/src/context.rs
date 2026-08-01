use rf_audio::AudioEngine;
use rf_ecs::{Time, World};
use rf_input::InputState;
use rf_physics::PhysicsWorld;
use rf_renderer::{Camera, DirectionalLight, FogSettings, ParticleInstance, Renderer, SkySettings};
use rf_ui::Canvas;

pub struct EngineContext<'a> {
    pub world: &'a mut World,
    pub physics: &'a mut PhysicsWorld,
    pub input: &'a InputState,
    pub audio: &'a AudioEngine,
    pub time: &'a Time,
    pub renderer: &'a Renderer,
    pub camera: &'a mut Camera,
    pub light: &'a mut DirectionalLight,
    pub sky: &'a mut SkySettings,
    pub fog: &'a mut FogSettings,

    pub cursor_locked: &'a mut bool,

    pub particle_batches: &'a mut Vec<Vec<ParticleInstance>>,

    pub ui: &'a mut Canvas,
}
