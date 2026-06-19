//! Integration test: in-process SCU sends C-STORE to dicom-net SCP.

use std::sync::Arc;

use dicom_core::{dicom_value, DataElement, VR};
use dicom_dictionary_std::{tags, uids::MR_IMAGE_STORAGE};
use dicom_net::device::DeviceBuilder;
use dicom_net::scp::{CEchoService, CStoreService, FileCStoreSink};
use dicom_object::{InMemDicomObject, OpenFileOptions, StandardDataDictionary};
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use dicom_ul::association::client::AsyncClientAssociation;
use tokio::net::TcpListener;

const SOP_INSTANCE_UID: &str = "1.2.3.4.5.6.7.8.9.0.1.2.3";

fn build_test_dataset() -> Vec<u8> {
    let obj = InMemDicomObject::from_element_iter([
        DataElement::new(tags::SOP_CLASS_UID, VR::UI, dicom_value!(Str, MR_IMAGE_STORAGE)),
        DataElement::new(tags::SOP_INSTANCE_UID, VR::UI, dicom_value!(Str, SOP_INSTANCE_UID)),
        DataElement::new(tags::PATIENT_NAME, VR::PN, dicom_value!(Str, "TEST^PATIENT")),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts).unwrap();
    data
}

fn build_cstore_command(message_id: u16) -> Vec<u8> {
    let cmd = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, MR_IMAGE_STORAGE),
        ),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0001])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0000]),
        ),
        DataElement::new(
            tags::AFFECTED_SOP_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, SOP_INSTANCE_UID),
        ),
    ]);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    cmd.write_dataset_with_ts(&mut data, &ts).unwrap();
    data
}

#[tokio::test(flavor = "multi_thread")]
async fn cstore_scp_receives_and_stores_instance() {
    let output_dir = tempfile::tempdir().unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let server = {
        let output_dir = output_dir.path().to_path_buf();
        tokio::spawn(async move {
            let sink = FileCStoreSink::new(output_dir);
            let _ = DeviceBuilder::new()
                .ae_title("TESTSCP")
                .bind(addr)
                .register_service(Arc::new(CEchoService::new()))
                .register_cstore(Arc::new(CStoreService::new(sink)))
                .run()
                .await;
        })
    };

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut scu: AsyncClientAssociation<_> = ClientAssociationOptions::new()
        .calling_ae_title("TESTSCU")
        .called_ae_title("TESTSCP")
        .with_presentation_context(
            MR_IMAGE_STORAGE,
            vec![IMPLICIT_VR_LITTLE_ENDIAN.uid()],
        )
        .establish_async(addr)
        .await
        .expect("association");

    let pc_id = scu.presentation_contexts()[0].id;
    let cmd_data = build_cstore_command(1);
    let object_data = build_test_dataset();

    let pdu = Pdu::PData {
        data: vec![
            PDataValue {
                presentation_context_id: pc_id,
                value_type: PDataValueType::Command,
                is_last: true,
                data: cmd_data,
            },
            PDataValue {
                presentation_context_id: pc_id,
                value_type: PDataValueType::Data,
                is_last: true,
                data: object_data,
            },
        ],
    };
    scu.send(&pdu).await.expect("send C-STORE");

    let rsp = scu.receive().await.expect("receive response");
    match rsp {
        Pdu::PData { data } => {
            let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
            let obj = InMemDicomObject::<StandardDataDictionary>::read_dataset_with_ts(
                data[0].data.as_slice(),
                &ts,
            )
            .unwrap();
            let status = obj.element(tags::STATUS).unwrap().to_int::<u16>().unwrap();
            assert_eq!(status, 0x0000);
        }
        other => panic!("unexpected response: {other:?}"),
    }

    scu.send(&Pdu::ReleaseRQ).await.unwrap();
    let _ = scu.receive().await;

    let stored_path = output_dir.path().join(format!("{SOP_INSTANCE_UID}.dcm"));
    assert!(stored_path.exists(), "stored file should exist");

    let stored = OpenFileOptions::new().open_file(&stored_path).unwrap();
    let meta = stored.meta();
    assert_eq!(
        meta.media_storage_sop_instance_uid.trim_end_matches('\0'),
        SOP_INSTANCE_UID
    );

    server.abort();
}
