//! Integration test: C-ECHO round-trip via dicom-net SCU client.

use std::net::SocketAddr;
use std::sync::Arc;

use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::CEchoService;
use dicom_net::scu::Client;
use tokio::net::TcpListener;

#[tokio::test(flavor = "multi_thread")]
async fn echo_roundtrip_via_client() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn);

    let mut ae = ApplicationEntity::new("TESTSCP")
        .acceptor(true)
        .add_connection(conn_index);
    ae.add_default_storage_capabilities();
    ae.register_service(Arc::new(CEchoService::new()));
    device.add_application_entity(ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Client::new()
        .calling_ae("TESTSCU")
        .called_ae("TESTSCP")
        .remote(format!("TESTSCP@{}", addr))
        .echo()
        .await
        .expect("C-ECHO should succeed");

    server.abort();
}
