//! Integration test: ApplicationEntity SCU methods against an in-process Device.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::{tags, uids::MR_IMAGE_STORAGE};
use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::{CEchoService, CStoreService, FileCStoreSink};
use dicom_net::scu::StoreOptions;
use dicom_object::{InMemDicomObject, OpenFileOptions};
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tokio::net::TcpListener;

const SOP_INSTANCE_UID: &str = "1.2.3.4.5.6.7.8.9.0.1.2.3";

fn write_test_file(path: &std::path::Path) {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(
            tags::SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, MR_IMAGE_STORAGE),
        ),
        DataElement::new(
            tags::SOP_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, SOP_INSTANCE_UID),
        ),
        DataElement::new(
            tags::PATIENT_NAME,
            VR::PN,
            dicom_value!(Str, "TEST^PATIENT"),
        ),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let file_meta = dicom_object::FileMetaTableBuilder::new()
        .media_storage_sop_class_uid(MR_IMAGE_STORAGE)
        .media_storage_sop_instance_uid(SOP_INSTANCE_UID)
        .transfer_syntax(ts.uid())
        .build()
        .unwrap();
    let file_obj = obj.with_exact_meta(file_meta);
    file_obj.write_to_file(path).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn application_entity_scu_echo_and_store() {
    let output_dir = tempfile::tempdir().unwrap();
    let input_dir = tempfile::tempdir().unwrap();
    let input_path = input_dir.path().join("test.dcm");
    write_test_file(&input_path);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn.clone());

    let out_path = output_dir.path().to_path_buf();
    let mut scp_ae = ApplicationEntity::new("TESTSCP")
        .acceptor(true)
        .add_connection(conn_index);
    scp_ae.add_default_storage_capabilities();
    scp_ae.register_service(Arc::new(CEchoService::new()));
    scp_ae.register_cstore(Arc::new(CStoreService::new(FileCStoreSink::new(out_path))));
    device.add_application_entity(scp_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let scu_ae = ApplicationEntity::new("TESTSCU").initiator(true);
    let remote = format!("TESTSCP@{}", addr);

    scu_ae
        .echo(&conn, &remote)
        .await
        .expect("ApplicationEntity::echo should succeed");

    let mut scu_ae = ApplicationEntity::new("TESTSCU").initiator(true);
    scu_ae
        .add_scu_storage_capabilities_from_files(&[PathBuf::from(&input_path)], false)
        .unwrap();

    let sent = scu_ae
        .store_files(
            &conn,
            &remote,
            &[PathBuf::from(input_path)],
            &StoreOptions::default(),
        )
        .await
        .expect("ApplicationEntity::store_files should succeed");

    assert_eq!(sent, 1);

    let stored_path = output_dir.path().join(format!("{SOP_INSTANCE_UID}.dcm"));
    assert!(stored_path.exists());

    let stored = OpenFileOptions::new().open_file(&stored_path).unwrap();
    assert_eq!(
        stored
            .meta()
            .media_storage_sop_instance_uid
            .trim_end_matches('\0'),
        SOP_INSTANCE_UID
    );

    server.abort();
}
