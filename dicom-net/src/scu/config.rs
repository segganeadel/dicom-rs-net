//! SCU association negotiation configuration.

use std::collections::HashSet;

use dicom_dictionary_std::uids;
use dicom_ul::association::client::ClientAssociationOptions;

use crate::error::{Error, Result};
use crate::scu::file::DicomFile;

/// Configuration for outbound SCU associations.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Calling application entity title.
    pub calling_ae: String,
    /// Called application entity title.
    pub called_ae: Option<String>,
    /// Remote address (`AE@host:port` or `host:port`).
    pub remote_addr: Option<String>,
    /// Maximum PDU length.
    pub max_pdu_length: u32,
    /// Enforce strict PDU length.
    pub strict: bool,
    /// Read timeout for association I/O.
    pub read_timeout: Option<std::time::Duration>,
    /// Write timeout for association I/O.
    pub write_timeout: Option<std::time::Duration>,
    /// TCP connect timeout.
    pub connection_timeout: Option<std::time::Duration>,
    /// Refuse transcoding when transfer syntaxes differ.
    pub never_transcode: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            calling_ae: "STORESCU".to_string(),
            called_ae: None,
            remote_addr: None,
            max_pdu_length: 16_378,
            strict: true,
            read_timeout: None,
            write_timeout: None,
            connection_timeout: None,
            never_transcode: false,
        }
    }
}

impl ClientConfig {
    /// Returns the configured remote address or an error.
    pub fn remote_addr(&self) -> Result<&str> {
        self.remote_addr
            .as_deref()
            .ok_or(Error::RemoteNotConfigured)
    }

    /// Builds client options for a C-ECHO association.
    pub fn build_echo_options(&self) -> ClientAssociationOptions<'_> {
        let mut options = self.base_options();
        options = options.with_abstract_syntax(uids::VERIFICATION);
        options
    }

    /// Builds client options for C-STORE from scanned files.
    pub fn build_store_options(&self, files: &[DicomFile]) -> ClientAssociationOptions<'_> {
        let mut options = self.base_options();
        let pairs = presentation_context_pairs(files, self.never_transcode);
        for (abstract_syntax, transfer_syntaxes) in pairs {
            options = options.with_presentation_context(abstract_syntax, transfer_syntaxes);
        }
        options
    }

    fn base_options(&self) -> ClientAssociationOptions<'_> {
        let mut options = ClientAssociationOptions::new()
            .calling_ae_title(self.calling_ae.clone())
            .max_pdu_length(self.max_pdu_length)
            .strict(self.strict);
        if let Some(called) = &self.called_ae {
            options = options.called_ae_title(called.clone());
        }
        if let Some(timeout) = self.read_timeout {
            options = options.read_timeout(timeout);
        }
        if let Some(timeout) = self.write_timeout {
            options = options.write_timeout(timeout);
        }
        if let Some(timeout) = self.connection_timeout {
            options = options.connection_timeout(timeout);
        }
        options
    }
}

/// Unique `(abstract_syntax, transfer_syntax)` pairs for association negotiation.
pub fn presentation_context_pairs(
    files: &[DicomFile],
    never_transcode: bool,
) -> Vec<(String, Vec<String>)> {
    let mut set: HashSet<(String, String)> = HashSet::new();

    for file in files {
        set.insert((
            file.sop_class_uid.clone(),
            file.file_transfer_syntax.clone(),
        ));
        if !never_transcode {
            set.insert((
                file.sop_class_uid.clone(),
                uids::EXPLICIT_VR_LITTLE_ENDIAN.to_string(),
            ));
            set.insert((
                file.sop_class_uid.clone(),
                uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string(),
            ));
        }
    }

    let mut by_abstract: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (abstract_syntax, ts) in set {
        by_abstract.entry(abstract_syntax).or_default().push(ts);
    }

    by_abstract.into_iter().collect()
}
