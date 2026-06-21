//! C-FIND SCP service.

use std::sync::Arc;

use async_trait::async_trait;
use dicom_encoding::TransferSyntaxIndex;
use dicom_object::InMemDicomObject;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;

use crate::association::AssociationContext;
use crate::dimse::DimseMessage;
use crate::error::{Error, Result};
use crate::qr::{QueryRetrieveLevel, STUDY_ROOT_FIND};
use crate::service::DicomService;
use crate::status::Status;

/// Hook for resolving C-FIND identifier keys to match datasets.
#[async_trait]
pub trait CFindSink: Send + Sync {
    /// Returns encoded match identifier datasets for the query.
    async fn find(
        &self,
        identifier: &[u8],
        transfer_syntax: &str,
        level: QueryRetrieveLevel,
    ) -> Result<Vec<Vec<u8>>>;
}

/// C-FIND SCP handler.
pub struct CFindService {
    sink: Arc<dyn CFindSink>,
}

impl std::fmt::Debug for CFindService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CFindService").finish()
    }
}

impl CFindService {
    /// Creates a C-FIND service with the given sink.
    pub fn new(sink: Arc<dyn CFindSink>) -> Self {
        Self { sink }
    }

    /// Executes a C-FIND query and returns match dataset bytes.
    pub async fn find_matches(
        &self,
        identifier: &[u8],
        transfer_syntax: &str,
    ) -> Result<Vec<Vec<u8>>> {
        let level = parse_level(identifier, transfer_syntax)?;
        self.sink.find(identifier, transfer_syntax, level).await
    }
}

#[async_trait]
impl DicomService for CFindService {
    fn sop_classes(&self) -> &[&str] {
        &[STUDY_ROOT_FIND]
    }

    async fn handle(
        &self,
        _request: DimseMessage,
        data: &[u8],
        _ctx: &AssociationContext,
    ) -> Result<Status> {
        if data.is_empty() {
            return Ok(Status::IDENTIFIER_DOES_NOT_MATCH);
        }
        Ok(Status::SUCCESS)
    }
}

fn parse_level(identifier: &[u8], transfer_syntax: &str) -> Result<QueryRetrieveLevel> {
    let ts = TransferSyntaxRegistry
        .get(transfer_syntax)
        .ok_or_else(|| Error::InvalidCommand {
            message: format!("unknown transfer syntax {transfer_syntax}"),
        })?;
    let obj = InMemDicomObject::read_dataset_with_ts(identifier, ts).map_err(|e| {
        Error::InvalidCommand {
            message: e.to_string(),
        }
    })?;
    let level_str = obj
        .element(dicom_dictionary_std::tags::QUERY_RETRIEVE_LEVEL)
        .map_err(|_| Error::InvalidCommand {
            message: "missing QueryRetrieveLevel".to_string(),
        })?
        .to_str()
        .map_err(|_| Error::InvalidCommand {
            message: "QueryRetrieveLevel is not a string".to_string(),
        })?;
    level_str.parse().map_err(|_| Error::InvalidCommand {
        message: format!("unsupported QueryRetrieveLevel: {level_str}"),
    })
}

/// In-memory C-FIND sink returning fixed matches (for tests).
pub struct StaticCFindSink {
    matches: Vec<Vec<u8>>,
}

impl StaticCFindSink {
    /// Creates a sink that always returns the given matches.
    pub fn new(matches: Vec<Vec<u8>>) -> Self {
        Self { matches }
    }
}

#[async_trait]
impl CFindSink for StaticCFindSink {
    async fn find(
        &self,
        _identifier: &[u8],
        _transfer_syntax: &str,
        _level: QueryRetrieveLevel,
    ) -> Result<Vec<Vec<u8>>> {
        Ok(self.matches.clone())
    }
}
