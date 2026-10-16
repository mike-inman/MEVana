#![cfg(all(unix, feature = "admin-mtls"))]
use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use rustls::RootCertStore;
use std::{
    fs,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::{sleep, Duration},
};
use x509_parser::prelude::*;

fn gen_ca() -> (Certificate, KeyPair) {
    let mut p = CertificateParams::default();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name
        .push(DnType::CommonName, "BinaryReloadTest CA");
    let kp = KeyPair::generate().unwrap();
    let cert = p.self_signed(&kp).unwrap();
    (cert, kp)
}

fn gen_server_cert(cn: &str, ca: &Certificate, cakp: &KeyPair) -> (Vec<u8>, Vec<u8>) {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, cn);
    p.subject_alt_names.push(rcgen::SanType::DnsName(
        rcgen::Ia5String::try_from("localhost").unwrap(),
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

fn write_pem_files(dir: &PathBuf, cert_der: &[u8], key_der: &[u8], ca_der: &[u8]) {
    // Write server cert
    let mut cert_pem = vec![
        b"-----BEGIN CERTIFICATE-----\n".to_vec(),
        base64::encode(cert_der).into_bytes(),
        b"\n-----END CERTIFICATE-----\n".to_vec(),
    ]
    .concat();
    fs::write(dir.join("server.crt"), &cert_pem).unwrap();
    // Write key (PKCS8)
    let mut key_pem = vec![
        b"-----BEGIN PRIVATE KEY-----\n".to_vec(),
        base64::encode(key_der).into_bytes(),
        b"\n-----END PRIVATE KEY-----\n".to_vec(),
    ]
    .concat();
    fs::write(dir.join("server.key"), &key_pem).unwrap();
    // CA chain
    let mut ca_pem = vec![
        b"-----BEGIN CERTIFICATE-----\n".to_vec(),
        base64::encode(ca_der).into_bytes(),
        b"\n-----END CERTIFICATE-----\n".to_vec(),
    ]
    .concat();
    fs::write(dir.join("ca.crt"), &ca_pem).unwrap();
}

async fn fetch_server_cn(
    port: u16,
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
        std::sync::Arc::new(cc)
    };
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let connector = tokio_rustls::TlsConnector::from(client_config);
    let mut tls_stream = connector.connect(server_name, stream).await.unwrap();
    // Send simple request
    let _ = tls_stream
        .write_all(b"GET /token/xyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await;
    let mut buf = Vec::new();
    let _ = tls_stream.read_to_end(&mut buf).await;
    let cert = tls_stream
        .get_ref()
        .1
        .peer_certificates()
        .and_then(|v| v.first().cloned())
        .unwrap();
    let (_, parsed) = parse_x509_certificate(cert.as_ref()).unwrap();
    parsed
        .subject()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn binary_mtls_reload_via_sighup() {
    // Skip test in CI if DATABASE_URL not provided (avoid spinning postgres); use skip marker
    if std::env::var("CI").is_ok() && std::env::var("DATABASE_URL").is_err() {
        eprintln!("SKIP: DATABASE_URL not set in CI");
        return;
    }
    let (ca, cakp) = gen_ca();
    let (srv1_cert, srv1_key) = gen_server_cert("executor-v1", &ca, &cakp);
    let (srv2_cert, srv2_key) = gen_server_cert("executor-v2", &ca, &cakp);
    let (client_cert, client_key) = gen_client_cert("binary-client", &ca, &cakp);

    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_path_buf();
    write_pem_files(&dir, &srv1_cert, &srv1_key, ca.der());

    // Choose random free port
    let port: u16 = 19000 + (fastrand::u16(..1000));

    // Launch binary
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--quiet")
        .arg("-p")
        .arg("executor-service")
        .arg("--bin")
        .arg("executor-service")
        .env("RUST_LOG", "info")
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb".into()),
        )
        .env("TOKEN_REGISTRY_PORT", port.to_string())
        .env("ADMIN_MTLS_ENABLE", "1")
        .env("ADMIN_TLS_CERT", dir.join("server.crt"))
        .env("ADMIN_TLS_KEY", dir.join("server.key"))
        .env("ADMIN_TLS_CA", dir.join("ca.crt"))
        .env("ADMIN_ALLOWED_SUBJECTS", "binary-client")
        .env("DISABLE_DB", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn executor-service");
    let stderr = child.stderr.take().unwrap();
    let log_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let log_buf_reader = log_buf.clone();
    tokio::spawn(async move {
        let mut rdr = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match rdr.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    log_buf_reader.lock().unwrap().push_str(&line);
                }
                Err(_) => break,
            }
        }
    });

    // Allow startup
    sleep(Duration::from_secs(2)).await;

    // First fetch should show v1
    let cn1 = fetch_server_cn(port, &client_cert, &client_key, ca.der()).await;
    assert_eq!(cn1, "executor-v1");

    // Overwrite cert/key with new version
    write_pem_files(&dir, &srv2_cert, &srv2_key, ca.der());

    // Send SIGHUP to trigger reload
    unsafe {
        libc::kill(child.id().unwrap() as i32, libc::SIGHUP);
    }
    sleep(Duration::from_millis(300)).await; // small delay for reload

    let cn2 = fetch_server_cn(port, &client_cert, &client_key, ca.der()).await;
    assert_eq!(cn2, "executor-v2");
    // Verify reload log occurred
    {
        let logs = log_buf.lock().unwrap().clone();
        assert!(
            logs.contains("mtls_config_reloaded"),
            "expected reload log not found. logs={}",
            logs
        );
    }

    // Terminate
    unsafe {
        libc::kill(child.id().unwrap() as i32, libc::SIGINT);
    }
    let _ = child.wait().await;
}

