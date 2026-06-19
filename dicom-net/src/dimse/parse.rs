//! DIMSE command dataset parsing.

use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;

use crate::dimse::{CommandField, DimseMessage};
use crate::error::{Error, Result};

/// Parses a DIMSE command dataset from raw bytes.
pub fn parse_command(
    bytes: &[u8],
    presentation_context_id: u8,
) -> Result<DimseMessage> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let obj = InMemDicomObject::read_dataset_with_ts(bytes, &ts).map_err(|e| {
        Error::InvalidCommand {
            message: e.to_string(),
        }
    })?;

    let command_field_raw = obj
        .element(tags::COMMAND_FIELD)
        .map_err(|_| Error::InvalidCommand {
            message: "missing Command Field".to_string(),
        })?
        .uint16()
        .map_err(|_| Error::InvalidCommand {
            message: "Command Field is not an integer".to_string(),
        })?;

    let command_field = CommandField::from_u16(command_field_raw).ok_or_else(|| {
        Error::InvalidCommand {
            message: format!("unknown command field {command_field_raw:#06x}"),
        }
    })?;

    let message_id = obj
        .element(tags::MESSAGE_ID)
        .map_err(|_| Error::InvalidCommand {
            message: "missing Message ID".to_string(),
        })?
        .to_int::<u16>()
        .map_err(|_| Error::InvalidCommand {
            message: "Message ID is not an integer".to_string(),
        })? as u16;

    let affected_sop_class_uid = obj
        .element(tags::AFFECTED_SOP_CLASS_UID)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.trim_end_matches('\0').to_string());

    let affected_sop_instance_uid = obj
        .element(tags::AFFECTED_SOP_INSTANCE_UID)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.trim_end_matches('\0').to_string());

    Ok(DimseMessage {
        command_field,
        dimse: command_field.dimse(),
        message_id,
        presentation_context_id,
        affected_sop_class_uid,
        affected_sop_instance_uid,
        command: bytes.to_vec(),
    })
}
