

use rf_engine::math::Vec3;
use rf_engine::EngineContext;
use rf_net::{
    ItemKind, PlayerState,
};


use super::*;
use super::util::{draw_pixel_icon, world_to_screen};

impl PlayingScreen {
    pub(super) fn draw_escape_menu(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
        const CYAN_DIM: [f32; 4] = [0.298, 0.910, 1.0, 0.14];
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 0.96];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const TEXT_DIM: [f32; 4] = [0.624, 0.690, 0.722, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const CUT_LG: f32 = 20.0;
        const BORDER_W: f32 = 1.5;

        ctx.ui
            .rect([0.0, 0.0], [w, h], [0.039, 0.051, 0.063, 0.55], 0.0);

        let panel_w = 300.0;
        let row_h = 42.0;
        let panel_h = 70.0 + row_h * ESCAPE_OPTIONS.len() as f32;
        let panel_pos = [w * 0.5 - panel_w * 0.5, h * 0.5 - panel_h * 0.5];
        ctx.ui.frame(
            panel_pos,
            [panel_w, panel_h],
            BORDER,
            PANEL,
            BORDER_W,
            CUT_LG,
        );

        let title = "PAUSED";
        let title_width = rf_engine::ui::Canvas::text_width(title, 20.0);
        ctx.ui.text(
            [w * 0.5 - title_width * 0.5, panel_pos[1] + 22.0],
            title,
            20.0,
            CYAN,
        );
        ctx.ui.rect(
            [w * 0.5 - 20.0, panel_pos[1] + 50.0],
            [40.0, 2.0],
            CYAN,
            0.0,
        );

        for (i, option) in ESCAPE_OPTIONS.iter().enumerate() {
            let y = panel_pos[1] + 74.0 + row_h * i as f32;
            let selected = i == self.escape_selected;
            if selected {
                ctx.ui.rect(
                    [panel_pos[0] + 8.0, y - 6.0],
                    [panel_w - 16.0, row_h - 8.0],
                    CYAN_DIM,
                    0.0,
                );
                ctx.ui
                    .rect([panel_pos[0] + 8.0, y - 6.0], [3.0, row_h - 8.0], CYAN, 0.0);
            }
            let color = if selected { CYAN } else { TEXT_DIM };
            let width = rf_engine::ui::Canvas::text_width(option, 15.0);
            ctx.ui
                .text([w * 0.5 - width * 0.5, y], *option, 15.0, color);
        }

        let hint = "UP/DOWN SELECT   ENTER CONFIRM   ESC RESUME";
        let hint_width = rf_engine::ui::Canvas::text_width(hint, 10.0);
        ctx.ui.text(
            [w * 0.5 - hint_width * 0.5, panel_pos[1] + panel_h + 18.0],
            hint,
            10.0,
            STEEL_DIM,
        );
    }

