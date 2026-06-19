//! Application entity configuration for SCP and SCU roles.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::association::server::{AcceptAny, DefaultNegotiation, ServerAssociationOptions};

use crate::device::connection::Connection;
use crate::device::transfer_capability::{
    Role, TransferCapability, default_storage_scp_capabilities, default_transfer_syntaxes,
};
use crate::dimse::response::SubOperationCounts;
use crate::error::{Error, Result};
use crate::qr::{STUDY_ROOT_FIND, STUDY_ROOT_GET, STUDY_ROOT_MOVE};
use crate::scp::{CFindService, CGetService, CMoveService, CStoreService};
use crate::scu::{DicomFile, ScuAssociation, StoreOptions, build_study_find_identifier, presentation_context_pairs, scan_files};
use crate::service::{DicomService, ServiceRegistry};

/// A DICOM application entity with transfer capabilities and DIMSE services.
#[derive(Debug, Clone)]
pub struct ApplicationEntity {
    /// Application entity title.
    pub ae_title: String,
    /// Accepts inbound associations on linked connections.
    pub acceptor: bool,
    /// Initiates outbound associations.
    pub initiator: bool,
    /// Accept unknown SOP classes during SCP negotiation.
    pub promiscuous: bool,
    /// Only offer uncompressed transfer syntaxes as SCP.
    pub uncompressed_only: bool,
    /// SCP transfer capabilities.
    pub scp_capabilities: Vec<TransferCapability>,
    /// SCU transfer capabilities.
    pub scu_capabilities: Vec<TransferCapability>,
    /// Indices into the parent [`super::Device`] connection list.
    pub connection_indices: Vec<usize>,
    /// Optional allow-list of calling AE titles (SCP access control).
    pub accepted_calling_aets: Option<Vec<String>>,
    /// DIMSE services handled by this AE when acting as SCP.
    pub services: ServiceRegistry,
}

impl ApplicationEntity {
    /// Creates a new application entity with the given AE title.
    pub fn new(ae_title: impl Into<String>) -> Self {
        Self {
            ae_title: ae_title.into(),
            acceptor: false,
            initiator: false,
            promiscuous: false,
            uncompressed_only: false,
            scp_capabilities: Vec::new(),
            scu_capabilities: Vec::new(),
            connection_indices: Vec::new(),
            accepted_calling_aets: None,
            services: ServiceRegistry::new(),
        }
    }

    /// Marks this AE as an association acceptor (SCP).
    pub fn acceptor(mut self, acceptor: bool) -> Self {
        self.acceptor = acceptor;
        self
    }

    /// Marks this AE as an association initiator (SCU).
    pub fn initiator(mut self, initiator: bool) -> Self {
        self.initiator = initiator;
        self
    }

    /// Enables promiscuous SCP negotiation.
    pub fn promiscuous(mut self, promiscuous: bool) -> Self {
        self.promiscuous = promiscuous;
        self
    }

    /// Restricts offered SCP transfer syntaxes to uncompressed only.
    pub fn uncompressed_only(mut self, value: bool) -> Self {
        self.uncompressed_only = value;
        self
    }

    /// Links this AE to a connection by index in the parent device.
    pub fn add_connection(mut self, index: usize) -> Self {
        self.connection_indices.push(index);
        self
    }

    /// Restricts accepted calling AE titles for inbound associations.
    pub fn accepted_calling_aets(mut self, aets: Vec<String>) -> Self {
        self.accepted_calling_aets = Some(aets);
        self
    }

    /// Adds an SCP transfer capability.
    pub fn add_scp_capability(&mut self, capability: TransferCapability) {
        self.scp_capabilities.push(capability);
    }

    /// Adds an SCU transfer capability.
    pub fn add_scu_capability(&mut self, capability: TransferCapability) {
        self.scu_capabilities.push(capability);
    }

