//! DIMSE response command encoding.

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, StandardDataDictionary};
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;

use crate::error::{Error, Result};
use crate::status::Status;

/// Encodes a C-ECHO-RSP command dataset.
pub fn build_cecho_rsp(message_id: u16, status: Status) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x8030])),
        DataElement::new(
            tags::MESSAGE_ID_BEING_RESPONDED_TO,
            VR::US,
            dicom_value!(U16, [message_id]),
        ),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0101]),
        ),
        DataElement::new(tags::STATUS, VR::US, dicom_value!(U16, [status.0])),
    ]);
    encode_command(&obj)
}

/// Encodes a C-STORE-RSP command dataset.
pub fn build_cstore_rsp(
    message_id: u16,
    sop_class_uid: &str,
    sop_instance_uid: &str,
    status: Status,
) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, sop_class_uid),
        ),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x8001])),
        DataElement::new(
            tags::MESSAGE_ID_BEING_RESPONDED_TO,
            VR::US,
            dicom_value!(U16, [message_id]),
        ),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0101]),
        ),
        DataElement::new(tags::STATUS, VR::US, dicom_value!(U16, [status.0])),
        DataElement::new(
            tags::AFFECTED_SOP_INSTANCE_UID,
            VR::UI,
            dicom_value!(Str, sop_instance_uid),
        ),
    ]);
    encode_command(&obj)
}

/// Sub-operation counts for C-MOVE / C-GET responses.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubOperationCounts {
    /// Remaining sub-operations.
    pub remaining: u16,
    /// Completed sub-operations.
    pub completed: u16,
    /// Failed sub-operations.
    pub failed: u16,
    /// Warning sub-operations.
    pub warning: u16,
}

/// Encodes a C-FIND-RSP command dataset.
pub fn build_cfind_rsp(message_id: u16, status: Status) -> Result<Vec<u8>> {
    build_qr_rsp(0x8020, message_id, status, None)
}

/// Encodes a C-MOVE-RSP command dataset.
pub fn build_cmove_rsp(
    message_id: u16,
    status: Status,
    counts: Option<SubOperationCounts>,
) -> Result<Vec<u8>> {
    build_qr_rsp(0x8021, message_id, status, counts)
}

/// Encodes a C-GET-RSP command dataset.
pub fn build_cget_rsp(
    message_id: u16,
    status: Status,
    counts: Option<SubOperationCounts>,
) -> Result<Vec<u8>> {
    build_qr_rsp(0x8010, message_id, status, counts)
}

fn build_qr_rsp(
    command_field: u16,
    message_id: u16,
    status: Status,
    counts: Option<SubOperationCounts>,
) -> Result<Vec<u8>> {
    let mut elements = vec![
        DataElement::new(
            tags::COMMAND_FIELD,
            VR::US,
            dicom_value!(U16, [command_field]),
        ),
        DataElement::new(
            tags::MESSAGE_ID_BEING_RESPONDED_TO,
            VR::US,
            dicom_value!(U16, [message_id]),
        ),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [if status.is_pending() { 0x0000 } else { 0x0101 }]),
        ),
        DataElement::new(tags::STATUS, VR::US, dicom_value!(U16, [status.0])),
    ];

    if let Some(counts) = counts {
        elements.push(DataElement::new(
            tags::NUMBER_OF_REMAINING_SUBOPERATIONS,
            VR::US,
            dicom_value!(U16, [counts.remaining]),
        ));
        elements.push(DataElement::new(
            tags::NUMBER_OF_COMPLETED_SUBOPERATIONS,
            VR::US,
            dicom_value!(U16, [counts.completed]),
        ));
        elements.push(DataElement::new(
            tags::NUMBER_OF_FAILED_SUBOPERATIONS,
            VR::US,
            dicom_value!(U16, [counts.failed]),
        ));
        elements.push(DataElement::new(
            tags::NUMBER_OF_WARNING_SUBOPERATIONS,
            VR::US,
            dicom_value!(U16, [counts.warning]),
        ));
    }

    let obj = InMemDicomObject::command_from_element_iter(elements);
    encode_command(&obj)
}

fn encode_command(obj: &InMemDicomObject<StandardDataDictionary>) -> Result<Vec<u8>> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts)
        .map_err(|e| Error::EncodeResponse {
            message: e.to_string(),
        })?;
    Ok(data)
}
