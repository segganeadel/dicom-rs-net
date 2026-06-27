//! TLS echo round-trip via ApplicationEntity (requires `tls` feature).

#![cfg(feature = "tls")]

use std::net::SocketAddr;
use std::sync::Arc;

use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::CEchoService;
use rcgen::{CertificateParams, DnType, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::net::TcpListener;

fn load_test_tls_configs() -> (Arc<ServerConfig>, Arc<ClientConfig>) {
    let key_pair = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    let cert = params.self_signed(&key_pair).unwrap();
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(key_pair.serialize_der()).unwrap();

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der.clone_key())
        .unwrap();

    let mut roots = RootCertStore::empty();
    roots.add(cert_der).unwrap();
    let client_config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    (Arc::new(server_config), Arc::new(client_config))
}

#[tokio::test(flavor = "multi_thread")]
async fn tls_echo_roundtrip() {
    let (server_tls, client_tls) = load_test_tls_configs();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new()
        .port(addr.port())
        .tls_server_config(server_tls);
    let client_conn = Connection::new()
        .port(addr.port())
        .tls_client_config(client_tls)
        .tls_server_name("localhost");

    let mut device = Device::new();
    let conn_index = device.add_connection(conn.clone());

    let mut scp_ae = ApplicationEntity::new("TLSSCP")
        .acceptor(true)
        .add_connection(conn_index);
    scp_ae.add_default_storage_capabilities();
    scp_ae.register_service(Arc::new(CEchoService::new()));
    device.add_application_entity(scp_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let scu_ae = ApplicationEntity::new("TLSSCU").initiator(true);
    scu_ae
        .echo(&client_conn, 0, &format!("TLSSCP@{}", addr))
        .await
        .unwrap();

    server.abort();
}
