mod config;
mod discord;
mod game;
mod logo;
mod menu;
mod settings;

use rf_engine::input::KeyCode;
use rf_engine::{EngineContext, Game};

use discord::DiscordPresence;

pub struct Connecting {
    pub rx: std::sync::mpsc::Receiver<Result<rf_net::NetworkClient, String>>,
    pub address: String,
    pub player_name: String,
    pub elapsed: f32,
}

pub enum Screen {
    Menu(menu::MenuScreen),
    Connecting(Connecting),
    Playing(Box<game::PlayingScreen>),
}

struct ArenaClient {
    screen: Screen,
    discord: DiscordPresence,
}

impl Default for ArenaClient {
    fn default() -> Self {
        Self {
            screen: Screen::Menu(menu::MenuScreen::new()),
            discord: DiscordPresence::new(),
        }
    }
}

impl Game for ArenaClient {
    fn init(&mut self, _ctx: &mut EngineContext) {}

    fn fixed_update(&mut self, ctx: &mut EngineContext) {
        let mut transition = None;
        match &mut self.screen {
            Screen::Menu(_) => {}
            Screen::Connecting(connecting) => {
                connecting.elapsed += ctx.time.fixed_delta.as_secs_f32();
                if ctx.input.key_pressed(KeyCode::Escape) {
                    transition = Some(Screen::Menu(menu::MenuScreen::new()));
                } else if let Ok(result) = connecting.rx.try_recv() {
                    transition = Some(match result {
                        Ok(client) => Screen::Playing(Box::new(game::PlayingScreen::new(
                            ctx,
                            client,
                            connecting.address.clone(),
                            connecting.player_name.clone(),
                        ))),
                        Err(err) => Screen::Menu(menu::MenuScreen::with_error(
                            connecting.address.clone(),
                            connecting.player_name.clone(),
                            err,
                        )),
                    });
                }
            }
            Screen::Playing(playing) => {
                transition = playing.fixed_update(ctx);
            }
        }
        if let Some(next) = transition {
            self.screen = next;
        }
    }

    fn update(&mut self, ctx: &mut EngineContext) {
        match &mut self.screen {
            Screen::Menu(menu) => {
                self.discord.set_menu();
                if let Some(next) = menu.update(ctx) {
                    self.screen = next;
                }
            }
            Screen::Connecting(connecting) => {
                self.discord.set_connecting(&connecting.address);
                draw_connecting(ctx, connecting);
            }
            Screen::Playing(playing) => {
                playing.sync_discord_presence(&mut self.discord);
                if let Some(next) = playing.update(ctx) {
                    self.screen = next;
                }
            }
        }
    }
}

fn draw_connecting(ctx: &mut EngineContext, connecting: &Connecting) {
    let w = ctx.ui.width;
    let h = ctx.ui.height;
    const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
    const INK: [f32; 4] = [0.039, 0.051, 0.063, 1.0];
    const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 1.0];
    const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
    const STEEL: [f32; 4] = [0.545, 0.584, 0.631, 1.0];
    const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
    const CUT_LG: f32 = 20.0;
    const CUT_SM: f32 = 3.0;
    const BORDER_W: f32 = 1.5;

    ctx.ui.rect([0.0, 0.0], [w, h], INK, 0.0);

    let panel_w = 380.0;
    let panel_h = 130.0;
    let panel_pos = [w * 0.5 - panel_w * 0.5, h * 0.5 - panel_h * 0.5];
    ctx.ui.frame(
        panel_pos,
        [panel_w, panel_h],
        BORDER,
        PANEL,
        BORDER_W,
        CUT_LG,
    );

    let text = "CONNECTING";
    let scale = 20.0;
    let width = rf_engine::ui::Canvas::text_width(text, scale);
    ctx.ui.text(
        [w * 0.5 - width * 0.5, panel_pos[1] + 24.0],
        text,
        scale,
        CYAN,
    );

    let dot_count = 5;
    let dot_size = 9.0;
    let gap = 9.0;
    let total_width = dot_count as f32 * dot_size + (dot_count as f32 - 1.0) * gap;
    let start_x = w * 0.5 - total_width * 0.5;
    let active = ((connecting.elapsed * 3.0) as usize) % dot_count;
    for i in 0..dot_count {
        let alpha = if i == active { 1.0 } else { 0.25 };
        ctx.ui.rect(
            [start_x + i as f32 * (dot_size + gap), panel_pos[1] + 62.0],
            [dot_size, dot_size],
            [CYAN[0], CYAN[1], CYAN[2], alpha],
            CUT_SM,
        );
    }

    let address_text = format!("SERVER: {}", connecting.address);
    let address_width = rf_engine::ui::Canvas::text_width(&address_text, 12.0);
    ctx.ui.text(
        [w * 0.5 - address_width * 0.5, panel_pos[1] + 88.0],
        address_text,
        12.0,
        STEEL,
    );

    let hint = "ESC TO CANCEL";
    let hint_width = rf_engine::ui::Canvas::text_width(hint, 10.0);
    ctx.ui.text(
        [w * 0.5 - hint_width * 0.5, panel_pos[1] + panel_h - 22.0],
        hint,
        10.0,
        STEEL_DIM,
    );
}

fn install_crash_log_hook() {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "full");
    }
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let message = format!("{info}\nbacktrace:\n{backtrace}\n");
        eprintln!("PANIC: {message}");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("crash.log")
        {
            use std::io::Write;
            let _ = writeln!(file, "--- panic ---\n{message}");
        }
    }));
}

const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon/icon_1024.png");

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    install_crash_log_hook();
    rf_net::tls::install_default_crypto_provider();
    rf_engine::run_with_icon("RustForge Arena", Some(ICON_PNG), ArenaClient::default())
}
