mod config;
mod service;
mod state;

use std::sync::Arc;
use std::time::Duration;

use rf_net::tonic::transport::{Identity, Server, ServerTlsConfig};
use rf_net::ArenaServiceServer;
use tokio::sync::{broadcast, Mutex};

use config::ServerConfig;
use service::{ArenaServiceImpl, Shared};
use state::ArenaWorld;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    install_crash_log_hook();
    rf_net::tls::install_default_crypto_provider();

    let config = ServerConfig::load();
    let bind_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| config.bind_addr_default.clone());

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    let (cert_pem, key_pem, fingerprint) = rf_net::tls::load_or_generate_identity(
        &exe_dir.join("server_cert.pem"),
        &exe_dir.join("server_key.pem"),
    )?;
    log::info!(
        "TLS certificate fingerprint (sha256): {fingerprint} -- clients pin this on first \
         connect; give it to players who want to verify they're talking to this server"
    );
    let tls_config = ServerTlsConfig::new().identity(Identity::from_pem(cert_pem, key_pem));

    let (snapshots, _) = broadcast::channel(16);
    let tick_rate_hz = config.tick_rate_hz;
    let shared = Arc::new(Shared {
        world: Mutex::new(ArenaWorld::new(config)),
        snapshots,
    });

    let tick_shared = shared.clone();
    tokio::spawn(async move {
        let dt = 1.0 / tick_rate_hz;
        let mut interval = tokio::time::interval(Duration::from_secs_f32(dt));
        loop {
            interval.tick().await;
            let snapshot = tick_shared.world.lock().await.tick(dt);

            let _ = tick_shared.snapshots.send(snapshot);
        }
    });

    let service = ArenaServiceImpl { shared };
    let addr = bind_addr.parse()?;
    log::info!("arena server listening on {addr} (gRPC over TLS)");

    Server::builder()
        .tls_config(tls_config)?
        .add_service(ArenaServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
