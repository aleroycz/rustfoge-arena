use rf_engine::audio::Sound;
use rf_engine::input::{KeyCode, MouseButton};
use rf_engine::EngineContext;

use crate::config::AppConfig;
use crate::game::load_sound;
use crate::logo::{LOGO_BACKGROUND, LOGO_CUBE, LOGO_GRID, LOGO_PALETTE};
use crate::settings::SettingsPanel;
use crate::Screen;

const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
const INK: [f32; 4] = [0.039, 0.051, 0.063, 1.0];
const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 1.0];
const PANEL_2: [f32; 4] = [0.090, 0.114, 0.141, 1.0];
const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
const STEEL: [f32; 4] = [0.545, 0.584, 0.631, 1.0];
const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
const TEXT: [f32; 4] = [0.918, 0.953, 0.965, 1.0];
const TEXT_ON_AMBER: [f32; 4] = [0.102, 0.055, 0.020, 1.0];
const ERROR: [f32; 4] = [1.0, 0.302, 0.302, 1.0];

const CUT_MD: f32 = 12.0;
const BORDER_W: f32 = 1.5;

const RIGHT_PANEL_MIN: f32 = 340.0;
const RIGHT_PANEL_MAX: f32 = 420.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Address,
    Name,
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    pos: [f32; 2],
    size: [f32; 2],
}

impl Rect {
    fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.pos[0]
            && p[0] <= self.pos[0] + self.size[0]
            && p[1] >= self.pos[1]
            && p[1] <= self.pos[1] + self.size[1]
    }
}

struct Layout {
    address_field: Rect,
    name_field: Rect,
    join_button: Rect,
    settings_button: Rect,
    quit_button: Rect,
    panel_x: f32,
    content_x: f32,
    content_w: f32,
}

fn layout(w: f32, h: f32) -> Layout {
    let right_w = (w * 0.3).clamp(RIGHT_PANEL_MIN, RIGHT_PANEL_MAX);
    let panel_x = w - right_w;
    let content_x = panel_x + 32.0;
    let content_w = right_w - 64.0;

    let mut y = h * 0.24;
    let field_h = 40.0;
    let label_gap = 22.0;
    let field_gap = 26.0;

    y += label_gap;
    let address_field = Rect {
        pos: [content_x, y],
        size: [content_w, field_h],
    };
    y += field_h + field_gap + label_gap;
    let name_field = Rect {
        pos: [content_x, y],
        size: [content_w, field_h],
    };
    y += field_h + field_gap + 36.0;

    let button_h = 50.0;
    let button_gap = 14.0;
    let join_button = Rect {
        pos: [content_x, y],
        size: [content_w, button_h],
    };
    y += button_h + button_gap;
    let settings_button = Rect {
        pos: [content_x, y],
        size: [content_w, button_h - 6.0],
    };
    y += button_h - 6.0 + button_gap;
    let quit_button = Rect {
        pos: [content_x, y],
        size: [content_w, button_h - 6.0],
    };

    Layout {
        address_field,
        name_field,
        join_button,
        settings_button,
        quit_button,
        panel_x,
        content_x,
        content_w,
    }
}

pub struct MenuScreen {
    pub address: String,
    pub name: String,
    focus: Field,
    error: Option<String>,
    config: AppConfig,
    settings_open: bool,
    settings: SettingsPanel,
    tick_sound: Sound,
    join_sound: Sound,
}

impl MenuScreen {
    pub fn new() -> Self {
        let config = AppConfig::load();
        Self {
            address: config.last_address.clone(),
            name: config.last_name.clone(),
            focus: Field::Address,
            error: None,
            config,
            settings_open: false,
            settings: SettingsPanel::new(),
            tick_sound: load_sound("assets/sounds/ui_tick.mp3", || {
                Sound::sine_wave(900.0, std::time::Duration::from_millis(35), 44100)
            }),
            join_sound: load_sound("assets/sounds/ui_confirm.mp3", || {
                Sound::sine_wave(1300.0, std::time::Duration::from_millis(90), 44100)
            }),
        }
    }

    pub fn with_error(address: String, name: String, error: String) -> Self {
        let config = AppConfig::load();
        Self {
            address,
            name,
            focus: Field::Address,
            error: Some(error),
            config,
            settings_open: false,
            settings: SettingsPanel::new(),
            tick_sound: load_sound("assets/sounds/ui_tick.mp3", || {
                Sound::sine_wave(900.0, std::time::Duration::from_millis(35), 44100)
            }),
            join_sound: load_sound("assets/sounds/ui_confirm.mp3", || {
                Sound::sine_wave(1300.0, std::time::Duration::from_millis(90), 44100)
            }),
        }
    }

