use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::pb::arena_service_client::ArenaServiceClient;
use crate::pb::{JoinRequest, PlayerInput, WorldSnapshot};

pub struct NetworkClient {
    pub player_id: u32,
    session_token: u64,
    input_tx: tokio::sync::mpsc::Sender<PlayerInput>,
    snapshot_rx: tokio::sync::mpsc::Receiver<WorldSnapshot>,
    disconnected: Arc<AtomicBool>,
}

impl NetworkClient {
    pub fn connect(
        address: String,
        player_name: String,
    ) -> std::sync::mpsc::Receiver<Result<NetworkClient, String>> {
        let (result_tx, result_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    let _ = result_tx.send(Err(format!("failed to start async runtime: {err}")));
                    return;
                }
            };

            runtime.block_on(async move {
                match connect_and_join(&address, player_name).await {
                    Ok((client, pump_handle)) => {
                        let _ = result_tx.send(Ok(client));

                        let _ = pump_handle.await;
                    }
                    Err(err) => {
                        let _ = result_tx.send(Err(err));
                    }
                }
            });
        });

        result_rx
    }

    pub fn send_input(&self, mut input: PlayerInput) {
        input.player_id = self.player_id;
        input.session_token = self.session_token;
        let _ = self.input_tx.try_send(input);
    }

    pub fn poll_latest_snapshot(&mut self) -> Option<WorldSnapshot> {
        let mut latest = None;
        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            latest = Some(snapshot);
        }
        latest
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected.load(Ordering::Relaxed)
    }
}

async fn connect_and_join(
    address: &str,
    player_name: String,
) -> Result<(NetworkClient, tokio::task::JoinHandle<()>), String> {
    let host_port = address
        .rsplit_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(address);

    let trust_store_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("trusted_servers.txt");

    let channel = crate::tls::connect_tls(host_port, trust_store_path).await?;
    let mut client = ArenaServiceClient::new(channel);

    let response = client
        .join(JoinRequest { player_name })
        .await
        .map_err(|err| format!("join request failed: {err}"))?
        .into_inner();

    if !response.accepted {
        let reason = if response.reject_reason.is_empty() {
            "join rejected by server".to_string()
        } else {
            response.reject_reason
        };
        return Err(reason);
    }

    let (input_tx, input_rx) = tokio::sync::mpsc::channel::<PlayerInput>(32);
    let outbound = ReceiverStream::new(input_rx);
    let mut inbound = client
        .stream_game(outbound)
        .await
        .map_err(|err| format!("failed to open game stream: {err}"))?
        .into_inner();

    let (snapshot_tx, snapshot_rx) = tokio::sync::mpsc::channel::<WorldSnapshot>(8);
    let disconnected = Arc::new(AtomicBool::new(false));
    let pump_disconnected = disconnected.clone();
    let pump_handle = tokio::spawn(async move {
        while let Some(Ok(snapshot)) = inbound.next().await {
            if snapshot_tx.send(snapshot).await.is_err() {
                break;
            }
        }
        pump_disconnected.store(true, Ordering::Relaxed);
    });

    Ok((
        NetworkClient {
            player_id: response.player_id,
            session_token: response.session_token,
            input_tx,
            snapshot_rx,
            disconnected,
        },
        pump_handle,
    ))
}
