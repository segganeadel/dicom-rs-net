//! Listener / device abstraction for SCP deployments.

mod config;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{error, info};

use crate::association::handle_association;
use crate::device::config::AssociationConfig;
use crate::error::{Error, Result};
use crate::service::ServiceRegistry;

pub use config::AssociationConfig as DeviceAssociationConfig;

/// Builder for a DICOM network device (listener).
#[derive(Debug)]
pub struct DeviceBuilder {
    assoc_config: AssociationConfig,
    bind_addr: Option<SocketAddr>,
    services: ServiceRegistry,
}

impl DeviceBuilder {
    /// Creates a new device builder with default settings.
    pub fn new() -> Self {
        Self {
            assoc_config: AssociationConfig::default(),
            bind_addr: None,
            services: ServiceRegistry::new(),
        }
    }

    /// Sets the AE title announced by this device.
    pub fn ae_title(mut self, ae_title: impl Into<String>) -> Self {
        self.assoc_config.ae_title = ae_title.into();
        self
    }

    /// Sets the local socket address to bind.
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    /// Sets association negotiation options.
    pub fn association_config(mut self, config: AssociationConfig) -> Self {
        self.assoc_config = config;
        self
    }

    /// Enables promiscuous mode (accept unknown SOP classes).
    pub fn promiscuous(mut self, promiscuous: bool) -> Self {
        self.assoc_config.promiscuous = promiscuous;
        self
    }

    /// Enables strict max PDU length enforcement.
    pub fn strict(mut self, strict: bool) -> Self {
        self.assoc_config.strict = strict;
        self
    }

    /// Restricts offered transfer syntaxes to uncompressed only.
    pub fn uncompressed_only(mut self, value: bool) -> Self {
        self.assoc_config.set_uncompressed_only(value);
        self
    }

    /// Sets the maximum PDU length.
    pub fn max_pdu_length(mut self, max_pdu_length: u32) -> Self {
        self.assoc_config.max_pdu_length = max_pdu_length;
        self
    }

    /// Replaces the service registry used for incoming associations.
    pub fn services(mut self, services: ServiceRegistry) -> Self {
        self.services = services;
        self
    }

    /// Registers a C-STORE service (indexes for streaming dispatch).
    pub fn register_cstore(mut self, service: Arc<crate::scp::CStoreService>) -> Self {
        self.services.register_cstore(service);
        self
    }

    /// Registers a single DIMSE service.
    pub fn register_service(mut self, service: Arc<dyn crate::service::DicomService>) -> Self {
        self.services.register(service);
        self
    }

    /// Starts accepting associations and serving DIMSE requests.
    pub async fn run(self) -> Result<()> {
        let bind_addr = self
            .bind_addr
            .ok_or_else(|| Error::InvalidCommand {
                message: "bind address not set".to_string(),
            })?;

        let options = self.assoc_config.build_server_options();
        let services = Arc::new(self.services);

        let listener = TcpListener::bind(bind_addr)
            .await
            .map_err(|source| Error::Io { source })?;

        info!(
            "{} listening on tcp://{}",
            self.assoc_config.ae_title, bind_addr
        );

        loop {
            let (socket, addr) = listener
                .accept()
                .await
                .map_err(|source| Error::Io { source })?;

            let options = options.clone();
            let services = Arc::clone(&services);

            tokio::spawn(async move {
                match options.establish_async(socket).await {
                    Ok(association) => {
                        if let Err(e) = handle_association(association, services).await {
                            error!("Association with {addr} failed: {e}");
                        }
                    }
                    Err(e) => {
                        error!("Association negotiation with {addr} failed: {e}");
                    }
                }
            });
        }
    }
}

impl Default for DeviceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
