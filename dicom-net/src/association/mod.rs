//! Per-association context for DIMSE service handlers.

mod dataset_stream;
mod dimse_loop;
mod retrieve;

use dicom_ul::association::server::AsyncServerAssociation;
use dicom_ul::pdu::PresentationContextNegotiated;

pub use dataset_stream::{DatasetReader, DatasetStream};
pub use dimse_loop::handle_association;
pub use retrieve::{
    CRetrieveSink, FileRetrieveSink, InstanceLocator, RetrieveSource, run_cget_subops,
    run_cmove_subops,
};

/// Snapshot of association state exposed to SCP services.
#[derive(Debug, Clone)]
pub struct AssociationContext {
    calling_ae: String,
    called_ae: String,
    presentation_contexts: Vec<PresentationContextNegotiated>,
}

impl AssociationContext {
    /// Builds a context from an established server association.
    pub fn from_association<S>(association: &AsyncServerAssociation<S>) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
    {
        use dicom_ul::association::Association;

        Self {
            calling_ae: association.peer_ae_title().to_string(),
            called_ae: association.called_ae_title().to_string(),
            presentation_contexts: association.presentation_contexts().to_vec(),
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

    /// Finds a presentation context by its ID.
    pub fn presentation_context(&self, id: u8) -> Option<&PresentationContextNegotiated> {
        self.presentation_contexts.iter().find(|ctx| ctx.id == id)
    }
}
