//! DIMSE request command encoding.

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::{tags, uids};
use dicom_object::{InMemDicomObject, StandardDataDictionary};
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;

use crate::error::{Error, Result};

fn encode_command(obj: &InMemDicomObject<StandardDataDictionary>) -> Result<Vec<u8>> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts)
        .map_err(|e| Error::InvalidCommand {
            message: e.to_string(),
        })?;
    Ok(data)
}

/// Encodes a C-ECHO-RQ command dataset.
pub fn build_cecho_rq(message_id: u16) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(tags::AFFECTED_SOP_CLASS_UID, VR::UI, uids::VERIFICATION),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0030])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0101]),
        ),
    ]);
    encode_command(&obj)
}

/// Encodes a C-STORE-RQ command dataset.
pub fn build_cstore_rq(
    sop_class_uid: &str,
    sop_instance_uid: &str,
    message_id: u16,
) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, sop_class_uid),
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
            dicom_value!(Str, sop_instance_uid),
        ),
    ]);
    encode_command(&obj)
}

/// Encodes a C-FIND-RQ command dataset.
pub fn build_cfind_rq(sop_class_uid: &str, message_id: u16) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, sop_class_uid),
        ),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0020])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0000]),
        ),
    ]);
    encode_command(&obj)
}

/// Encodes a C-MOVE-RQ command dataset.
pub fn build_cmove_rq(
    sop_class_uid: &str,
    message_id: u16,
    move_destination: &str,
) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, sop_class_uid),
        ),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0021])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0000]),
        ),
        DataElement::new(
            tags::MOVE_DESTINATION,
            VR::AE,
            dicom_value!(Str, move_destination),
        ),
    ]);
    encode_command(&obj)
}

/// Encodes a C-GET-RQ command dataset.
pub fn build_cget_rq(sop_class_uid: &str, message_id: u16) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            dicom_value!(Str, sop_class_uid),
        ),
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0010])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0000]),
        ),
    ]);
    encode_command(&obj)
}

/// Encodes a C-CANCEL-RQ command dataset.
pub fn build_ccancel_rq(message_id: u16) -> Result<Vec<u8>> {
    let obj = InMemDicomObject::command_from_element_iter([
        DataElement::new(tags::COMMAND_FIELD, VR::US, dicom_value!(U16, [0x0FFF])),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(
            tags::COMMAND_DATA_SET_TYPE,
            VR::US,
            dicom_value!(U16, [0x0101]),
        ),
    ]);
    encode_command(&obj)
}
