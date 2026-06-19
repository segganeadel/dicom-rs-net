//! DICOM transfer syntax transcoding for C-STORE SCU.

use dicom_encoding::transfer_syntax::TransferSyntax;
use dicom_object::DefaultDicomObject;

use crate::error::{Error, Result};

/// Converts a DICOM object to the target transfer syntax when needed.
#[cfg(feature = "transcode")]
pub fn into_ts(
    dicom_file: DefaultDicomObject,
    ts_selected: &TransferSyntax,
    verbose: bool,
) -> Result<DefaultDicomObject> {
    if ts_selected.uid() != dicom_file.meta().transfer_syntax() {
        use dicom_pixeldata::Transcode;
        let mut file = dicom_file;
        if verbose {
            tracing::info!(
                "Transcoding file from {} to {}",
                file.meta().transfer_syntax(),
                ts_selected.uid()
            );
        }
        file.transcode(ts_selected).map_err(|e| Error::Transcode {
            message: e.to_string(),
        })?;
        Ok(file)
    } else {
        Ok(dicom_file)
    }
}

/// Converts a DICOM object to the target transfer syntax when needed.
#[cfg(not(feature = "transcode"))]
pub fn into_ts(
    dicom_file: DefaultDicomObject,
    ts_selected: &TransferSyntax,
    _verbose: bool,
) -> Result<DefaultDicomObject> {
    if ts_selected.uid() != dicom_file.meta().transfer_syntax() {
        Err(Error::TranscodeRequired {
            message: format!(
                "file uses {} but negotiated {}",
                dicom_file.meta().transfer_syntax(),
                ts_selected.uid()
            ),
        })
    } else {
        Ok(dicom_file)
    }
}
