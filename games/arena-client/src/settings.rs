use rf_engine::input::KeyCode;
use rf_engine::EngineContext;

use crate::config::AppConfig;

const VOLUME_STEP: f32 = 0.05;

#[derive(Default)]
pub struct SettingsPanel {
    selected: usize,
}

type VolumeGetter = fn(&AppConfig) -> f32;
type VolumeSetter = fn(&mut AppConfig, f32);

const ROWS: [(&str, VolumeGetter, VolumeSetter); 3] = [
    (
        "MASTER VOLUME",
        |c| c.master_volume,
        |c, v| c.master_volume = v,
    ),
    (
        "MUSIC VOLUME",
        |c| c.music_volume,
        |c, v| c.music_volume = v,
    ),
    ("SFX VOLUME", |c| c.sfx_volume, |c, v| c.sfx_volume = v),
];

impl SettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, ctx: &mut EngineContext, config: &mut AppConfig) -> bool {
        if ctx.input.key_pressed(KeyCode::ArrowDown) {
            self.selected = (self.selected + 1) % ROWS.len();
        }
        if ctx.input.key_pressed(KeyCode::ArrowUp) {
            self.selected = (self.selected + ROWS.len() - 1) % ROWS.len();
        }

        let (_, get, set) = ROWS[self.selected];
        let mut changed = false;
        if ctx.input.key_pressed(KeyCode::ArrowRight) {
            set(config, (get(config) + VOLUME_STEP).min(1.0));
            changed = true;
        }
        if ctx.input.key_pressed(KeyCode::ArrowLeft) {
            set(config, (get(config) - VOLUME_STEP).max(0.0));
            changed = true;
        }
        if changed {
            config.save();
        }

        self.draw(ctx, config);

        ctx.input.key_pressed(KeyCode::Escape) || ctx.input.key_pressed(KeyCode::Enter)
    }

    fn draw(&self, ctx: &mut EngineContext, config: &AppConfig) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 1.0];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
        const TEXT_DIM: [f32; 4] = [0.624, 0.690, 0.722, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const CUT_LG: f32 = 20.0;
        const CUT_SM: f32 = 5.0;
        const BORDER_W: f32 = 1.5;

        let panel_w = 420.0;
        let row_h = 56.0;
        let panel_h = 70.0 + row_h * ROWS.len() as f32;
        let panel_pos = [w * 0.5 - panel_w * 0.5, h * 0.5 - panel_h * 0.5];
        ctx.ui.frame(
            panel_pos,
            [panel_w, panel_h],
            BORDER,
            PANEL,
            BORDER_W,
            CUT_LG,
        );

        let title = "SETTINGS";
        let title_width = rf_engine::ui::Canvas::text_width(title, 20.0);
        ctx.ui.text(
            [w * 0.5 - title_width * 0.5, panel_pos[1] + 24.0],
            title,
            20.0,
            CYAN,
        );
        ctx.ui.rect(
            [w * 0.5 - 20.0, panel_pos[1] + 52.0],
            [40.0, 2.0],
            CYAN,
            0.0,
        );

        for (i, (label, get, _)) in ROWS.iter().enumerate() {
            let y = panel_pos[1] + 76.0 + row_h * i as f32;
            let is_selected = i == self.selected;
            let label_color = if is_selected { CYAN } else { TEXT_DIM };
            ctx.ui
                .text([panel_pos[0] + 24.0, y], *label, 12.0, label_color);

            let percent = format!("{}%", (get(config) * 100.0).round() as i32);
            let percent_width = rf_engine::ui::Canvas::text_width(&percent, 11.0);
            ctx.ui.text(
                [panel_pos[0] + panel_w - percent_width - 24.0, y - 1.0],
                percent,
                11.0,
                STEEL_DIM,
            );

            let bar_color = if is_selected { AMBER } else { CYAN };
            let bar_pos = [panel_pos[0] + 24.0, y + 18.0];
            let bar_size = [panel_w - 48.0, 10.0];
            ctx.ui
                .bar(bar_pos, bar_size, get(config), TRACK, bar_color, CUT_SM);
        }

        let hint = "UP/DOWN SELECT   LEFT/RIGHT ADJUST   ENTER/ESC BACK";
        let hint_width = rf_engine::ui::Canvas::text_width(hint, 10.0);
        ctx.ui.text(
            [w * 0.5 - hint_width * 0.5, panel_pos[1] + panel_h + 20.0],
            hint,
            10.0,
            STEEL_DIM,
        );
    }
}
