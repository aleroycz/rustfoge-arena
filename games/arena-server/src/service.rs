use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use rf_net::tonic::{Request, Response, Status, Streaming};
use rf_net::{ArenaService, JoinRequest, JoinResponse, PlayerInput, WorldSnapshot};
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::ArenaWorld;

pub struct Shared {
    pub world: Mutex<ArenaWorld>,
    pub snapshots: broadcast::Sender<WorldSnapshot>,
}

pub struct ArenaServiceImpl {
    pub shared: Arc<Shared>,
}

#[tonic::async_trait]
impl ArenaService for ArenaServiceImpl {
    async fn join(&self, request: Request<JoinRequest>) -> Result<Response<JoinResponse>, Status> {
        let name = request.into_inner().player_name;
        let name = if name.trim().is_empty() {
            "Player".to_string()
        } else {
            name
        };
        let mut world = self.shared.world.lock().await;
        let (player_id, session_token) = world.join(name);
        Ok(Response::new(JoinResponse {
            player_id,
            accepted: true,
            reject_reason: String::new(),
            session_token,
        }))
    }

    type StreamGameStream =
        Pin<Box<dyn Stream<Item = Result<WorldSnapshot, Status>> + Send + 'static>>;

    async fn stream_game(
        &self,
        request: Request<Streaming<PlayerInput>>,
    ) -> Result<Response<Self::StreamGameStream>, Status> {
        let mut inbound = request.into_inner();
        let shared = self.shared.clone();

        let input_shared = shared.clone();
        tokio::spawn(async move {
            let mut resolved_player_id: Option<u32> = None;
            while let Ok(Some(input)) = inbound.message().await {
                if let Some(id) = input_shared.world.lock().await.apply_input(input) {
                    resolved_player_id = Some(id);
                }
            }
            if let Some(id) = resolved_player_id {
                input_shared.world.lock().await.leave(id);
            }
        });

        let receiver = shared.snapshots.subscribe();
        let outbound = BroadcastStream::new(receiver).filter_map(|item| match item {
            Ok(snapshot) => Some(Ok(snapshot)),
            Err(_lagged) => None,
        });

        Ok(Response::new(Box::pin(outbound)))
    }
}
