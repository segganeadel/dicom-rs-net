//! Per-association context for DIMSE service handlers.

mod dataset_stream;
mod dimse_loop;
mod retrieve;

use std::sync::Arc;

use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::pdu::PresentationContextNegotiated;

pub use dataset_stream::{DatasetReader, DatasetStream};
pub use dimse_loop::{handle_association, AssociationTracker};
pub use retrieve::{
    CRetrieveSink, FileRetrieveSink, InstanceLocator, RetrieveSource, run_cget_subops,
    run_cmove_subops,
};

use crate::device::SharedAssociationRegistry;

/// Snapshot of association state exposed to SCP services.
#[derive(Debug, Clone)]
pub struct AssociationContext {
    calling_ae: String,
    called_ae: String,
    presentation_contexts: Vec<PresentationContextNegotiated>,
    association_id: Option<u64>,
    association_registry: Option<SharedAssociationRegistry>,
    ae_id: String,
    connection_id: String,
    connection_index: usize,
}

impl AssociationContext {
    /// Builds a context from an established server association.
    pub fn from_association<S>(
        association: &AsyncServerAssociation<S>,
        tracker: &AssociationTracker,
    ) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        use dicom_ul::association::Association;

        Self {
            calling_ae: association.peer_ae_title().to_string(),
            called_ae: association.called_ae_title().to_string(),
            presentation_contexts: association.presentation_contexts().to_vec(),
            association_id: Some(tracker.id),
            association_registry: Some(Arc::clone(&tracker.registry)),
            ae_id: tracker.ae_id.clone(),
            connection_id: tracker.connection_id.clone(),
            connection_index: tracker.connection_index,
        }
    }

    /// Application entity title of the calling peer (SCU).
    pub fn calling_ae(&self) -> &str {
        &self.calling_ae
    }

    /// Application entity title this SCP was called as.
    pub fn called_ae(&self) -> &str {
        &self.called_ae
    }

    /// Negotiated presentation contexts for this association.
    pub fn presentation_contexts(&self) -> &[PresentationContextNegotiated] {
        &self.presentation_contexts
    }

    /// Config AE id for the local SCP.
    pub fn ae_id(&self) -> &str {
        &self.ae_id
    }

    /// Connection id for the listener that accepted this association.
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }

    /// Index of the listener connection on the device.
    pub fn connection_index(&self) -> usize {
        self.connection_index
    }

    /// Registry id for this association, if tracked.
    pub fn association_id(&self) -> Option<u64> {
        self.association_id
    }

    /// Shared association registry for sub-operation tracking.
    pub fn association_registry(&self) -> Option<&SharedAssociationRegistry> {
        self.association_registry.as_ref()
    }

    /// Finds a presentation context by its ID.
    pub fn presentation_context(&self, id: u8) -> Option<&PresentationContextNegotiated> {
        self.presentation_contexts.iter().find(|ctx| ctx.id == id)
    }
}
