//! Listener / device abstraction for SCP deployments.

mod application_entity;
mod config;
mod connection;
mod host;
mod transfer_capability;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::device::config::AssociationConfig;
use crate::error::Result;
use crate::service::ServiceRegistry;

pub use application_entity::{ApplicationEntity, normalize_ae_title};
pub use config::AssociationConfig as DeviceAssociationConfig;
pub use connection::Connection;
pub use host::Device;
pub use transfer_capability::{Role, TransferCapability, default_storage_scp_capabilities};

/// Builder for a DICOM network device (listener).
///
/// Prefer building a [`Device`] with [`ApplicationEntity`] and [`Connection`] directly.
#[deprecated(note = "use Device with ApplicationEntity and Connection instead")]
#[derive(Debug)]
pub struct DeviceBuilder {
    assoc_config: AssociationConfig,
    bind_addr: Option<SocketAddr>,
    services: ServiceRegistry,
}

#[allow(deprecated)]
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
    #[deprecated(note = "use Device with ApplicationEntity instead")]
    pub fn ae_title(mut self, ae_title: impl Into<String>) -> Self {
        self.assoc_config.ae_title = ae_title.into();
        self
    }

    /// Sets the local socket address to bind.
    #[deprecated(note = "use Device::add_connection instead")]
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    /// Sets association negotiation options.
    #[deprecated(note = "use ApplicationEntity transfer capabilities instead")]
    pub fn association_config(mut self, config: AssociationConfig) -> Self {
        self.assoc_config = config;
        self
    }

    /// Enables promiscuous mode (accept unknown SOP classes).
    #[deprecated(note = "use ApplicationEntity::promiscuous instead")]
    pub fn promiscuous(mut self, promiscuous: bool) -> Self {
        self.assoc_config.promiscuous = promiscuous;
        self
    }

    /// Enables strict max PDU length enforcement.
    #[deprecated(note = "use Connection::strict instead")]
    pub fn strict(mut self, strict: bool) -> Self {
        self.assoc_config.strict = strict;
        self
    }

    /// Restricts offered transfer syntaxes to uncompressed only.
    #[deprecated(note = "use ApplicationEntity::uncompressed_only instead")]
    pub fn uncompressed_only(mut self, value: bool) -> Self {
        self.assoc_config.set_uncompressed_only(value);
        self
    }

    /// Sets the maximum PDU length.
    #[deprecated(note = "use Connection::max_pdu_length instead")]
    pub fn max_pdu_length(mut self, max_pdu_length: u32) -> Self {
        self.assoc_config.max_pdu_length = max_pdu_length;
        self
    }

    /// Replaces the service registry used for incoming associations.
    #[deprecated(note = "use ApplicationEntity::register_service instead")]
    pub fn services(mut self, services: ServiceRegistry) -> Self {
        self.services = services;
        self
    }

    /// Registers a C-STORE service (indexes for streaming dispatch).
    #[deprecated(note = "use ApplicationEntity::register_cstore instead")]
    pub fn register_cstore(mut self, service: Arc<crate::scp::CStoreService>) -> Self {
        self.services.register_cstore(service);
        self
    }

    /// Registers a single DIMSE service.
    #[deprecated(note = "use ApplicationEntity::register_service instead")]
    pub fn register_service(mut self, service: Arc<dyn crate::service::DicomService>) -> Self {
        self.services.register(service);
        self
    }

    /// Builds a [`Device`] with a single connection and application entity.
    pub fn build(self) -> Device {
        let bind_addr = self
            .bind_addr
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 11111)));

        let mut conn = Connection::new()
            .port(bind_addr.port())
            .max_pdu_length(self.assoc_config.max_pdu_length)
            .strict(self.assoc_config.strict);
        if bind_addr.is_ipv4() {
            conn.hostname = bind_addr.ip().to_string();
        }

        let mut device = Device::new();
        let conn_index = device.add_connection(conn);

        let mut ae = ApplicationEntity::new(&self.assoc_config.ae_title)
            .acceptor(true)
            .promiscuous(self.assoc_config.promiscuous)
            .uncompressed_only(self.assoc_config.uncompressed_only)
            .add_connection(conn_index);

        if self.assoc_config.abstract_syntaxes.is_empty() {
            ae.add_default_storage_capabilities();
        } else {
            for uid in &self.assoc_config.abstract_syntaxes {
                ae.add_scp_capability(TransferCapability::storage_scp(
                    uid.clone(),
                    self.assoc_config.transfer_syntaxes.clone(),
                ));
            }
        }

        ae.services = self.services;
        device.add_application_entity(ae);
        device
    }

    /// Starts accepting associations and serving DIMSE requests.
    #[deprecated(note = "use Device::bind_connections instead")]
    pub async fn run(self) -> Result<()> {
        Arc::new(self.build()).bind_connections().await
    }
}

#[allow(deprecated)]
impl Default for DeviceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