    pub(super) fn draw_lobby(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 0.94];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const TEXT: [f32; 4] = [0.918, 0.953, 0.965, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const HP_GREEN: [f32; 4] = [0.349, 1.0, 0.627, 1.0];
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const CUT_LG: f32 = 20.0;
        const CUT_SM: f32 = 6.0;
        const BORDER_W: f32 = 1.5;

        let title = "LOBBY -- WAITING FOR PLAYERS";
        let title_width = rf_engine::ui::Canvas::text_width(title, 20.0);
        ctx.ui
            .text([w * 0.5 - title_width * 0.5, h * 0.15], title, 20.0, CYAN);

        let players = self
            .latest_snapshot
            .as_ref()
            .map(|s| s.players.as_slice())
            .unwrap_or(&[]);
        let panel_w = 360.0;
        let row_h = 28.0;
        let panel_h = 40.0 + row_h * players.len().max(1) as f32;
        let panel_pos = [w * 0.5 - panel_w * 0.5, h * 0.5 - panel_h * 0.5];
        ctx.ui.frame(
            panel_pos,
            [panel_w, panel_h],
            BORDER,
            PANEL,
            BORDER_W,
            CUT_LG,
        );

        if players.is_empty() {
            ctx.ui.text(
                [panel_pos[0] + 16.0, panel_pos[1] + 16.0],
                "NO PLAYERS YET",
                13.0,
                STEEL_DIM,
            );
        }
        for (i, player) in players.iter().enumerate() {
            let y = panel_pos[1] + 16.0 + row_h * i as f32;
            let name_color = if player.player_id == self.local_player_id {
                CYAN
            } else {
                TEXT
            };
            ctx.ui
                .text([panel_pos[0] + 16.0, y], &player.name, 14.0, name_color);
            let (label, color) = if player.ready {
                ("READY", HP_GREEN)
            } else {
                ("NOT READY", AMBER)
            };
            let label_width = rf_engine::ui::Canvas::text_width(label, 11.0);
            ctx.ui.rect(
                [panel_pos[0] + panel_w - label_width - 26.0, y - 2.0],
                [label_width + 14.0, 17.0],
                [color[0], color[1], color[2], 0.15],
                CUT_SM,
            );
            ctx.ui.text(
                [panel_pos[0] + panel_w - label_width - 19.0, y],
                label,
                11.0,
                color,
            );
        }

        let hint = "PRESS R TO TOGGLE READY";
        let hint_width = rf_engine::ui::Canvas::text_width(hint, 12.0);
        ctx.ui.text(
            [w * 0.5 - hint_width * 0.5, panel_pos[1] + panel_h + 24.0],
            hint,
            12.0,
            STEEL_DIM,
        );
    }

    pub(super) fn draw_starting(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const STEEL: [f32; 4] = [0.545, 0.584, 0.631, 1.0];

        let seconds = self
            .latest_snapshot
            .as_ref()
            .map(|s| s.phase_time_remaining.ceil().max(0.0) as i32)
            .unwrap_or(0);
        let text = format!("MATCH STARTING IN {seconds}");
        let width = rf_engine::ui::Canvas::text_width(&text, 28.0);
        ctx.ui
            .text([w * 0.5 - width * 0.5, h * 0.45], text, 28.0, AMBER);

        let sub = "GET READY -- DROP-IN INCOMING";
        let sub_width = rf_engine::ui::Canvas::text_width(sub, 12.0);
        ctx.ui.text(
            [w * 0.5 - sub_width * 0.5, h * 0.45 + 44.0],
            sub,
            12.0,
            STEEL,
        );
    }

    pub(super) fn draw_ending(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 0.96];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const TEXT: [f32; 4] = [0.918, 0.953, 0.965, 1.0];
        const TEXT_DIM: [f32; 4] = [0.624, 0.690, 0.722, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const CUT_LG: f32 = 20.0;
        const BORDER_W: f32 = 1.5;

        let Some(snapshot) = &self.latest_snapshot else {
            return;
        };

        let title = "MATCH OVER";
        let title_width = rf_engine::ui::Canvas::text_width(title, 24.0);
        ctx.ui
            .text([w * 0.5 - title_width * 0.5, h * 0.14], title, 24.0, AMBER);

        let winner_text = format!("WINNER: {}", snapshot.winner_name.to_uppercase());
        let winner_width = rf_engine::ui::Canvas::text_width(&winner_text, 18.0);
        ctx.ui.text(
            [w * 0.5 - winner_width * 0.5, h * 0.14 + 40.0],
            winner_text,
            18.0,
            TEXT,
        );

        let mut players = snapshot.players.clone();
        players.sort_by_key(|p| std::cmp::Reverse(p.kills));
        let panel_w = 320.0;
        let row_h = 26.0;
        let panel_h = 40.0 + row_h * players.len().max(1) as f32;
        let panel_pos = [w * 0.5 - panel_w * 0.5, h * 0.35];
        ctx.ui.frame(
            panel_pos,
            [panel_w, panel_h],
            BORDER,
            PANEL,
            BORDER_W,
            CUT_LG,
        );
        for (i, player) in players.iter().enumerate() {
            let y = panel_pos[1] + 16.0 + row_h * i as f32;
            let color = if player.player_id == snapshot.winner_player_id {
                AMBER
            } else {
                TEXT_DIM
            };
            let rank_name = format!("{}. {}", i + 1, player.name);
            ctx.ui
                .text([panel_pos[0] + 16.0, y], rank_name, 14.0, color);
            let stats = format!("{}K / {}D", player.kills, player.deaths);
            let stats_width = rf_engine::ui::Canvas::text_width(&stats, 13.0);
            ctx.ui.text(
                [panel_pos[0] + panel_w - stats_width - 16.0, y],
                stats,
                13.0,
                color,
            );
        }

        let hint = "RETURNING TO LOBBY SHORTLY...";
        let hint_width = rf_engine::ui::Canvas::text_width(hint, 11.0);
        ctx.ui.text(
            [w * 0.5 - hint_width * 0.5, panel_pos[1] + panel_h + 20.0],
            hint,
            11.0,
            STEEL_DIM,
        );
    }

