fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    rf_net::tls::install_default_crypto_provider();

    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:50051".to_string());

    for attempt in 1..=2 {
        println!("--- attempt {attempt} ---");
        let rx = rf_net::NetworkClient::connect(address.clone(), "smoke-test".to_string());
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(client)) => println!("joined OK, player_id={}", client.player_id),
            Ok(Err(err)) => {
                println!("join failed: {err}");
                std::process::exit(1);
            }
            Err(err) => {
                println!("timed out waiting for connection: {err}");
                std::process::exit(1);
            }
        }
    }
}
