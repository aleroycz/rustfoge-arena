use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use http::Uri;
use hyper_util::rt::TokioIo;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

pub fn install_default_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

pub fn fingerprint_hex(der: &[u8]) -> String {
    Sha256::digest(der)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn load_or_generate_identity(
    cert_path: &Path,
    key_path: &Path,
) -> std::io::Result<(String, String, String)> {
    if let (Ok(cert_pem), Ok(key_pem)) = (
        std::fs::read_to_string(cert_path),
        std::fs::read_to_string(key_path),
    ) {
        let fingerprint = pem_fingerprint(&cert_pem).unwrap_or_default();
        return Ok((cert_pem, key_pem, fingerprint));
    }

    let cert = rcgen::generate_simple_self_signed(vec!["arena-server".to_string()])
        .map_err(std::io::Error::other)?;
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    if let Some(parent) = cert_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(cert_path, &cert_pem)?;
    std::fs::write(key_path, &key_pem)?;

    let fingerprint = fingerprint_hex(cert.cert.der());
    Ok((cert_pem, key_pem, fingerprint))
}

fn pem_fingerprint(cert_pem: &str) -> Option<String> {
    let der = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .next()?
        .ok()?;
    Some(fingerprint_hex(&der))
}

struct TrustStore {
    path: PathBuf,
    entries: HashMap<String, String>,
}

impl TrustStore {
    fn load(path: PathBuf) -> Self {
        let mut entries = HashMap::new();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            for line in contents.lines() {
                if let Some((host, fp)) = line.split_once('=') {
                    entries.insert(host.trim().to_string(), fp.trim().to_string());
                }
            }
        }
        Self { path, entries }
    }

    fn save(&self) {
        let mut contents = String::new();
        for (host, fp) in &self.entries {
            contents.push_str(host);
            contents.push('=');
            contents.push_str(fp);
            contents.push('\n');
        }
        if let Ok(mut file) = std::fs::File::create(&self.path) {
            let _ = file.write_all(contents.as_bytes());
        }
    }
}

#[derive(Debug)]
struct PinningVerifier {
    host_key: String,
    store: Mutex<TrustStore>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl std::fmt::Debug for TrustStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustStore")
            .field("path", &self.path)
            .finish()
    }
}

impl ServerCertVerifier for PinningVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint = fingerprint_hex(end_entity);
        let mut store = self.store.lock().unwrap();
        match store.entries.get(&self.host_key).cloned() {
            Some(expected) if expected == fingerprint => Ok(ServerCertVerified::assertion()),
            Some(expected) => {
                log::error!(
                    "refusing to connect to {}: certificate fingerprint changed \
                     (trusted {expected}, server presented {fingerprint}). If this is expected \
                     (e.g. the server was reinstalled), remove its line from trusted_servers.txt.",
                    self.host_key
                );
                Err(rustls::Error::General(format!(
                    "certificate fingerprint mismatch for {}",
                    self.host_key
                )))
            }
            None => {
                log::info!(
                    "trusting new server certificate for {} (fingerprint {fingerprint})",
                    self.host_key
                );
                store.entries.insert(self.host_key.clone(), fingerprint);
                store.save();
                Ok(ServerCertVerified::assertion())
            }
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn server_name_for(host: &str) -> Result<ServerName<'static>, String> {
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return Ok(ServerName::IpAddress(ip.into()));
    }
    ServerName::try_from(host.to_string()).map_err(|err| format!("invalid server address: {err}"))
}

pub async fn connect_tls(address: &str, trust_store_path: PathBuf) -> Result<Channel, String> {
    let (host, _port) = address
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid address (expected host:port): {address}"))?;
    let server_name = server_name_for(host)?;

    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let verifier = Arc::new(PinningVerifier {
        host_key: address.to_string(),
        store: Mutex::new(TrustStore::load(trust_store_path)),
        provider: provider.clone(),
    });

    let mut client_config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| err.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    client_config.alpn_protocols = vec![b"h2".to_vec()];

    let tls_connector = TlsConnector::from(Arc::new(client_config));
    let tcp_addr = address.to_string();

    let connector = service_fn(move |_uri: Uri| {
        let tls_connector = tls_connector.clone();
        let tcp_addr = tcp_addr.clone();
        let server_name = server_name.clone();
        async move {
            let tcp = TcpStream::connect(&tcp_addr).await?;
            tcp.set_nodelay(true).ok();
            let tls = tls_connector.connect(server_name, tcp).await?;
            Ok::<_, std::io::Error>(TokioIo::new(tls))
        }
    });

    let uri: Uri = format!("http://{address}")
        .parse()
        .map_err(|err| format!("invalid address: {err}"))?;

    Endpoint::from_shared(uri.to_string())
        .map_err(|err| err.to_string())?
        .connect_with_connector(connector)
        .await
        .map_err(|err| format!("TLS connection to {address} failed: {err}"))
}
