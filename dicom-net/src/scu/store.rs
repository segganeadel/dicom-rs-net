//! C-STORE SCU operations.

use dicom_encoding::TransferSyntaxIndex;
use dicom_object::open_file;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};

use crate::dimse::request::{build_cecho_rq, build_cstore_rq};
use crate::dimse::rsp::ensure_success;
use crate::error::{Error, Result};
use crate::scu::association::ScuAssociation;
use crate::scu::file::DicomFile;
use crate::scu::presentation_context::select_presentation_context;
use crate::scu::transcode::into_ts;

/// Options controlling C-STORE SCU behaviour.
#[derive(Debug, Clone, Default)]
pub struct StoreOptions {
    /// Stop on first failure.
    pub fail_first: bool,
    /// Refuse transcoding.
    pub never_transcode: bool,
    /// Verbose logging.
    pub verbose: bool,
}

impl ScuAssociation {
    /// Performs C-ECHO on an established association with Verification SOP.
    pub async fn echo(&mut self) -> Result<()> {
        let pc = self
            .verification_context()
            .ok_or_else(|| Error::NoPresentationContext {
                sop_class: "1.2.840.10008.1.1".to_string(),
            })?;

        let message_id = self.next_message_id();
        let cmd = build_cecho_rq(message_id)?;
        self.send_command(pc.id, cmd).await?;
        let response = self.receive_response().await?;
        ensure_success(response)?;
        Ok(())
    }

    /// Sends one or more DICOM files via C-STORE.
    pub async fn store_files(
        &mut self,
        files: &mut [DicomFile],
        options: &StoreOptions,
    ) -> Result<usize> {
        let contexts = self.presentation_contexts().to_vec();
        let mut sent = 0usize;

        for file in files.iter_mut() {
            match self.store_file(file, &contexts, options).await {
                Ok(()) => sent += 1,
                Err(e) => {
                    if options.fail_first {
                        return Err(e);
                    }
                    tracing::warn!("Failed to store {}: {e}", file.path.display());
                }
            }
        }

        Ok(sent)
    }

    async fn store_file(
        &mut self,
        file: &mut DicomFile,
        contexts: &[dicom_ul::pdu::PresentationContextNegotiated],
        options: &StoreOptions,
    ) -> Result<()> {
        let (pc, ts_uid) = select_presentation_context(file, contexts, options.never_transcode)?;
        let ts_selected = TransferSyntaxRegistry.get(&ts_uid).ok_or_else(|| {
            Error::UnsupportedTransferSyntax {
                uid: ts_uid.clone(),
            }
        })?;

        let dicom_file = open_file(&file.path).map_err(|e| Error::ReadFile {
            path: file.path.display().to_string(),
            message: e.to_string(),
        })?;

        let dicom_file = into_ts(dicom_file, ts_selected, options.verbose)?;

        let message_id = self.next_message_id();
        let cmd = build_cstore_rq(&file.sop_class_uid, &file.sop_instance_uid, message_id)?;

        let mut object_data = Vec::new();
        dicom_file
            .write_dataset_with_ts(&mut object_data, ts_selected)
            .map_err(|e| Error::ReadFile {
                path: file.path.display().to_string(),
                message: e.to_string(),
            })?;

        self.send_cstore(pc.id, cmd, object_data).await?;
        let response = self.receive_response().await?;
        ensure_success(response)?;

        file.presentation_context = Some(pc);
        file.transfer_syntax_selected = Some(ts_uid);

        if options.verbose {
            tracing::info!(
                "Stored {} (sop={}, ts={})",
                file.path.display(),
                file.sop_instance_uid,
                file.transfer_syntax_selected.as_deref().unwrap_or("")
            );
        }

        Ok(())
    }

    async fn send_cstore(&mut self, pc_id: u8, cmd: Vec<u8>, object_data: Vec<u8>) -> Result<()> {
        let max_pdu = self.acceptor_max_pdu_length() as usize;
        let header_overhead = 12;
        let max_chunk = max_pdu.saturating_sub(header_overhead).max(1024);

        let total = cmd.len() + object_data.len();
        if total + header_overhead * 2 <= max_pdu {
            let pdu = Pdu::PData {
                data: vec![
                    PDataValue {
                        presentation_context_id: pc_id,
                        value_type: PDataValueType::Command,
                        is_last: true,
                        data: cmd,
                    },
                    PDataValue {
                        presentation_context_id: pc_id,
                        value_type: PDataValueType::Data,
                        is_last: true,
                        data: object_data,
                    },
                ],
            };
            return self.send_pdu(pdu).await;
        }

        self.send_command(pc_id, cmd).await?;
        self.send_dataset(pc_id, object_data, max_chunk).await
    }

    async fn send_dataset(&mut self, pc_id: u8, data: Vec<u8>, max_chunk: usize) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let chunks: Vec<_> = data.chunks(max_chunk).collect();
        let last_idx = chunks.len() - 1;

        for (i, chunk) in chunks.into_iter().enumerate() {
            self.send_pdu(Pdu::PData {
                data: vec![PDataValue {
                    presentation_context_id: pc_id,
                    value_type: PDataValueType::Data,
                    is_last: i == last_idx,
                    data: chunk.to_vec(),
                }],
            })
            .await?;
        }

        Ok(())
    }
}