    fn play_tick(&self, ctx: &mut EngineContext) {
        if let Ok(sink) = ctx.audio.play(&self.tick_sound, self.config.sfx_gain()) {
            sink.detach();
        }
    }

    pub fn update(&mut self, ctx: &mut EngineContext) -> Option<Screen> {
        if self.settings_open {
            self.draw(ctx, &layout(ctx.ui.width, ctx.ui.height));
            ctx.ui.rect(
                [0.0, 0.0],
                [ctx.ui.width, ctx.ui.height],
                [0.0, 0.0, 0.0, 0.45],
                0.0,
            );
            if self.settings.update(ctx, &mut self.config) {
                self.settings_open = false;
                self.play_tick(ctx);
            }
            return None;
        }

        let layout = layout(ctx.ui.width, ctx.ui.height);
        let mouse: [f32; 2] = ctx.input.mouse_position().into();
        let clicked = ctx.input.mouse_pressed(MouseButton::Left);

        if ctx.input.key_pressed(KeyCode::F1) {
            self.settings_open = true;
            self.play_tick(ctx);
            return None;
        }

        if ctx.input.key_pressed(KeyCode::Tab) {
            self.focus = match self.focus {
                Field::Address => Field::Name,
                Field::Name => Field::Address,
            };
            self.play_tick(ctx);
        }

        if clicked {
            if layout.address_field.contains(mouse) && self.focus != Field::Address {
                self.focus = Field::Address;
                self.play_tick(ctx);
            } else if layout.name_field.contains(mouse) && self.focus != Field::Name {
                self.focus = Field::Name;
                self.play_tick(ctx);
            }
        }

        let field = match self.focus {
            Field::Address => &mut self.address,
            Field::Name => &mut self.name,
        };
        if ctx.input.key_pressed(KeyCode::Backspace) {
            field.pop();
        }
        for ch in ctx.input.typed_text().chars() {
            if !ch.is_control() && field.len() < 48 {
                field.push(ch);
            }
        }

        let settings_clicked = clicked && layout.settings_button.contains(mouse);
        if settings_clicked {
            self.settings_open = true;
            self.play_tick(ctx);
            return None;
        }

        if clicked && layout.quit_button.contains(mouse) {
            std::process::exit(0);
        }

        let join_requested = ctx.input.key_pressed(KeyCode::Enter)
            || ctx.input.key_pressed(KeyCode::NumpadEnter)
            || (clicked && layout.join_button.contains(mouse));

        self.draw(ctx, &layout);

        if join_requested && !self.address.trim().is_empty() {
            let address = self.address.trim().to_string();
            let name = if self.name.trim().is_empty() {
                "PLAYER".to_string()
            } else {
                self.name.trim().to_string()
            };

            self.config.last_address = address.clone();
            self.config.last_name = name.clone();
            self.config.save();

            if let Ok(sink) = ctx.audio.play(&self.join_sound, self.config.sfx_gain()) {
                sink.detach();
            }

            let rx = rf_net::NetworkClient::connect(address.clone(), name.clone());
            return Some(Screen::Connecting(crate::Connecting {
                rx,
                address,
                player_name: name,
                elapsed: 0.0,
            }));
        }
        None
    }

    fn draw(&self, ctx: &mut EngineContext, layout: &Layout) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        let mouse: [f32; 2] = ctx.input.mouse_position().into();

        ctx.ui.rect([0.0, 0.0], [w, h], INK, 0.0);
        self.draw_banner(ctx, w, h, layout.panel_x);

        ctx.ui
            .rect([layout.panel_x, 0.0], [w - layout.panel_x, h], PANEL, 0.0);
        ctx.ui.rect([layout.panel_x, 0.0], [1.0, h], BORDER, 0.0);

        ctx.ui.text(
            [layout.content_x, layout.address_field.pos[1] - 22.0],
            "SERVER ADDRESS",
            11.0,
            STEEL,
        );
        self.draw_field(
            ctx,
            layout.address_field,
            &self.address,
            self.focus == Field::Address,
        );

        ctx.ui.text(
            [layout.content_x, layout.name_field.pos[1] - 22.0],
            "PLAYER NAME",
            11.0,
            STEEL,
        );
        self.draw_field(
            ctx,
            layout.name_field,
            &self.name,
            self.focus == Field::Name,
        );

        let join_hover = layout.join_button.contains(mouse);
        draw_button(
            ctx,
            layout.join_button,
            "JOIN SERVER",
            18.0,
            join_hover,
            true,
        );

        let settings_hover = layout.settings_button.contains(mouse);
        draw_button(
            ctx,
            layout.settings_button,
            "SETTINGS",
            14.0,
            settings_hover,
            false,
        );

