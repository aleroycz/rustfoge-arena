use std::sync::Arc;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use rf_audio::AudioEngine;
use rf_ecs::{Time, World};
use rf_physics::PhysicsWorld;
use rf_renderer::{Camera, DirectionalLight, RenderObject, Renderer, TerrainRenderObject};
use rf_ui::Canvas;

use crate::{EngineContext, Game};

pub fn run<G: Game>(window_title: &str, game: G) -> Result<()> {
    run_with_icon(window_title, None, game)
}

pub fn run_with_icon<G: Game>(window_title: &str, icon_png: Option<&[u8]>, game: G) -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let icon = icon_png.and_then(|bytes| match image::load_from_memory(bytes) {
        Ok(image) => {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();
            match winit::window::Icon::from_rgba(rgba.into_raw(), width, height) {
                Ok(icon) => Some(icon),
                Err(err) => {
                    log::warn!("failed to build window icon: {err}");
                    None
                }
            }
        }
        Err(err) => {
            log::warn!("failed to decode window icon: {err}");
            None
        }
    });

    let mut app = App {
        window_title: window_title.to_string(),
        window_icon: icon,
        window: None,
        renderer: None,
        audio: None,
        game,
        world: World::new(),
        time: Time::new(60.0),
        input: rf_input::InputState::new(),
        physics: PhysicsWorld::default(),
        camera: Camera::default(),
        light: DirectionalLight::default(),
        sky: rf_renderer::SkySettings::default(),
        fog: rf_renderer::FogSettings::default(),
        initialized: false,
        cursor_locked: false,
        cursor_locked_applied: false,
        particle_batches: Vec::new(),
        ui_canvas: Canvas::new(1280.0, 720.0),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App<G: Game> {
    window_title: String,
    window_icon: Option<winit::window::Icon>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    audio: Option<AudioEngine>,
    game: G,
    world: World,
    time: Time,
    input: rf_input::InputState,
    physics: PhysicsWorld,
    camera: Camera,
    light: DirectionalLight,
    sky: rf_renderer::SkySettings,
    fog: rf_renderer::FogSettings,
    initialized: bool,
    cursor_locked: bool,
    cursor_locked_applied: bool,
    particle_batches: Vec<Vec<rf_renderer::ParticleInstance>>,
    ui_canvas: Canvas,
}

impl<G: Game> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title(self.window_title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_window_icon(self.window_icon.clone());
        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                log::error!("failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };

        let renderer = match Renderer::new(window.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::error!("failed to initialize renderer: {err:#}");
                event_loop.exit();
                return;
            }
        };
        self.camera.aspect = renderer.aspect_ratio();

        match AudioEngine::new() {
            Ok(audio) => self.audio = Some(audio),
            Err(err) => log::warn!("audio disabled: {err}"),
        }

        self.window = Some(window);
        self.renderer = Some(renderer);

        if !self.initialized {
            self.initialized = true;
            if let (Some(renderer), Some(audio)) = (&self.renderer, &self.audio) {
                let mut ctx = EngineContext {
                    world: &mut self.world,
                    physics: &mut self.physics,
                    input: &self.input,
                    audio,
                    time: &self.time,
                    renderer,
                    camera: &mut self.camera,
                    light: &mut self.light,
                    sky: &mut self.sky,
                    fog: &mut self.fog,
                    cursor_locked: &mut self.cursor_locked,
                    particle_batches: &mut self.particle_batches,
                    ui: &mut self.ui_canvas,
                };
                self.game.init(&mut ctx);
            }
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    self.camera.aspect = renderer.aspect_ratio();
                }
            }
            WindowEvent::RedrawRequested => {
                self.tick();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return;
            }
            _ => {}
        }
        self.input.process_window_event(&event);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.input.process_device_event(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<G: Game> App<G> {
    fn tick(&mut self) {
        let (Some(renderer), Some(audio)) = (&self.renderer, &self.audio) else {
            return;
        };

        self.time.tick();
        self.particle_batches.clear();
        if let Some(window) = &self.window {
            let size = window.inner_size();
            self.ui_canvas.clear(size.width as f32, size.height as f32);
        }

        while self.time.consume_fixed_step() {
            self.physics.step(self.time.fixed_delta.as_secs_f32());
            rf_physics::sync_transforms(&mut self.world, &self.physics);

            let mut ctx = EngineContext {
                world: &mut self.world,
                physics: &mut self.physics,
                input: &self.input,
                audio,
                time: &self.time,
                renderer,
                camera: &mut self.camera,
                light: &mut self.light,
                sky: &mut self.sky,
                fog: &mut self.fog,
                cursor_locked: &mut self.cursor_locked,
                particle_batches: &mut self.particle_batches,
                ui: &mut self.ui_canvas,
            };
            self.game.fixed_update(&mut ctx);
        }

        {
            let mut ctx = EngineContext {
                world: &mut self.world,
                physics: &mut self.physics,
                input: &self.input,
                audio,
                time: &self.time,
                renderer,
                camera: &mut self.camera,
                light: &mut self.light,
                sky: &mut self.sky,
                fog: &mut self.fog,
                cursor_locked: &mut self.cursor_locked,
                particle_batches: &mut self.particle_batches,
                ui: &mut self.ui_canvas,
            };
            self.game.update(&mut ctx);
        }

        if self.cursor_locked != self.cursor_locked_applied {
            self.cursor_locked_applied = self.cursor_locked;
            if let Some(window) = &self.window {
                apply_cursor_lock(window, self.cursor_locked);
            }
        }

        rf_scene::update_global_transforms(&mut self.world);

        let renderer_mut = self.renderer.as_mut().unwrap();
        sync_render_objects(&mut self.world, renderer_mut.queue());

        renderer_mut.set_camera(&self.camera);
        renderer_mut.set_light(&self.light);
        renderer_mut.set_sky(
            &self.camera,
            &self.light,
            &self.sky,
            self.time.elapsed_seconds(),
        );
        renderer_mut.set_fog(&self.fog);

        let mut query = self.world.query::<&RenderObject>();
        let objects: Vec<&RenderObject> = query.iter().map(|(_, r)| r).collect();
        let mut terrain_query = self.world.query::<&TerrainRenderObject>();
        let terrain_objects: Vec<&TerrainRenderObject> =
            terrain_query.iter().map(|(_, r)| r).collect();
        renderer_mut.prepare_ui(&self.ui_canvas);
        if let Err(err) = renderer_mut.render(objects, terrain_objects, &self.particle_batches) {
            match err {
                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                    let size = self.window.as_ref().unwrap().inner_size();
                    renderer_mut.resize(size.width, size.height);
                }
                wgpu::SurfaceError::OutOfMemory => log::error!("GPU out of memory, exiting"),
                other => log::warn!("surface error: {other:?}"),
            }
        }

        self.input.end_frame();
    }
}

fn apply_cursor_lock(window: &Window, locked: bool) {
    if locked {
        if window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .is_err()
        {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
        }
        window.set_cursor_visible(false);
    } else {
        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
        window.set_cursor_visible(true);
    }
}

fn sync_render_objects(world: &mut World, queue: &wgpu::Queue) {
    for (_entity, (transform, render_object)) in
        world.query_mut::<(&rf_math::Transform, &RenderObject)>()
    {
        render_object.set_transform(queue, transform);
    }
    for (_entity, (transform, render_object)) in
        world.query_mut::<(&rf_math::Transform, &TerrainRenderObject)>()
    {
        render_object.set_transform(queue, transform);
    }
}
