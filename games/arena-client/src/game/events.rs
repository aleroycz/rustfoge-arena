

use rf_engine::audio::Sound;
use rf_engine::math::Vec3;
use rf_engine::EngineContext;
use rf_net::EventKind;


use super::*;

impl PlayingScreen {
    pub(super) fn handle_event(&mut self, ctx: &mut EngineContext, event: &rf_net::GameEvent) {
        match EventKind::try_from(event.kind).unwrap_or(EventKind::EventNone) {
            EventKind::EventNone
            | EventKind::EventCatDropped
            | EventKind::EventCatTeleported
            | EventKind::EventMatchStarting
            | EventKind::EventMatchBegin
            | EventKind::EventMatchEnd
            | EventKind::EventWeatherChanged => {}
            EventKind::EventOrbitalStrikeWarning => {
                self.push_toast("ORBITAL STRIKE INCOMING");
                self.play_at_event_position(ctx, &self.siren_sound, event, 1.0);
            }
            EventKind::EventOrbitalStrikeImpact => {
                self.push_toast("ORBITAL STRIKE");
                self.play_at_event_position(ctx, &self.thunder_sound, event, 1.0);
            }
            EventKind::EventZeroGravityStart => self.push_toast("ZERO GRAVITY"),
            EventKind::EventZeroGravityEnd => self.push_toast("GRAVITY RESTORED"),
            EventKind::EventGiantModeStart => self.push_toast("GIANT MODE -- EVERYONE'S HUGE"),
            EventKind::EventGiantModeEnd => self.push_toast("GIANT MODE ENDED"),
            EventKind::EventAbilityUnlocked => {
                self.push_toast("QUANTUM BREAKDOWN UNLOCKED!");
                if let Ok(sink) = ctx
                    .audio
                    .play(&self.hit_marker_sound, self.config.sfx_gain())
                {
                    sink.detach();
                }
            }
            EventKind::EventUltimateUsed => {
                self.push_toast("QUANTUM BREAKDOWN!");
                self.play_at_event_position(ctx, &self.ultimate_sound, event, 1.0);
                if let Some(pos) = &event.position {
                    self.death_particles.origin = Vec3::new(pos.x, pos.y + 1.0, pos.z);
                    self.death_particles.burst(30);
                }
            }

            EventKind::EventWeaponFired => {
                if event.source_player_id == self.local_player_id {
                    return;
                }
                if let Some(pos) = &event.position {
                    let direction = event
                        .direction
                        .as_ref()
                        .map(|d| Vec3::new(d.x, d.y, d.z))
                        .unwrap_or(Vec3::NEG_Z);
                    self.muzzle_particles.origin = Vec3::new(pos.x, pos.y, pos.z);
                    self.muzzle_particles.config.direction = direction;
                    self.muzzle_particles.burst(6);
                    self.play_at_event_position(ctx, &self.shoot_sound, event, 0.7);
                }
            }
            EventKind::EventAbilityDashUsed => {
                if let Some(pos) = &event.position {
                    self.death_particles.origin = Vec3::new(pos.x, pos.y + 0.3, pos.z);
                    self.death_particles.burst(10);
                }
                self.play_at_event_position(ctx, &self.dash_sound, event, 1.0);
            }
            EventKind::EventAbilitySuperspeedUsed => {
                self.play_at_event_position(ctx, &self.superspeed_sound, event, 1.0);
            }
            EventKind::EventAbilitySwapUsed => {
                if let Some(pos) = &event.position {
                    self.death_particles.origin = Vec3::new(pos.x, pos.y + 0.5, pos.z);
                    self.death_particles.burst(14);
                }
                self.play_at_event_position(ctx, &self.swap_sound, event, 1.0);
            }
        }
    }

    fn play_at_event_position(
        &self,
        ctx: &mut EngineContext,
        sound: &Sound,
        event: &rf_net::GameEvent,
        volume_scale: f32,
    ) {
        let Some(pos) = &event.position else { return };
        let emitter = Vec3::new(pos.x, pos.y, pos.z);
        let listener = ctx.camera.position;
        if let Ok(sink) = ctx.audio.play_spatial(sound, emitter, listener) {
            sink.set_volume(self.config.sfx_gain() * volume_scale);
            sink.detach();
        }
    }

    pub(super) fn push_toast(&mut self, text: impl Into<String>) {
        self.toasts.push((text.into(), 3.0));
    }

}