    pub(super) fn draw_hud(&self, ctx: &mut EngineContext) {
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const CYAN: [f32; 4] = [0.298, 0.910, 1.0, 1.0];
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 0.82];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
        const TEXT: [f32; 4] = [0.918, 0.953, 0.965, 1.0];
        const TEXT_DIM: [f32; 4] = [0.624, 0.690, 0.722, 1.0];
        const STEEL: [f32; 4] = [0.545, 0.584, 0.631, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const HP_GREEN: [f32; 4] = [0.349, 1.0, 0.627, 1.0];
        const HP_RED: [f32; 4] = [1.0, 0.231, 0.231, 1.0];
        const NAMEPLATE_BG: [f32; 4] = [0.039, 0.051, 0.063, 0.55];
        const CUT_MD: f32 = 10.0;
        const CUT_SM: f32 = 5.0;
        const BORDER_W: f32 = 1.5;

        let local = self.latest_snapshot.as_ref().and_then(|s| {
            s.players
                .iter()
                .find(|p| p.player_id == self.local_player_id)
        });

        if let Some(snapshot) = &self.latest_snapshot {
            for player in &snapshot.players {
                if player.player_id == self.local_player_id || !player.alive {
                    continue;
                }
                let Some(position) = &player.position else {
                    continue;
                };
                let head = Vec3::new(position.x, position.y, position.z)
                    + Vec3::new(0.0, 1.1 * player.scale.max(0.01), 0.0);
                if let Some(screen) = world_to_screen(ctx.camera, w, h, head) {
                    let width = rf_engine::ui::Canvas::text_width(&player.name, 13.0);
                    ctx.ui.rect(
                        [screen[0] - width * 0.5 - 6.0, screen[1] - 3.0],
                        [width + 12.0, 19.0],
                        NAMEPLATE_BG,
                        CUT_SM,
                    );
                    ctx.ui.text(
                        [screen[0] - width * 0.5, screen[1]],
                        &player.name,
                        13.0,
                        TEXT,
                    );
                }
            }
            for poi in &snapshot.points_of_interest {
                let Some(pos) = &poi.position else { continue };
                let head = Vec3::new(pos.x, 5.5, pos.z);
                if let Some(screen) = world_to_screen(ctx.camera, w, h, head) {
                    let width = rf_engine::ui::Canvas::text_width(&poi.name, 12.0);
                    ctx.ui.rect(
                        [screen[0] - width * 0.5 - 6.0, screen[1] - 3.0],
                        [width + 12.0, 18.0],
                        NAMEPLATE_BG,
                        CUT_SM,
                    );
                    ctx.ui.text(
                        [screen[0] - width * 0.5, screen[1]],
                        &poi.name,
                        12.0,
                        TEXT_DIM,
                    );
                }
            }
        }

        if local.map(|p| p.alive).unwrap_or(true) {
            let center = [w * 0.5, h * 0.5];
            for rect in [
                ([center[0] - 1.5, center[1] - 10.0], [3.0, 7.0]),
                ([center[0] - 1.5, center[1] + 3.0], [3.0, 7.0]),
                ([center[0] - 10.0, center[1] - 1.5], [7.0, 3.0]),
                ([center[0] + 3.0, center[1] - 1.5], [7.0, 3.0]),
            ] {
                ctx.ui.rect(rect.0, rect.1, CYAN, 0.0);
            }
        }

        let panel_pos = [24.0, h - 70.0];
        ctx.ui
            .frame(panel_pos, [200.0, 46.0], BORDER, PANEL, BORDER_W, CUT_MD);
        ctx.ui.text(
            [panel_pos[0] + 14.0, panel_pos[1] + 8.0],
            "HEALTH",
            11.0,
            STEEL,
        );
        let health_fraction = local
            .map(|p| (p.health.max(0) as f32) / 100.0)
            .unwrap_or(1.0);
        let health_color = if health_fraction < 0.3 {
            HP_RED
        } else {
            HP_GREEN
        };
        ctx.ui.bar(
            [panel_pos[0] + 14.0, panel_pos[1] + 24.0],
            [172.0, 12.0],
            health_fraction,
            TRACK,
            health_color,
            CUT_SM,
        );

        if let Some(local) = local {
            let held = ItemKind::try_from(local.held_item).unwrap_or(ItemKind::ItemNone);
            if held != ItemKind::ItemNone {
                let name = match held {
                    ItemKind::ItemHeal => "HEAL PACK",
                    ItemKind::ItemDancingCat => "THE CAT",
                    ItemKind::ItemNone => "",
                };
                let text = format!("HOLDING: {name}  [E] THROW");
                let width = rf_engine::ui::Canvas::text_width(&text, 13.0);
                ctx.ui.frame(
                    [w * 0.5 - width * 0.5 - 12.0, h - 100.0],
                    [width + 24.0, 26.0],
                    BORDER,
                    PANEL,
                    BORDER_W,
                    CUT_MD,
                );
                ctx.ui
                    .text([w * 0.5 - width * 0.5, h - 92.0], text, 13.0, AMBER);
            }
        }

        self.draw_ability_hotbar(ctx, local);

        if let Some(snapshot) = &self.latest_snapshot {
            let mut players = snapshot.players.clone();
            players.sort_by_key(|p| std::cmp::Reverse(p.kills));
            let board_w = 220.0;
            let row_h = 22.0;
            let board_h = 30.0 + row_h * players.len() as f32;
            let board_pos = [w - board_w - 20.0, 60.0];
            ctx.ui.frame(
                board_pos,
                [board_w, board_h],
                BORDER,
                PANEL,
                BORDER_W,
                CUT_MD,
            );
            ctx.ui.text(
                [board_pos[0] + 14.0, board_pos[1] + 10.0],
                "SCOREBOARD",
                11.0,
                STEEL_DIM,
            );
            for (i, player) in players.iter().enumerate() {
                let y = board_pos[1] + 30.0 + row_h * i as f32;
                let color = if player.player_id == self.local_player_id {
                    CYAN
                } else {
                    TEXT_DIM
                };
                ctx.ui
                    .text([board_pos[0] + 14.0, y], &player.name, 12.0, color);
                let stats = format!("{}/{}", player.kills, player.deaths);
                let stats_width = rf_engine::ui::Canvas::text_width(&stats, 12.0);
                ctx.ui.text(
                    [board_pos[0] + board_w - stats_width - 14.0, y],
                    stats,
                    12.0,
                    color,
                );
            }

            let remaining = snapshot.phase_time_remaining.max(0.0) as u32;
            let clock_text = format!("{:02}:{:02}", remaining / 60, remaining % 60);
            let clock_width = rf_engine::ui::Canvas::text_width(&clock_text, 16.0);
            ctx.ui.frame(
                [w * 0.5 - clock_width * 0.5 - 10.0, 14.0],
                [clock_width + 20.0, 26.0],
                BORDER,
                PANEL,
                BORDER_W,
                CUT_MD,
            );
            ctx.ui
                .text([w * 0.5 - clock_width * 0.5, 20.0], clock_text, 16.0, TEXT);
        }

        let mut toast_y = h * 0.22;
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

        if let Some(local) = local {
            if !local.alive {
                let text = "YOU DIED";
                let width = rf_engine::ui::Canvas::text_width(text, 24.0);
                ctx.ui
                    .text([w * 0.5 - width * 0.5, h * 0.38], text, 24.0, HP_RED);

                let seconds = local.respawn_time_remaining.ceil().max(0.0) as i32;
                let respawn_text = format!("RESPAWNING IN {seconds}");
                let respawn_width = rf_engine::ui::Canvas::text_width(&respawn_text, 16.0);
                ctx.ui.rect(
                    [w * 0.5 - respawn_width * 0.5 - 12.0, h * 0.38 + 34.0],
                    [respawn_width + 24.0, 28.0],
                    PANEL,
                    8.0,
                );
                ctx.ui.text(
                    [w * 0.5 - respawn_width * 0.5, h * 0.38 + 42.0],
                    respawn_text,
                    16.0,
                    TEXT,
                );

                let hint = "FREE CAM: WASD + MOUSE, SPACE/SHIFT UP-DOWN";
                let hint_width = rf_engine::ui::Canvas::text_width(hint, 11.0);
                ctx.ui.text(
                    [w * 0.5 - hint_width * 0.5, h * 0.38 + 70.0],
                    hint,
                    11.0,
                    STEEL_DIM,
                );
            }
        }

        let hint = "ESC: MENU";
        let hint_width = rf_engine::ui::Canvas::text_width(hint, 12.0);
        ctx.ui.rect(
            [w - hint_width - 32.0, h - 36.0],
            [hint_width + 20.0, 22.0],
            PANEL,
            CUT_SM,
        );
        ctx.ui
            .text([w - hint_width - 22.0, h - 32.0], hint, 12.0, STEEL_DIM);
    }

