//! Integration test: Study Root C-MOVE round-trip with in-process destination SCP.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::{tags, uids::MR_IMAGE_STORAGE};
use dicom_net::association::FileRetrieveSink;
use dicom_net::device::{ApplicationEntity, Connection, Device, TransferCapability};
use dicom_net::qr::STUDY_ROOT_MOVE;
use dicom_net::scp::{CMoveService, CStoreService, FileCStoreSink};
use dicom_object::{InMemDicomObject, OpenFileOptions};
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tokio::net::TcpListener;

const SOP_INSTANCE_UID: &str = "1.2.3.4.5.6.7.8.9.0.1.2.3.4";

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
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let file_meta = dicom_object::FileMetaTableBuilder::new()
        .media_storage_sop_class_uid(MR_IMAGE_STORAGE)
        .media_storage_sop_instance_uid(SOP_INSTANCE_UID)
        .transfer_syntax(ts.uid())
        .build()
        .unwrap();
    obj.with_exact_meta(file_meta).write_to_file(path).unwrap();
}

fn build_image_move_identifier() -> Vec<u8> {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(
            tags::QUERY_RETRIEVE_LEVEL,
            VR::CS,
            dicom_value!(Str, "IMAGE"),
        ),
        DataElement::new(
            tags::SOP_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, SOP_INSTANCE_UID),
        ),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts).unwrap();
    data
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cmove_roundtrip() {
    let output_dir = tempfile::tempdir().unwrap();
    let input_dir = tempfile::tempdir().unwrap();
    let input_path = input_dir.path().join("test.dcm");
    write_test_file(&input_path);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn.clone());

    let out_path = output_dir.path().to_path_buf();
    let mut dest_ae = ApplicationEntity::new("DESTSCP")
        .acceptor(true)
        .add_connection(conn_index);
    dest_ae.add_default_storage_capabilities();
    dest_ae.register_cstore(Arc::new(CStoreService::new(Arc::new(FileCStoreSink::new(
        out_path.clone(),
    )))));
    device.add_application_entity(dest_ae);

    let mut destinations = HashMap::new();
    destinations.insert("DESTSCP".to_string(), format!("DESTSCP@{}", addr));

    let mut qr_ae = ApplicationEntity::new("QRSCP")
        .acceptor(true)
        .add_connection(conn_index);
    qr_ae.add_scp_capability(TransferCapability::query_retrieve_move_scp(STUDY_ROOT_MOVE));
    qr_ae.add_default_storage_capabilities();
    qr_ae.register_cmove(Arc::new(CMoveService::new(
        Arc::new(FileRetrieveSink::new(vec![PathBuf::from(input_path)])),
        destinations,
    )));
    device.add_application_entity(qr_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let mut scu_ae = ApplicationEntity::new("MOVESCU").initiator(true);
    scu_ae.add_scu_capability(TransferCapability::query_retrieve_move_scu(STUDY_ROOT_MOVE));

    let identifier = build_image_move_identifier();
    scu_ae
        .move_instances(&conn, 0, &format!("QRSCP@{}", addr), &identifier, "DESTSCP")
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let stored = output_dir.path().join(format!("{SOP_INSTANCE_UID}.dcm"));
    let obj = OpenFileOptions::new().open_file(&stored).unwrap();
    assert_eq!(
        obj.element(tags::SOP_INSTANCE_UID)
            .unwrap()
            .to_str()
            .unwrap(),
        SOP_INSTANCE_UID
    );

    server.abort();
}
