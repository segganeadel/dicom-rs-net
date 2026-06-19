//! Integration test: multi-AE routing on a single TCP port.

use std::net::SocketAddr;
use std::sync::Arc;

use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::CEchoService;
use dicom_net::scu::Client;
use tokio::net::TcpListener;

#[tokio::test(flavor = "multi_thread")]
async fn multi_ae_routes_by_called_title() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn);

    let mut store_ae = ApplicationEntity::new("STORESCP")
        .acceptor(true)
        .add_connection(conn_index);
    store_ae.add_default_storage_capabilities();
    store_ae.register_service(Arc::new(CEchoService::new()));
    device.add_application_entity(store_ae);

    let mut worklist_ae = ApplicationEntity::new("WORKLIST")
        .acceptor(true)
        .add_connection(conn_index);
    worklist_ae.add_default_storage_capabilities();
    worklist_ae.register_service(Arc::new(CEchoService::new()));
    device.add_application_entity(worklist_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    Client::new()
        .calling_ae("TESTSCU")
        .called_ae("STORESCP")
        .remote(format!("STORESCP@{}", addr))
        .echo()
        .await
        .expect("C-ECHO to STORESCP should succeed");

    Client::new()
        .calling_ae("TESTSCU")
        .called_ae("WORKLIST")
        .remote(format!("WORKLIST@{}", addr))
        .echo()
        .await
        .expect("C-ECHO to WORKLIST should succeed");

    let err = Client::new()
        .calling_ae("TESTSCU")
        .called_ae("UNKNOWN")
        .remote(format!("UNKNOWN@{}", addr))
        .echo()
        .await
        .expect_err("unknown called AE should fail");

    assert!(
        err.to_string().contains("upper layer") || err.to_string().contains("Ul"),
        "unexpected error: {err}"
    );

    server.abort();
}