#[tokio::test]
async fn binary_mtls_unauthorized_subject_rejected() {
    if std::env::var("CI").is_ok() && std::env::var("DATABASE_URL").is_err() {
        eprintln!("SKIP: DATABASE_URL not set in CI");
        return;
    }
    let (ca, cakp) = gen_ca();
    let (srv_cert, srv_key) = gen_server_cert("executor-auth", &ca, &cakp);
    let (client_cert_blocked, client_key_blocked) = gen_client_cert("blocked-subj", &ca, &cakp);
    let tmpdir = tempfile::tempdir().unwrap();
    let dir = tmpdir.path().to_path().to_path_buf();
    write_pem_files(&dir, &srv_cert, &srv_key, ca.der());
    let port: u16 = 19200 + (fastrand::u16(..1000));
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--quiet")
        .arg("-p")
        .arg("executor-service")
        .arg("--bin")
        .arg("executor-service")
        .env("RUST_LOG", "info")
        .env("DISABLE_DB", "1")
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://mev:<POSTGRES_PASSWORD>@localhost:5433/mevdb".into()),
        )
        .env("TOKEN_REGISTRY_PORT", port.to_string())
        .env("ADMIN_MTLS_ENABLE", "1")
        .env("ADMIN_TLS_CERT", dir.join("server.crt"))
        .env("ADMIN_TLS_KEY", dir.join("server.key"))
        .env("ADMIN_TLS_CA", dir.join("ca.crt"))
        .env("ADMIN_ALLOWED_SUBJECTS", "allowed-subj")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn executor-service");
    sleep(Duration::from_secs(2)).await;
    // Build client config with blocked subject cert
    let mut root_store = RootCertStore::empty();
    let _ = root_store.add(rustls::pki_types::CertificateDer::from(ca.der().to_vec()));
    let client_config = {
        let cc = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(
                vec![rustls::pki_types::CertificateDer::from(
                    client_cert_blocked.clone(),
                )],
                rustls::pki_types::PrivateKeyDer::Pkcs8(client_key_blocked.clone().into()),
            )
            .unwrap();
        std::sync::Arc::new(cc)
    };
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let connector = tokio_rustls::TlsConnector::from(client_config);
    let mut tls_stream = connector.connect(server_name, stream).await.unwrap();
    // Unauthorized POST
    let req = b"POST /admin/registry/refresh HTTP/1.1\r\nHost: localhost\r\nContent-Length:0\r\nConnection: close\r\n\r\n";
    let _ = tls_stream.write_all(req).await;
    let mut resp = Vec::new();
    let _ = tls_stream.read_to_end(&mut resp).await;
    let txt = String::from_utf8_lossy(&resp);
    assert!(
        txt.contains("401"),
        "expected 401 unauthorized, got: {}",
        txt
    );
    unsafe {
        libc::kill(child.id().unwrap() as i32, libc::SIGINT);
    }
    let _ = child.wait().await;
}
