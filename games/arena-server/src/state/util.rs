use rand::rngs::SmallRng;
use rand::Rng;
use rf_math::Vec3;
use rf_net::{EventKind, GameEvent, Vec3 as PbVec3};

use crate::config::ServerConfig;

pub(crate) fn random_arena_point(rng: &mut SmallRng, config: &ServerConfig, height: f32) -> Vec3 {
    let margin = 4.0;
    let half_extent = config.arena_half_extent;
    Vec3::new(
        rng.gen_range(-half_extent + margin..half_extent - margin),
        height,
        rng.gen_range(-half_extent + margin..half_extent - margin),
    )
}

pub(crate) fn to_pb(v: Vec3) -> PbVec3 {
    PbVec3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

pub(crate) fn event(kind: EventKind, position: Vec3, value: f32) -> GameEvent {
    GameEvent {
        kind: kind as i32,
        position: Some(to_pb(position)),
        value,
        direction: None,
        source_player_id: 0,
    }
}

pub(crate) fn player_event(
    kind: EventKind,
    position: Vec3,
    direction: Vec3,
    source_player_id: u32,
    value: f32,
) -> GameEvent {
    GameEvent {
        kind: kind as i32,
        position: Some(to_pb(position)),
        value,
        direction: Some(to_pb(direction)),
        source_player_id,
    }
}
