use rcgen::{BasicConstraints, Certificate, CertificateParams, DnType, IsCa, KeyPair};
use rustls::{RootCertStore, ServerConfig};
use std::{fs::File, io::BufReader, sync::Arc};

/// Build a ServerConfig from environment variables ADMIN_TLS_CERT, ADMIN_TLS_KEY, ADMIN_TLS_CA
/// Requires PEM encoded cert chain, PKCS8 private key and CA bundle.
pub fn build_server_config_from_env() -> anyhow::Result<Arc<ServerConfig>> {
    let cert_path = std::env::var("ADMIN_TLS_CERT")?;
    let key_path = std::env::var("ADMIN_TLS_KEY")?;
    let ca_path = std::env::var("ADMIN_TLS_CA")?;
    let mut cert_rd = BufReader::new(File::open(cert_path)?);
    let certs = rustls_pemfile::certs(&mut cert_rd)
        .filter_map(|c| c.ok())
        .collect::<Vec<_>>();
    let mut key_rd = BufReader::new(File::open(key_path)?);
    let key = rustls_pemfile::pkcs8_private_keys(&mut key_rd)
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing pkcs8 key"))??;
    let mut roots = RootCertStore::empty();
    let mut ca_rd = BufReader::new(File::open(ca_path)?);
    for c in rustls_pemfile::certs(&mut ca_rd) {
        if let Ok(b) = c {
            let _ = roots.add(rustls::pki_types::CertificateDer::from(b));
        }
    }
    let client_verifier = rustls::server::WebPkiClientVerifier::builder(roots.into()).build()?;
    let sc = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(
            certs
                .into_iter()
                .map(rustls::pki_types::CertificateDer::from)
                .collect(),
            rustls::pki_types::PrivateKeyDer::Pkcs8(key.into()),
        )?;
    Ok(Arc::new(sc))
}

/// Generate a self-signed CA certificate & key pair
pub fn generate_ca() -> (Certificate, KeyPair) {
    let mut p = CertificateParams::default();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    p.distinguished_name.push(DnType::CommonName, "Test CA");
    let kp = KeyPair::generate().expect("ca key");
    let cert = p.self_signed(&kp).expect("self signed ca");
    (cert, kp)
}

/// Generate a leaf certificate signed by the provided CA
pub fn generate_signed_cert(
    ca: &Certificate,
    ca_kp: &KeyPair,
    cn: &str,
) -> (Certificate, Vec<u8>, Vec<u8>) {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, cn);
    // Add SAN DNS name for hostname validation
    p.subject_alt_names.push(rcgen::SanType::DnsName(
        rcgen::Ia5String::try_from(cn).unwrap(),
    ));
    let kp = KeyPair::generate().expect("leaf key");
    let cert = p.signed_by(&kp, ca, ca_kp).expect("sign leaf");
    (cert, cert.der().to_vec(), kp.serialize_der())
}
