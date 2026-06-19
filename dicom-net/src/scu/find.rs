//! C-FIND SCU operations.

use dicom_core::{DataElement, VR, dicom_value};
use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::entries::IMPLICIT_VR_LITTLE_ENDIAN;

use crate::dimse::request::build_cfind_rq;
use crate::dimse::rsp::ensure_success;
use crate::error::{Error, Result};
use crate::qr::STUDY_ROOT_FIND;
use crate::scu::association::ScuAssociation;

impl ScuAssociation {
    /// Performs C-FIND and collects all match datasets.
    pub async fn find(&mut self, identifier: &[u8]) -> Result<Vec<Vec<u8>>> {
        let pc = self
            .find_context(STUDY_ROOT_FIND)
            .ok_or_else(|| Error::NoPresentationContext {
                sop_class: STUDY_ROOT_FIND.to_string(),
            })?;

        let message_id = self.next_message_id();
        let cmd = build_cfind_rq(STUDY_ROOT_FIND, message_id)?;
        self.send_command(pc.id, cmd).await?;
        self.send_data(pc.id, identifier).await?;

        let mut matches = Vec::new();
        loop {
            let (response, data) = self.receive_with_optional_data().await?;
            if response.status.is_pending() {
                if let Some(dataset) = data {
                    matches.push(dataset);
                }
            } else {
                ensure_success(response)?;
                break;
            }
        }
        Ok(matches)
    }
}

/// Builds a minimal Study-level C-FIND identifier.
pub fn build_study_find_identifier(patient_id: Option<&str>) -> Result<Vec<u8>> {
    let mut elements = vec![DataElement::new(
        tags::QUERY_RETRIEVE_LEVEL,
        VR::CS,
        dicom_value!(Str, "STUDY"),
    )];
    if let Some(pid) = patient_id {
        elements.push(DataElement::new(
            tags::PATIENT_ID,
            VR::LO,
            dicom_value!(Str, pid),
        ));
    }
    elements.push(DataElement::new(
        tags::PATIENT_NAME,
        VR::PN,
        dicom_value!(Str, ""),
    ));
    elements.push(DataElement::new(
        tags::STUDY_INSTANCE_UID,
        VR::UI,
        dicom_value!(Str, ""),
    ));
    let obj = InMemDicomObject::from_element_iter(elements);
    let ts = IMPLICIT_VR_LITTLE_ENDIAN.erased();
    let mut data = Vec::new();
    obj.write_dataset_with_ts(&mut data, &ts)
        .map_err(|e| Error::InvalidCommand {
            message: e.to_string(),
        })?;
    Ok(data)
}