    fn draw_ability_hotbar(&self, ctx: &mut EngineContext, local: Option<&PlayerState>) {
        let Some(local) = local else { return };
        let w = ctx.ui.width;
        let h = ctx.ui.height;
        const PANEL: [f32; 4] = [0.071, 0.086, 0.106, 0.88];
        const BORDER: [f32; 4] = [0.165, 0.196, 0.231, 1.0];
        const AMBER: [f32; 4] = [1.0, 0.541, 0.239, 1.0];
        const TEXT: [f32; 4] = [0.918, 0.953, 0.965, 1.0];
        const STEEL: [f32; 4] = [0.545, 0.584, 0.631, 1.0];
        const STEEL_DIM: [f32; 4] = [0.333, 0.376, 0.420, 1.0];
        const CUT_MD: f32 = 10.0;
        const BORDER_W: f32 = 1.5;

        const SLOT: f32 = 58.0;
        const GAP: f32 = 10.0;

        struct Slot<'a> {
            name: &'a str,
            key: &'a str,
            icon: &'a [[u8; 5]; 5],
            color: [f32; 4],
            unlock_kills: u32,
            cooldown_remaining: f32,
            cooldown_max: f32,
        }

        let slots = [
            Slot {
                name: "DASH",
                key: "1",
                icon: &DASH_ICON,
                color: [1.0, 0.84, 0.24, 1.0],
                unlock_kills: 0,
                cooldown_remaining: local.dash_cooldown_remaining,
                cooldown_max: DASH_COOLDOWN_MAX,
            },
            Slot {
                name: "SPEED",
                key: "2",
                icon: &SUPERSPEED_ICON,
                color: [0.3, 0.85, 1.0, 1.0],
                unlock_kills: 0,
                cooldown_remaining: local.superspeed_cooldown_remaining,
                cooldown_max: SUPERSPEED_COOLDOWN_MAX,
            },
            Slot {
                name: "SWAP",
                key: "3",
                icon: &SWAP_ICON,
                color: [0.78, 0.47, 1.0, 1.0],
                unlock_kills: 0,
                cooldown_remaining: local.swap_cooldown_remaining,
                cooldown_max: SWAP_COOLDOWN_MAX,
            },
            Slot {
                name: "ULT",
                key: "4",
                icon: &ULTIMATE_ICON,
                color: [1.0, 0.43, 0.24, 1.0],
                unlock_kills: 5,
                cooldown_remaining: local.ultimate_cooldown_remaining,
                cooldown_max: ULTIMATE_COOLDOWN_MAX,
            },
        ];