    /// Adds default storage and verification SCP capabilities.
    pub fn add_default_storage_capabilities(&mut self) {
        self.scp_capabilities
            .extend(default_storage_scp_capabilities(self.uncompressed_only));
        if !self
            .scp_capabilities
            .iter()
            .any(|c| c.sop_class == dicom_dictionary_std::uids::VERIFICATION)
        {
            self.add_scp_capability(TransferCapability::verification_scp());
        }
    }

    /// Adds SCU storage capabilities inferred from DICOM files on disk.
    pub fn add_scu_storage_capabilities_from_files(
        &mut self,
        paths: &[PathBuf],
        never_transcode: bool,
    ) -> Result<()> {
        let files = scan_files(paths)?;
        self.add_scu_storage_capabilities_from_dicom_files(&files, never_transcode);
        Ok(())
    }

    /// Adds SCU storage capabilities from scanned file metadata.
    pub fn add_scu_storage_capabilities_from_dicom_files(
        &mut self,
        files: &[DicomFile],
        never_transcode: bool,
    ) {
        for (abstract_syntax, transfer_syntaxes) in
            presentation_context_pairs(files, never_transcode)
        {
            self.add_scu_capability(TransferCapability::storage_scu(
                abstract_syntax,
                transfer_syntaxes,
            ));
        }
    }

    /// Registers a DIMSE SCP service.
    pub fn register_service(&mut self, service: Arc<dyn DicomService>) {
        self.services.register(service);
    }

    /// Registers a C-STORE SCP service.
    pub fn register_cstore(&mut self, service: Arc<CStoreService>) {
        self.services.register_cstore(service);
    }

    /// Registers a C-FIND SCP service.
    pub fn register_cfind(&mut self, service: Arc<CFindService>) {
        self.services.register_cfind(service);
    }

    /// Registers a C-MOVE SCP service.
    pub fn register_cmove(&mut self, service: Arc<CMoveService>) {
        self.services.register_cmove(service);
    }

    /// Registers a C-GET SCP service.
    pub fn register_cget(&mut self, service: Arc<CGetService>) {
        self.services.register_cget(service);
    }

    /// Builds [`ServerAssociationOptions`] for SCP negotiation on a connection.
    pub fn build_server_options(
        &self,
        conn: &Connection,
    ) -> ServerAssociationOptions<'static, AcceptAny, DefaultNegotiation> {
        let mut options = ServerAssociationOptions::new()
            .accept_any()
            .ae_title(self.ae_title.clone())
            .strict(conn.strict)
            .max_pdu_length(conn.max_pdu_length)
            .promiscuous(self.promiscuous);

        if let Some(timeout) = conn.read_timeout {
            options = options.read_timeout(timeout);
        }
        if let Some(timeout) = conn.write_timeout {
            options = options.write_timeout(timeout);
        }

        #[cfg(feature = "tls")]
        if let Some(tls) = &conn.tls_server_config {
            options = options.tls_config(std::sync::Arc::clone(tls));
        }

        let transfer_syntaxes = self.scp_transfer_syntaxes();
        for ts in transfer_syntaxes {
            options = options.with_transfer_syntax(ts);
        }

        for uid in self.scp_abstract_syntaxes() {
            options = options.with_abstract_syntax(uid);
        }

