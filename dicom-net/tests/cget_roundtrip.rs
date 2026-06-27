//! Integration test: Study Root C-GET round-trip.

use std::path::PathBuf;
use std::sync::Arc;

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::{tags, uids::MR_IMAGE_STORAGE};
use dicom_net::association::FileRetrieveSink;
use dicom_net::device::{ApplicationEntity, Connection, Device, TransferCapability};
use dicom_net::qr::STUDY_ROOT_GET;
use dicom_net::scp::CGetService;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use tokio::net::TcpListener;

const SOP_INSTANCE_UID: &str = "1.2.3.4.5.6.7.8.9.0.1.2.3.5";

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

fn build_image_get_identifier() -> Vec<u8> {
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

#[tokio::test(flavor = "multi_thread")]
async fn cget_roundtrip() {
    let input_dir = tempfile::tempdir().unwrap();
    let input_path = input_dir.path().join("test.dcm");
    write_test_file(&input_path);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let conn = Connection::new().port(addr.port());
    let mut device = Device::new();
    let conn_index = device.add_connection(conn.clone());

    let mut qr_ae = ApplicationEntity::new("GETSCP")
        .acceptor(true)
        .add_connection(conn_index);
    qr_ae.add_scp_capability(TransferCapability::query_retrieve_get_scp(STUDY_ROOT_GET));
    qr_ae.add_default_storage_capabilities();
    qr_ae.register_cget(Arc::new(CGetService::new(Arc::new(FileRetrieveSink::new(
        vec![PathBuf::from(input_path)],
    )))));
    device.add_application_entity(qr_ae);

    let server = tokio::spawn(async move {
        let _ = Arc::new(device).bind_connections().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut scu_ae = ApplicationEntity::new("GETSCU").initiator(true);
    scu_ae.add_scu_capability(TransferCapability::query_retrieve_get_scu(STUDY_ROOT_GET));
    scu_ae.add_scu_capability(TransferCapability::storage_scu(
        MR_IMAGE_STORAGE,
        vec![dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string()],
    ));

    let counts = scu_ae
        .get_instances(
            &conn,
            0,
            &format!("GETSCP@{}", addr),
            &build_image_get_identifier(),
        )
        .await
        .unwrap();

    assert_eq!(counts.completed, 1);
    server.abort();
}