        let total_w = slots.len() as f32 * SLOT + (slots.len() as f32 - 1.0) * GAP;
        let start_x = w * 0.5 - total_w * 0.5;
        let y = h - 172.0;

        for (i, slot) in slots.iter().enumerate() {
            let x = start_x + i as f32 * (SLOT + GAP);
            let unlocked = local.kills >= slot.unlock_kills;
            let ready = unlocked && slot.cooldown_remaining <= 0.05;

            let border = if ready { AMBER } else { BORDER };
            ctx.ui
                .frame([x, y], [SLOT, SLOT], border, PANEL, BORDER_W, CUT_MD);

            let icon_color = if !unlocked {
                [
                    slot.color[0] * 0.35,
                    slot.color[1] * 0.35,
                    slot.color[2] * 0.35,
                    0.7,
                ]
            } else if ready {
                slot.color
            } else {
                STEEL_DIM
            };
            draw_pixel_icon(ctx, [x + 9.0, y + 7.0], 7.0, slot.icon, icon_color);

            ctx.ui.text([x + 4.0, y + 2.0], slot.key, 9.0, STEEL);
            let name_width = rf_engine::ui::Canvas::text_width(slot.name, 8.0);
            let name_color = if unlocked { TEXT } else { STEEL_DIM };
            ctx.ui.text(
                [x + SLOT * 0.5 - name_width * 0.5, y + SLOT - 13.0],
                slot.name,
                8.0,
                name_color,
            );

            if !unlocked {
                ctx.ui
                    .rect([x, y], [SLOT, SLOT], [0.039, 0.051, 0.063, 0.72], CUT_MD);
                let lock_text = format!("{}K", slot.unlock_kills);
                let lock_width = rf_engine::ui::Canvas::text_width(&lock_text, 11.0);
                ctx.ui.text(
                    [x + SLOT * 0.5 - lock_width * 0.5, y + SLOT * 0.5 - 5.0],
                    lock_text,
                    11.0,
                    STEEL,
                );
            } else if slot.cooldown_remaining > 0.05 {
                let fraction = (slot.cooldown_remaining / slot.cooldown_max).clamp(0.0, 1.0);
                let overlay_h = SLOT * fraction;
                ctx.ui.rect(
                    [x, y + (SLOT - overlay_h)],
                    [SLOT, overlay_h],
                    [0.039, 0.051, 0.063, 0.78],
                    0.0,
                );
                let seconds_text = format!("{}", slot.cooldown_remaining.ceil() as i32);
                let seconds_width = rf_engine::ui::Canvas::text_width(&seconds_text, 18.0);
                ctx.ui.text(
                    [x + SLOT * 0.5 - seconds_width * 0.5, y + SLOT * 0.5 - 9.0],
                    seconds_text,
                    18.0,
                    TEXT,
                );
            }
        }
    }
}