        options
    }

    /// Builds base [`ClientAssociationOptions`] from SCU capabilities.
    pub fn build_client_options(&self, conn: &Connection) -> ClientAssociationOptions<'_> {
        let mut options = ClientAssociationOptions::new()
            .calling_ae_title(self.ae_title.clone())
            .max_pdu_length(conn.max_pdu_length)
            .strict(conn.strict);

        if let Some(timeout) = conn.read_timeout {
            options = options.read_timeout(timeout);
        }
        if let Some(timeout) = conn.write_timeout {
            options = options.write_timeout(timeout);
        }
        if let Some(timeout) = conn.connection_timeout {
            options = options.connection_timeout(timeout);
        }

        #[cfg(feature = "tls")]
        if let Some(tls) = &conn.tls_client_config {
            options = options.tls_config(std::sync::Arc::clone(tls));
            if let Some(name) = &conn.tls_server_name {
                options = options.server_name(name.as_str());
            }
        }

        for cap in &self.scu_capabilities {
            if cap.role == Role::Scu {
                options = options.with_presentation_context(
                    cap.sop_class.clone(),
                    cap.transfer_syntaxes.clone(),
                );
            }
        }

        options
    }

    /// Builds client options for C-ECHO.
    pub fn build_echo_options(&self, conn: &Connection) -> ClientAssociationOptions<'_> {
        let mut options = self.build_client_options(conn);
        if !self
            .scu_capabilities
            .iter()
            .any(|c| c.sop_class == dicom_dictionary_std::uids::VERIFICATION)
        {
            options = options.with_abstract_syntax(dicom_dictionary_std::uids::VERIFICATION);
        }
        options
    }

    /// Builds client options for C-STORE from scanned files.
    pub fn build_store_options(
        &self,
        conn: &Connection,
        files: &[DicomFile],
        never_transcode: bool,
    ) -> ClientAssociationOptions<'_> {
        let mut options = self.build_client_options(conn);
        for (abstract_syntax, transfer_syntaxes) in
            presentation_context_pairs(files, never_transcode)
        {
            options = options.with_presentation_context(abstract_syntax, transfer_syntaxes);
        }
        options
    }

    /// Builds client options for Study Root C-FIND.
    pub fn build_find_options(&self, conn: &Connection) -> ClientAssociationOptions<'_> {
        let mut options = self.build_client_options(conn);
        if !self
            .scu_capabilities
            .iter()
            .any(|c| c.sop_class == STUDY_ROOT_FIND)
        {
            options = options.with_presentation_context(
                STUDY_ROOT_FIND,
                vec![
                    dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN,
                    dicom_dictionary_std::uids::EXPLICIT_VR_LITTLE_ENDIAN,
                ],
            );
        }
        options
    }

    /// Builds client options for Study Root C-MOVE.
    pub fn build_move_options(&self, conn: &Connection) -> ClientAssociationOptions<'_> {
        let mut options = self.build_client_options(conn);
        if !self
            .scu_capabilities
            .iter()
            .any(|c| c.sop_class == STUDY_ROOT_MOVE)
        {
            options = options.with_presentation_context(
                STUDY_ROOT_MOVE,
                vec![
                    dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN,
                    dicom_dictionary_std::uids::EXPLICIT_VR_LITTLE_ENDIAN,
                ],
            );
        }
        options
    }

    /// Builds client options for Study Root C-GET.
    pub fn build_get_options(&self, conn: &Connection) -> ClientAssociationOptions<'_> {
        let mut options = self.build_client_options(conn);
        if !self
            .scu_capabilities
            .iter()
            .any(|c| c.sop_class == STUDY_ROOT_GET)
        {
            options = options.with_presentation_context(
                STUDY_ROOT_GET,
                vec![
                    dicom_dictionary_std::uids::IMPLICIT_VR_LITTLE_ENDIAN,
                    dicom_dictionary_std::uids::EXPLICIT_VR_LITTLE_ENDIAN,
                ],
            );
        }
        options
    }

    async fn establish_scu(
        &self,
        conn: &Connection,
        options: ClientAssociationOptions<'_>,
        remote: &str,
    ) -> Result<ScuAssociation> {
        if !self.initiator {
            return Err(Error::InvalidCommand {
                message: format!("AE {} is not configured as an initiator", self.ae_title),
            });
        }

        #[cfg(feature = "tls")]
        if conn.tls_client_config.is_some() {
            let inner = options
                .establish_with_async_tls(remote)
                .await
                .map_err(|source| Error::Ul { source })?;
            return Ok(ScuAssociation::new_tls(inner));
        }

        let inner = options
            .establish_with_async(remote)
            .await
            .map_err(|source| Error::Ul { source })?;
        Ok(ScuAssociation::new(inner))
    }

    /// Establishes an SCU association for verification.
    pub async fn connect(&self, conn: &Connection, remote: &str) -> Result<ScuAssociation> {
        let options = self.build_echo_options(conn);
        self.establish_scu(conn, options, remote).await
    }

    /// One-shot C-ECHO against a remote SCP.
    pub async fn echo(&self, conn: &Connection, remote: &str) -> Result<()> {
        let mut assoc = self.connect(conn, remote).await?;
        assoc.echo().await?;
        assoc.release().await
    }

    /// One-shot Study Root C-FIND against a remote SCP.
    pub async fn find(
        &self,
        conn: &Connection,
        remote: &str,
        patient_id: Option<&str>,
    ) -> Result<Vec<Vec<u8>>> {
        let identifier = build_study_find_identifier(patient_id)?;
        let options = self.build_find_options(conn);
        let mut assoc = self.establish_scu(conn, options, remote).await?;
        let matches = assoc.find(&identifier).await?;
        assoc.release().await?;
        Ok(matches)
    }

    /// One-shot Study Root C-MOVE against a remote SCP.
    pub async fn move_instances(
        &self,
        conn: &Connection,
        remote: &str,
        identifier: &[u8],
        move_destination: &str,
    ) -> Result<SubOperationCounts> {
        let options = self.build_move_options(conn);
        let mut assoc = self.establish_scu(conn, options, remote).await?;
        let counts = assoc
            .move_instances(identifier, move_destination)
            .await?;
        assoc.release().await?;
        Ok(counts)
    }

    /// One-shot Study Root C-GET against a remote SCP.
    pub async fn get_instances(
        &self,
        conn: &Connection,
        remote: &str,
        identifier: &[u8],
    ) -> Result<SubOperationCounts> {
        let options = self.build_get_options(conn);
        let mut assoc = self.establish_scu(conn, options, remote).await?;
        let counts = assoc.get_instances(identifier).await?;
        assoc.release().await?;
        Ok(counts)
    }

    /// One-shot C-STORE of files against a remote SCP.
    pub async fn store_files(
        &self,
        conn: &Connection,
        remote: &str,
        paths: &[PathBuf],
        options: &StoreOptions,
    ) -> Result<usize> {
        let store_options = options.clone();
        let mut files = scan_files(paths)?;
        let client_options =
            self.build_store_options(conn, &files, store_options.never_transcode);
        let mut assoc = self
            .establish_scu(conn, client_options, remote)
            .await?;
        let sent = assoc.store_files(&mut files, &store_options).await?;
        assoc.release().await?;
        Ok(sent)
    }

    /// Returns whether the given calling AE title is allowed.
    pub fn accepts_calling_ae(&self, calling_ae: &str) -> bool {
        match &self.accepted_calling_aets {
            Some(allowed) => allowed.iter().any(|a| normalize_ae_title(a) == calling_ae),
            None => true,
        }
    }

    fn scp_abstract_syntaxes(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for cap in &self.scp_capabilities {
            if cap.role == Role::Scp {
                set.insert(cap.sop_class.clone());
            }
        }
        set.into_iter().collect()
    }

    fn scp_transfer_syntaxes(&self) -> Vec<String> {
        let mut set = HashSet::new();
        for cap in &self.scp_capabilities {
            if cap.role == Role::Scp {
                for ts in &cap.transfer_syntaxes {
                    set.insert(ts.clone());
                }
            }
        }
        if set.is_empty() && !self.promiscuous {
            set.extend(default_transfer_syntaxes(self.uncompressed_only));
        }
        set.into_iter().collect()
    }
}

/// Normalizes a DICOM AE title for comparison.
pub fn normalize_ae_title(title: &str) -> String {
    title.trim_end_matches([' ', '\0']).to_string()
}
