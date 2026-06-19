//! DICOM file metadata for C-STORE SCU.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use dicom_core::header::Tag;
use dicom_encoding::TransferSyntaxIndex;
use dicom_object::OpenFileOptions;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use dicom_ul::pdu::PresentationContextNegotiated;
use walkdir::WalkDir;

use crate::error::{Error, Result};

/// Metadata for a DICOM file to be sent via C-STORE.
#[derive(Debug, Clone)]
pub struct DicomFile {
    /// Path to the file on disk.
    pub path: PathBuf,
    /// Storage SOP class UID.
    pub sop_class_uid: String,
    /// Storage SOP instance UID.
    pub sop_instance_uid: String,
    /// Transfer syntax UID from file meta information.
    pub file_transfer_syntax: String,
    /// Negotiated presentation context (set after association).
    pub presentation_context: Option<PresentationContextNegotiated>,
    /// Selected transfer syntax UID for sending.
    pub transfer_syntax_selected: Option<String>,
}

/// Expands paths (files and directories) and reads DICOM file metadata.
pub fn scan_files(paths: &[PathBuf]) -> Result<Vec<DicomFile>> {
    let mut checked: Vec<PathBuf> = Vec::new();

    for path in paths {
        if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| !e.file_type().is_dir())
            {
                checked.push(entry.into_path());
            }
        } else {
            checked.push(path.clone());
        }
    }

    let mut files = Vec::new();
    for path in checked {
        match read_file_metadata(&path) {
            Ok(file) => files.push(file),
            Err(e) => {
                tracing::warn!("Skipping {}: {e}", path.display());
            }
        }
    }

    if files.is_empty() {
        return Err(Error::ReadFile {
            path: paths
                .first()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            message: "no supported DICOM files found".to_string(),
        });
    }

    Ok(files)
}

fn read_file_metadata(path: &Path) -> Result<DicomFile> {
    if path.file_name() == Some(OsStr::new("DICOMDIR")) {
        return Err(Error::ReadFile {
            path: path.display().to_string(),
            message: "DICOMDIR not supported".to_string(),
        });
    }

    let dicom_file = OpenFileOptions::new()
        .read_until(Tag(0x0001, 0x000))
        .open_file(path)
        .map_err(|e| Error::ReadFile {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;

    let meta = dicom_file.meta();
    let sop_class = meta.media_storage_sop_class_uid.trim_end_matches('\0');
    let sop_instance = meta.media_storage_sop_instance_uid.trim_end_matches('\0');
    let ts_uid = meta.transfer_syntax.trim_end_matches('\0');

    TransferSyntaxRegistry
        .get(ts_uid)
        .ok_or_else(|| Error::UnsupportedTransferSyntax {
            uid: ts_uid.to_string(),
        })?;

    Ok(DicomFile {
        path: path.to_path_buf(),
        sop_class_uid: sop_class.to_string(),
        sop_instance_uid: sop_instance.to_string(),
        file_transfer_syntax: ts_uid.to_string(),
        presentation_context: None,
        transfer_syntax_selected: None,
    })
}
