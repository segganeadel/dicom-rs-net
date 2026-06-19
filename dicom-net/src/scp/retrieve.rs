//! C-MOVE and C-GET SCP services.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::association::CRetrieveSink;
use crate::association::AssociationContext;
use crate::dimse::DimseMessage;
use crate::error::Result;
use crate::qr::{STUDY_ROOT_GET, STUDY_ROOT_MOVE};
use crate::service::DicomService;
use crate::status::Status;

/// C-MOVE SCP handler.
pub struct CMoveService {
    pub(crate) sink: Arc<dyn CRetrieveSink>,
    /// Move destination AE title → remote address (`AE@host:port`).
    pub move_destinations: HashMap<String, String>,
}

impl std::fmt::Debug for CMoveService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CMoveService")
            .field("destinations", &self.move_destinations.len())
            .finish()
    }
}

impl CMoveService {
    /// Creates a C-MOVE service.
    pub fn new(sink: Arc<dyn CRetrieveSink>, move_destinations: HashMap<String, String>) -> Self {
        Self {
            sink,
            move_destinations,
        }
    }
}

#[async_trait]
impl DicomService for CMoveService {
    fn sop_classes(&self) -> &[&str] {
        &[STUDY_ROOT_MOVE]
    }

    async fn handle(
        &self,
        _request: DimseMessage,
        _data: &[u8],
        _ctx: &AssociationContext,
    ) -> Result<Status> {
        Ok(Status::SUCCESS)
    }
}

/// C-GET SCP handler.
pub struct CGetService {
    pub(crate) sink: Arc<dyn CRetrieveSink>,
}

impl std::fmt::Debug for CGetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CGetService").finish()
    }
}

impl CGetService {
    /// Creates a C-GET service.
    pub fn new(sink: Arc<dyn CRetrieveSink>) -> Self {
        Self { sink }
    }
}

#[async_trait]
impl DicomService for CGetService {
    fn sop_classes(&self) -> &[&str] {
        &[STUDY_ROOT_GET]
    }

    async fn handle(
        &self,
        _request: DimseMessage,
        _data: &[u8],
        _ctx: &AssociationContext,
    ) -> Result<Status> {
        Ok(Status::SUCCESS)
    }
}
