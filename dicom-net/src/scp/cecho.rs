//! C-ECHO SCP service.

use async_trait::async_trait;
use dicom_dictionary_std::uids::VERIFICATION;

use crate::association::AssociationContext;
use crate::dimse::DimseMessage;
use crate::error::Result;
use crate::service::DicomService;
use crate::status::Status;

/// Verification SCP (C-ECHO) handler.
#[derive(Debug, Default)]
pub struct CEchoService;

impl CEchoService {
    /// Creates a new C-ECHO service.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DicomService for CEchoService {
    fn sop_classes(&self) -> &[&str] {
        &[VERIFICATION]
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
