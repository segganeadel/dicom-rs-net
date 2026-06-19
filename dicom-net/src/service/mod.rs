//! DIMSE SCP service traits and routing.

mod registry;

use async_trait::async_trait;

use crate::association::AssociationContext;
use crate::dimse::DimseMessage;
use crate::error::Result;
use crate::status::Status;

pub use registry::ServiceRegistry;

/// Handler for one or more SOP classes on the SCP side.
#[async_trait]
pub trait DicomService: Send + Sync {
    /// SOP class UIDs handled by this service, or `"*"` for promiscuous mode.
    fn sop_classes(&self) -> &[&str];

    /// Handles an incoming DIMSE request.
    async fn handle(
        &self,
        request: DimseMessage,
        data: &[u8],
        ctx: &AssociationContext,
    ) -> Result<Status>;
}
