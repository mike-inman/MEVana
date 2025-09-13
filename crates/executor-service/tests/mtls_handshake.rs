use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use rustls::{RootCertStore, ServerConfig};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use x509_parser::prelude::*;

fn gen_ca() -> (Certificate, KeyPair) {
    let mut p = CertificateParams::default();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name.push(DnType::CommonName, "Test CA");
    let kp = KeyPair::generate().unwrap();
    let cert = p.self_signed(&kp).unwrap();
    (cert, kp)
}

fn gen_cert(
    common_name: &str,
    ca: &Certificate,
    ca_kp: &KeyPair,
) -> (Certificate, Vec<u8>, Vec<u8>) {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, common_name);
    // Add SAN DNS name so rustls hostname validation passes
    p.subject_alt_names.push(rcgen::SanType::DnsName(
        rcgen::Ia5String::try_from(common_name).unwrap(),
    ));
    let kp = KeyPair::generate().unwrap();
    let cert = p.signed_by(&kp, ca, ca_kp).unwrap();
    let cert_der = cert.der().to_vec();
    let key_der = kp.serialize_der();
    (cert, cert_der, key_der)
}

#[tokio::test]
async fn mtls_handshake_logs_client_cn() {
    // Generate CA, server, client
    let (ca_cert, ca_key) = gen_ca();
    let (_server_cert, server_cert_der, server_key_der) =
        gen_cert("server.test", &ca_cert, &ca_key);
    let (_client_cert, client_cert_der, client_key_der) =
        gen_cert("client-test", &ca_cert, &ca_key);

    // Build rustls server config requiring client cert
    let mut roots = RootCertStore::empty();
    let _ = roots.add(rustls::pki_types::CertificateDer::from(
        ca_cert.der().clone(),
    ));
    let client_verifier = rustls::server::WebPkiClientVerifier::builder(roots.into())
        .build()
        .unwrap();
    let scerts = vec![rustls::pki_types::CertificateDer::from(
        server_cert_der.clone(),
    )];
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(server_key_der.clone().into());
    let server_config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(scerts, key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_cn: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_cn_server = captured_cn.clone();

    let server_handle = tokio::spawn(async move {
        if let Ok((tcp, _peer)) = listener.accept().await {
            if let Ok(mut tls) = acceptor.accept(tcp).await {
                if let Some(cert) = tls
                    .get_ref()
                    .1
                    .peer_certificates()
                    .and_then(|v| v.first().cloned())
                {
                    if let Ok((_, parsed)) = parse_x509_certificate(cert.as_ref()) {
                        if let Some(cn_attr) = parsed.subject().iter_common_name().next() {
                            if let Ok(cn_str) = cn_attr.as_str() {
                                *captured_cn_server.lock().unwrap() = Some(cn_str.to_string());
                            }
                        }
                    }
                }
                let _ = tls
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length:2\r\nConnection: close\r\n\r\nOK",
                    )
                    .await;
            }
        }
    });

    // Build client config
    let mut root_store = RootCertStore::empty();
    let _ = root_store.add(rustls::pki_types::CertificateDer::from(
        ca_cert.der().clone(),
    ));
    let client_config = {
        let cc = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(
                vec![rustls::pki_types::CertificateDer::from(
                    client_cert_der.clone(),
                )],
                rustls::pki_types::PrivateKeyDer::Pkcs8(client_key_der.clone().into()),
            )
            .unwrap();
        Arc::new(cc)
    };
    let server_name = rustls::pki_types::ServerName::try_from("server.test").unwrap();
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let connector = tokio_rustls::TlsConnector::from(client_config);
    let mut tls_stream = connector.connect(server_name, stream).await.unwrap();
    // Send minimal HTTP request
    let _ = tls_stream
        .write_all(b"GET / HTTP/1.1\r\nHost: server.test\r\nConnection: close\r\n\r\n")
        .await;
    let mut resp = Vec::new();
    let _ = tls_stream.read_to_end(&mut resp).await;

    server_handle.await.unwrap();
    let body = String::from_utf8_lossy(&resp);
    assert!(body.contains("200 OK"), "response missing 200 OK: {body}");
    let cn = captured_cn.lock().unwrap().clone();
    assert_eq!(cn.as_deref(), Some("client-test"));
}
