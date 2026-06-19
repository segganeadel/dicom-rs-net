//! DIMSE response command parsing.

use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;

use crate::error::{Error, Result};
use crate::status::Status;

/// Parsed fields from a DIMSE response command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimseResponse {
    /// Response status code.
    pub status: Status,
    /// Message ID being responded to.
    pub message_id: u16,
}

/// Parses a DIMSE response command dataset.
pub fn parse_response(bytes: &[u8]) -> Result<DimseResponse> {
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let obj =
        InMemDicomObject::read_dataset_with_ts(bytes, &ts).map_err(|e| Error::InvalidCommand {
            message: e.to_string(),
        })?;

    let status = obj
        .element(tags::STATUS)
        .map_err(|_| Error::InvalidCommand {
            message: "missing Status in response".to_string(),
        })?
        .to_int::<u16>()
        .map_err(|_| Error::InvalidCommand {
            message: "Status is not an integer".to_string(),
        })?;

    let message_id = obj
        .element(tags::MESSAGE_ID_BEING_RESPONDED_TO)
        .map_err(|_| Error::InvalidCommand {
            message: "missing Message ID Being Responded To".to_string(),
        })?
        .to_int::<u16>()
        .map_err(|_| Error::InvalidCommand {
            message: "Message ID Being Responded To is not an integer".to_string(),
        })?;

    Ok(DimseResponse {
        status: Status(status),
        message_id,
    })
}

/// Ensures the response status indicates success or warning; returns `DimseFailure` otherwise.
pub fn ensure_success(response: DimseResponse) -> Result<DimseResponse> {
    if response.status.is_success() || response.status.is_warning() {
        Ok(response)
    } else {
        Err(Error::DimseFailure {
            status: response.status.0,
        })
    }
}
