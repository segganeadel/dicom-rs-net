//! Presentation context selection for negotiated associations.

use dicom_dictionary_std::uids;
use dicom_encoding::TransferSyntaxIndex;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use dicom_ul::pdu::PresentationContextNegotiated;

use crate::error::{Error, Result};
use crate::scu::file::DicomFile;

/// Selects a negotiated presentation context and transfer syntax for a file.
pub fn select_presentation_context(
    file: &DicomFile,
    contexts: &[PresentationContextNegotiated],
    never_transcode: bool,
) -> Result<(PresentationContextNegotiated, String)> {
    let file_ts = TransferSyntaxRegistry
        .get(&file.file_transfer_syntax)
        .ok_or_else(|| Error::UnsupportedTransferSyntax {
            uid: file.file_transfer_syntax.clone(),
        })?;

    if let Some(pc) = contexts
        .iter()
        .filter(|pc| pc.abstract_syntax == file.sop_class_uid)
        .find(|pc| pc.transfer_syntax == file_ts.uid())
    {
        return Ok((pc.clone(), pc.transfer_syntax.clone()));
    }

    let pc = contexts.iter().find(|pc| {
        if pc.abstract_syntax != file.sop_class_uid {
            return false;
        }
        let ts = &pc.transfer_syntax;
        ts == file_ts.uid()
            || TransferSyntaxRegistry
                .get(ts)
                .filter(|ts| file_ts.is_codec_free() && ts.is_codec_free())
                .is_some()
    });

    if let Some(pc) = pc {
        return Ok((pc.clone(), pc.transfer_syntax.clone()));
    }

    if never_transcode || !file_ts.can_decode_all() {
        return Err(Error::NoPresentationContext {
            sop_class: file.sop_class_uid.clone(),
        });
    }

    let pc = contexts
        .iter()
        .filter(|pc| pc.abstract_syntax == file.sop_class_uid)
        .find(|pc| pc.transfer_syntax == uids::EXPLICIT_VR_LITTLE_ENDIAN)
        .or_else(|| {
            contexts
                .iter()
                .filter(|pc| pc.abstract_syntax == file.sop_class_uid)
                .find(|pc| pc.transfer_syntax == uids::IMPLICIT_VR_LITTLE_ENDIAN)
        })
        .ok_or_else(|| Error::NoPresentationContext {
            sop_class: file.sop_class_uid.clone(),
        })?;

    Ok((pc.clone(), pc.transfer_syntax.clone()))
}
