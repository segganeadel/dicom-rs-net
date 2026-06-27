//! Integration test: Study Root C-FIND round-trip via ApplicationEntity.

use std::net::SocketAddr;
use std::sync::Arc;

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_net::device::{ApplicationEntity, Connection, Device, TransferCapability};
use dicom_net::qr::STUDY_ROOT_FIND;
use dicom_net::scp::{CFindService, StaticCFindSink};
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tokio::net::TcpListener;

#[tokio::test(flavor = "multi_thread")]
async fn find_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn.clone());

    let match_obj = InMemDicomObject::from_element_iter([
        DataElement::new(tags::PATIENT_NAME, VR::PN, dicom_value!(Str, "FIND^TEST")),
        DataElement::new(
            tags::STUDY_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, "1.2.3.4.5"),
        ),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut match_bytes = Vec::new();
    match_obj
        .write_dataset_with_ts(&mut match_bytes, &ts)
        .unwrap();

    let mut scp_ae = ApplicationEntity::new("FINDSCP")
        .acceptor(true)
        .add_connection(conn_index);
    let mut find_cap = TransferCapability::query_retrieve_find_scp(STUDY_ROOT_FIND);
    find_cap.transfer_syntaxes =
        vec![dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string()];
    scp_ae.add_scp_capability(find_cap);
    scp_ae.register_cfind(Arc::new(CFindService::new(Arc::new(StaticCFindSink::new(
        vec![match_bytes],
    )))));
    device.add_application_entity(scp_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut scu_ae = ApplicationEntity::new("FINDSCU").initiator(true);
    let mut find_cap = TransferCapability::query_retrieve_find_scu(STUDY_ROOT_FIND);
    find_cap.transfer_syntaxes =
        vec![dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string()];
    scu_ae.add_scu_capability(find_cap);

    let matches = scu_ae
        .find(&conn, 0, &format!("FINDSCP@{}", addr), None)
        .await
        .unwrap();

    assert_eq!(matches.len(), 1);
    server.abort();
}
