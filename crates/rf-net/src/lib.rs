pub mod pb {
    tonic::include_proto!("rustforge.arena");
}

mod client;
pub use client::NetworkClient;

pub mod tls;

pub use pb::arena_service_client::ArenaServiceClient;
pub use pb::arena_service_server::{ArenaService, ArenaServiceServer};
pub use pb::{
    EventKind, GameEvent, ItemKind, ItemState, JoinRequest, JoinResponse, MatchPhase, PlayerInput,
    PlayerState, PointOfInterest, PowerupKind, PowerupState, Vec3, WeatherKind, WorldSnapshot,
};

pub use tokio;
pub use tonic;
