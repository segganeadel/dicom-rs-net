//! DIMSE client (SCU) API.

mod association;
mod cmove;
mod config;
mod file;
mod find;
mod get;
mod presentation_context;
mod store;
mod transcode;

use std::path::PathBuf;

pub use association::ScuAssociation;
pub use config::{ClientConfig, presentation_context_pairs};
pub use file::{DicomFile, scan_files};
pub use find::build_study_find_identifier;
pub use store::StoreOptions;

use crate::error::{Error, Result};

/// DIMSE client for calling remote SCPs.
///
/// Convenience wrapper around [`ClientConfig`] and [`ScuAssociation`].
/// For device-model SCU operations, see [`crate::device::ApplicationEntity`].
#[derive(Debug, Default)]
pub struct Client {
    config: ClientConfig,
}

impl Client {
    /// Creates a new unconfigured client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the calling AE title.
    pub fn calling_ae(mut self, ae_title: impl Into<String>) -> Self {
        self.config.calling_ae = ae_title.into();
        self
    }

    /// Sets the called AE title.
    pub fn called_ae(mut self, ae_title: impl Into<String>) -> Self {
        self.config.called_ae = Some(ae_title.into());
        self
    }

    /// Sets the remote SCP address (`AE@host:port` or `host:port`).
    pub fn remote(mut self, addr: impl Into<String>) -> Self {
        self.config.remote_addr = Some(addr.into());
        self
    }

    /// Sets the maximum PDU length.
    pub fn max_pdu_length(mut self, max_pdu_length: u32) -> Self {
        self.config.max_pdu_length = max_pdu_length;
        self
    }

    /// Sets strict PDU length enforcement.
    pub fn strict(mut self, strict: bool) -> Self {
        self.config.strict = strict;
        self
    }

    /// Refuses transcoding when file and negotiated transfer syntaxes differ.
    pub fn never_transcode(mut self, never_transcode: bool) -> Self {
        self.config.never_transcode = never_transcode;
        self
    }

    /// Establishes an association for Verification (C-ECHO) only.
    pub async fn connect(&self) -> Result<ScuAssociation> {
        let addr = self.remote_address()?;
        let options = self.config.build_echo_options();
        let inner = options
            .establish_with_async(&addr)
            .await
            .map_err(|source| Error::Ul { source })?;
        Ok(ScuAssociation::new(inner, None))
    }

    /// One-shot C-ECHO: connect, echo, release.
    pub async fn echo(self) -> Result<()> {
        let mut assoc = self.connect().await?;
        assoc.echo().await?;
        assoc.release().await
    }

    /// Scans files, establishes a C-STORE association, and returns the association with file metadata.
    pub async fn connect_for_store(
        self,
        paths: &[PathBuf],
    ) -> Result<(ScuAssociation, Vec<DicomFile>)> {
        let files = scan_files(paths)?;
        let addr = self.remote_address()?;
        let options = self.config.build_store_options(&files);
        let inner = options
            .establish_with_async(&addr)
            .await
            .map_err(|source| Error::Ul { source })?;
        Ok((ScuAssociation::new(inner, None), files))
    }

    /// One-shot C-STORE: connect, send all files, release.
    pub async fn store_files(self, paths: &[PathBuf], options: &StoreOptions) -> Result<usize> {
        let mut store_options = options.clone();
        store_options.never_transcode = self.config.never_transcode || options.never_transcode;

        let (mut assoc, mut files) = self.connect_for_store(paths).await?;
        let sent = assoc.store_files(&mut files, &store_options).await?;
        assoc.release().await?;
        Ok(sent)
    }

    fn remote_address(&self) -> Result<String> {
        let remote = self.config.remote_addr()?;
        if let Some(called) = &self.config.called_ae {
            if remote.contains('@') {
                Ok(remote.to_string())
            } else {
                Ok(format!("{called}@{remote}"))
            }
        } else {
            Ok(remote.to_string())
        }
    }
}
