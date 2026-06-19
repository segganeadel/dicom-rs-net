//! Association negotiation configuration for SCP devices.

use dicom_transfer_syntax_registry::TransferSyntaxRegistry;
use dicom_ul::association::server::{
    AcceptAny, DefaultNegotiation, ServerAssociationOptions,
};

use crate::transfer::ABSTRACT_SYNTAXES;

/// Configuration for DICOM association negotiation on the SCP side.
#[derive(Debug, Clone)]
pub struct AssociationConfig {
    /// Application entity title announced by this SCP.
    pub ae_title: String,
    /// Enforce negotiated max PDU length.
    pub strict: bool,
    /// Accept unknown SOP classes (promiscuous mode).
    pub promiscuous: bool,
    /// Maximum PDU length.
    pub max_pdu_length: u32,
    /// Only offer native/uncompressed transfer syntaxes.
    pub uncompressed_only: bool,
    /// Abstract syntax UIDs to accept.
    pub abstract_syntaxes: Vec<String>,
    /// Transfer syntax UIDs to offer.
    pub transfer_syntaxes: Vec<String>,
}

impl Default for AssociationConfig {
    fn default() -> Self {
        Self {
            ae_title: "STORESCP".to_string(),
            strict: false,
            promiscuous: false,
            max_pdu_length: 16_378,
            uncompressed_only: false,
            abstract_syntaxes: ABSTRACT_SYNTAXES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            transfer_syntaxes: default_transfer_syntaxes(false),
        }
    }
}

fn default_transfer_syntaxes(uncompressed_only: bool) -> Vec<String> {
    if uncompressed_only {
        vec![
            "1.2.840.10008.1.2".to_string(),
            "1.2.840.10008.1.2.1".to_string(),
        ]
    } else {
        TransferSyntaxRegistry
            .iter()
            .filter(|ts| !ts.is_unsupported())
            .map(|ts| ts.uid().to_string())
            .collect()
    }
}

impl AssociationConfig {
    /// Builds [`ServerAssociationOptions`] for `dicom-ul`.
    pub fn build_server_options(
        &self,
    ) -> ServerAssociationOptions<'static, AcceptAny, DefaultNegotiation> {
        let mut options = ServerAssociationOptions::new()
            .accept_any()
            .ae_title(self.ae_title.clone())
            .strict(self.strict)
            .max_pdu_length(self.max_pdu_length)
            .promiscuous(self.promiscuous);

        for ts in self.transfer_syntaxes.clone() {
            options = options.with_transfer_syntax(ts);
        }

        for uid in self.abstract_syntaxes.clone() {
            options = options.with_abstract_syntax(uid);
        }

        options
    }

    /// Rebuilds transfer syntax list when `uncompressed_only` changes.
    pub fn set_uncompressed_only(&mut self, value: bool) {
        self.uncompressed_only = value;
        self.transfer_syntaxes = default_transfer_syntaxes(value);
    }
}
