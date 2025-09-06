#![cfg(feature = "admin-mtls")]
use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use rustls::{server::WebPkiClientVerifier, RootCertStore, ServerConfig};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, watch};
use tokio_rustls::TlsAcceptor;
use x509_parser::prelude::*;

// Helper to build CA
fn gen_ca() -> (Certificate, KeyPair) {
    let mut p = CertificateParams::default();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name
        .push(DnType::CommonName, "Reload Test CA");
    let kp = KeyPair::generate().unwrap();
    let cert = p.self_signed(&kp).unwrap();
    (cert, kp)
}

fn gen_server_cert(cn: &str, ca: &Certificate, cakp: &KeyPair) -> (Vec<u8>, Vec<u8>) {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, cn);
    // SAN for hostname validation
    p.subject_alt_names.push(rcgen::SanType::DnsName(
        rcgen::Ia5String::try_from("server.test").unwrap(),
    ));
    let kp = KeyPair::generate().unwrap();
    let cert = p.signed_by(&kp, ca, cakp).unwrap();
    (cert.der().to_vec(), kp.serialize_der())
}

fn gen_client_cert(cn: &str, ca: &Certificate, cakp: &KeyPair) -> (Vec<u8>, Vec<u8>) {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, cn);
    let kp = KeyPair::generate().unwrap();
    let cert = p.signed_by(&kp, ca, cakp).unwrap();
    (cert.der().to_vec(), kp.serialize_der())
}

fn build_server_config(cert_der: &[u8], key_der: &[u8], ca_der: &[u8]) -> Arc<ServerConfig> {
    let mut roots = RootCertStore::empty();
    let _ = roots.add(rustls::pki_types::CertificateDer::from(ca_der.to_vec()));
    let verifier = WebPkiClientVerifier::builder(roots.into()).build().unwrap();
    let sc = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![rustls::pki_types::CertificateDer::from(cert_der.to_vec())],
            rustls::pki_types::PrivateKeyDer::Pkcs8(key_der.to_vec().into()),
        )
        .unwrap();
    Arc::new(sc)
}

async fn start_reloadable_server(
    initial_cfg: Arc<ServerConfig>,
    mut cfg_rx: watch::Receiver<Arc<ServerConfig>>,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let (tcp, _peer) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            let mut cfg_rx_clone = cfg_rx.clone();
            let cfg_current = cfg_rx_clone.borrow().clone();
            let acceptor = TlsAcceptor::from(cfg_current);
            tokio::spawn(async move {
                match acceptor.accept(tcp).await {
                    Ok(mut tls) => {
                        // simple fixed HTTP response
                        let _ = tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length:2\r\nConnection: close\r\n\r\nOK").await;
                    }
                    Err(_) => {}
                }
            });
        }
    });
    (addr, handle)
}

async fn fetch_server_cert_cn(
    addr: std::net::SocketAddr,
    client_cert: &[u8],
    client_key: &[u8],
    ca_der: &[u8],
) -> String {
    // Build client config
    let mut root_store = RootCertStore::empty();
    let _ = root_store.add(rustls::pki_types::CertificateDer::from(ca_der.to_vec()));
    let client_config = {
        let cc = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(
                vec![rustls::pki_types::CertificateDer::from(
                    client_cert.to_vec(),
                )],
                rustls::pki_types::PrivateKeyDer::Pkcs8(client_key.to_vec().into()),
            )
            .unwrap();
        Arc::new(cc)
    };
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("server.test").unwrap();
    let connector = tokio_rustls::TlsConnector::from(client_config);
    let tls_stream = connector.connect(server_name, stream).await.unwrap();
    // Extract server cert
    let cert = tls_stream
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|v| v.first().cloned())
        .expect("server cert");
    let (_, parsed) = parse_x509_certificate(cert.as_ref()).unwrap();
    let cn = parsed
        .subject()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .unwrap()
        .to_string();
    cn
}

#[tokio::test]
async fn mtls_tls_config_reload_changes_server_cert() {
    let (ca, cakp) = gen_ca();
    let (srv1_cert, srv1_key) = gen_server_cert("server-v1", &ca, &cakp);
    let (srv2_cert, srv2_key) = gen_server_cert("server-v2", &ca, &cakp);
    let (client_cert, client_key) = gen_client_cert("client-reload", &ca, &cakp);

    // watch channel + reload trigger
    let initial_cfg = build_server_config(&srv1_cert, &srv1_key, ca.der());
    let (cfg_tx, cfg_rx) = watch::channel(initial_cfg);
    let (addr, _handle) = start_reloadable_server(cfg_rx.borrow().clone(), cfg_rx.clone()).await;

    // First handshake should see v1
    let cn1 = fetch_server_cert_cn(addr, &client_cert, &client_key, ca.der()).await;
    assert_eq!(cn1, "server-v1");

    // Simulate reload (mock SIGHUP) by sending new config through watch channel
    let new_cfg = build_server_config(&srv2_cert, &srv2_key, ca.der());
    cfg_tx.send(new_cfg).unwrap();
    // Allow tiny delay for watch propagation
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let cn2 = fetch_server_cert_cn(addr, &client_cert, &client_key, ca.der()).await;
    assert_eq!(cn2, "server-v2");
}
