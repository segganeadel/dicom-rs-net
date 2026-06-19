//! C-STORE SCP service and storage sinks.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dicom_object::FileMetaTableBuilder;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::association::AssociationContext;
use crate::association::DatasetReader;
use crate::dimse::DimseMessage;
use crate::error::{Error, Result};
use crate::service::DicomService;
use crate::status::Status;
use crate::transfer::STORAGE_ABSTRACT_SYNTAXES;

/// Hook for persisting a received C-STORE dataset (dcm4che `BasicCStoreSCP.store()`).
#[async_trait]
pub trait CStoreSink: Send + Sync {
    /// Streams the dataset to storage.
    async fn store<S>(
        &self,
        command: &DimseMessage,
        transfer_syntax: &str,
        dataset: &mut DatasetReader<'_, S>,
        ctx: &AssociationContext,
    ) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send;
}

/// C-STORE SCP service delegating storage to a [`FileCStoreSink`].
pub struct CStoreService {
    sink: Arc<FileCStoreSink>,
    promiscuous: bool,
}

impl std::fmt::Debug for CStoreService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CStoreService")
            .field("promiscuous", &self.promiscuous)
            .finish()
    }
}

impl CStoreService {
    /// Creates a service accepting the default storage SOP class list.
    pub fn new(sink: FileCStoreSink) -> Self {
        Self {
            sink: Arc::new(sink),
            promiscuous: false,
        }
    }

    /// Creates a promiscuous service accepting any storage SOP class (`"*"`).
    pub fn promiscuous(sink: FileCStoreSink) -> Self {
        Self {
            sink: Arc::new(sink),
            promiscuous: true,
        }
    }

    /// Handles a streaming C-STORE request.
    pub async fn handle_stream<S>(
        &self,
        command: &DimseMessage,
        transfer_syntax: &str,
        dataset: &mut DatasetReader<'_, S>,
        ctx: &AssociationContext,
    ) -> Result<Status>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        match self
            .sink
            .store(command, transfer_syntax, dataset, ctx)
            .await
        {
            Ok(()) => Ok(Status::SUCCESS),
            Err(e) => {
                tracing::warn!("C-STORE storage failed: {e}");
                Ok(Status::PROCESSING_FAILURE)
            }
        }
    }
}

#[async_trait]
impl DicomService for CStoreService {
    fn sop_classes(&self) -> &[&str] {
        if self.promiscuous {
            &["*"]
        } else {
            STORAGE_ABSTRACT_SYNTAXES
        }
    }

    async fn handle(
        &self,
        _request: DimseMessage,
        _data: &[u8],
        _ctx: &AssociationContext,
    ) -> Result<Status> {
        Err(Error::NotImplemented {
            feature: "buffered C-STORE (use streaming path)",
        })
    }
}

/// Writes received instances to disk using temp-file-then-rename.
#[derive(Debug, Clone)]
pub struct FileCStoreSink {
    output_dir: PathBuf,
}

impl FileCStoreSink {
    /// Creates a sink writing to the given directory.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }

    /// Output directory for stored instances.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    async fn write_fmi(
        file: &mut File,
        sop_class_uid: &str,
        sop_instance_uid: &str,
        transfer_syntax: &str,
    ) -> Result<()> {
        let meta = FileMetaTableBuilder::new()
            .media_storage_sop_class_uid(sop_class_uid)
            .media_storage_sop_instance_uid(sop_instance_uid)
            .transfer_syntax(transfer_syntax)
            .build()
            .map_err(|e| Error::EncodeResponse {
                message: e.to_string(),
            })?;

        file.write_all(&[0_u8; 128]).await?;
        file.write_all(b"DICM").await?;

        let mut meta_bytes = Vec::new();
        meta.write(&mut meta_bytes).map_err(|e| Error::EncodeResponse {
            message: e.to_string(),
        })?;
        file.write_all(&meta_bytes).await?;

        Ok(())
    }

    async fn rename_with_retry(from: &Path, to: &Path) -> Result<()> {
        for attempt in 0..5 {
            match tokio::fs::rename(from, to).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < 4 => {
                    tracing::debug!("rename retry {attempt}: {e}");
                    tokio::time::sleep(Duration::from_millis(50 * (attempt as u64 + 1))).await;
                }
                Err(e) => return Err(Error::Io { source: e }),
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CStoreSink for FileCStoreSink {
    async fn store<S>(
        &self,
        command: &DimseMessage,
        transfer_syntax: &str,
        dataset: &mut DatasetReader<'_, S>,
        _ctx: &AssociationContext,
    ) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        let sop_class_uid = command.affected_sop_class_uid.as_deref().ok_or_else(|| {
            Error::InvalidCommand {
                message: "missing Affected SOP Class UID".to_string(),
            }
        })?;
        let sop_instance_uid = command
            .affected_sop_instance_uid
            .as_deref()
            .ok_or_else(|| Error::InvalidCommand {
                message: "missing Affected SOP Instance UID".to_string(),
            })?;

        tokio::fs::create_dir_all(&self.output_dir).await?;

        let temp_path = self
            .output_dir
            .join(format!("{sop_instance_uid}.part"));
        let final_path = self
            .output_dir
            .join(format!("{sop_instance_uid}.dcm"));

        let mut file = File::create(&temp_path).await?;
        if let Err(e) = async {
            Self::write_fmi(
                &mut file,
                sop_class_uid,
                sop_instance_uid,
                transfer_syntax,
            )
            .await?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = dataset.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n]).await?;
            }
            file.flush().await?;
            Self::rename_with_retry(&temp_path, &final_path).await
        }
        .await
        {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(e);
        }

        info!("Stored {}", final_path.display());
        Ok(())
    }
}