        let quit_hover = layout.quit_button.contains(mouse);
        draw_button(ctx, layout.quit_button, "QUIT", 14.0, quit_hover, false);

        if let Some(error) = &self.error {
            let max_chars = (layout.content_w / rf_engine::ui::Canvas::text_advance(11.0)) as usize;
            let truncated: String = if error.chars().count() > max_chars {
                error
                    .chars()
                    .take(max_chars.saturating_sub(1))
                    .collect::<String>()
                    + "…"
            } else {
                error.clone()
            };
            ctx.ui.text(
                [layout.content_x, layout.quit_button.pos[1] + 60.0],
                truncated,
                12.0,
                ERROR,
            );
        }

        let build = format!("RUSTFORGE ARENA -- BUILD {}", env!("CARGO_PKG_VERSION"));
        ctx.ui.text([24.0, h - 46.0], build, 11.0, STEEL_DIM);
        ctx.ui.text(
            [24.0, h - 28.0],
            "(c) 2026 Vakea. All rights reserved.",
            11.0,
            STEEL_DIM,
        );
    }

    fn draw_field(&self, ctx: &mut EngineContext, rect: Rect, value: &str, active: bool) {
        let border = if active { CYAN } else { BORDER };
        let fill = if active { PANEL_2 } else { PANEL };
        ctx.ui
            .frame(rect.pos, rect.size, border, fill, BORDER_W, CUT_MD);
        ctx.ui.text(
            [rect.pos[0] + 14.0, rect.pos[1] + rect.size[1] * 0.5 - 8.0],
            value,
            16.0,
            TEXT,
        );
    }

    fn draw_banner(&self, ctx: &mut EngineContext, w: f32, h: f32, right_panel_x: f32) {
        let center_x = right_panel_x * 0.5;

        let logo_size = 220.0f32;
        let cell = logo_size / LOGO_GRID as f32;
        let logo_pos = [center_x - logo_size * 0.5, h * 0.5 - logo_size * 0.72];

        for (row_i, row) in LOGO_CUBE.iter().enumerate() {
            for (col_i, &idx) in row.iter().enumerate() {
                if idx == LOGO_BACKGROUND {
                    continue;
                }
                let color = LOGO_PALETTE[idx as usize];
                let pos = [
                    logo_pos[0] + col_i as f32 * cell,
                    logo_pos[1] + row_i as f32 * cell,
                ];
                ctx.ui.rect(pos, [cell + 0.5, cell + 0.5], color, 0.0);
            }
        }

        let title = "RUSTFORGE ARENA";
        let title_scale = 42.0;
        let title_width = rf_engine::ui::Canvas::text_width(title, title_scale);
        let title_y = logo_pos[1] + logo_size + 34.0;
        ctx.ui.text(
            [center_x - title_width * 0.5, title_y],
            title,
            title_scale,
            TEXT,
        );

        let subtitle = "PROTOTYPE ARENA SHOOTER";
        let subtitle_width = rf_engine::ui::Canvas::text_width(subtitle, 13.0);
        ctx.ui.text(
            [center_x - subtitle_width * 0.5, title_y + 52.0],
            subtitle,
            13.0,
            AMBER,
        );

        let divider_w = 90.0;
        ctx.ui.rect(
            [center_x - divider_w * 0.5, title_y + 78.0],
            [divider_w, 3.0],
            CYAN,
            0.0,
        );

        let _ = w;
    }
}

fn draw_button(
    ctx: &mut EngineContext,
    rect: Rect,
    label: &str,
    scale: f32,
    hovered: bool,
    primary: bool,
) {
    if primary {
        let fill = if hovered {
            [1.0, 0.62, 0.34, 1.0]
        } else {
            AMBER
        };
        ctx.ui.rect(rect.pos, rect.size, AMBER, CUT_MD);
        ctx.ui.rect(rect.pos, rect.size, fill, CUT_MD);
    } else {
        let border = if hovered { CYAN } else { BORDER };
        let fill = if hovered { PANEL_2 } else { PANEL };
        ctx.ui
            .frame(rect.pos, rect.size, border, fill, BORDER_W, CUT_MD);
        if hovered {
            ctx.ui.rect(rect.pos, [3.0, rect.size[1]], CYAN, 0.0);
        }
    }

    let text_color = if primary { TEXT_ON_AMBER } else { TEXT };
    let label_width = rf_engine::ui::Canvas::text_width(label, scale);
    ctx.ui.text(
        [
            rect.pos[0] + rect.size[0] * 0.5 - label_width * 0.5,
            rect.pos[1] + rect.size[1] * 0.5 - scale * 0.5,
        ],
        label,
        scale,
        text_color,
    );
}
