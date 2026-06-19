//! TCP connection settings for a DICOM device.

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use crate::error::{Error, Result};

/// Network connection parameters for association establishment.
#[derive(Debug, Clone)]
pub struct Connection {
    /// Hostname or IP address to bind or connect from.
    pub hostname: String,
    /// TCP port.
    pub port: u16,
    /// Maximum PDU length negotiated on this connection.
    pub max_pdu_length: u32,
    /// Enforce strict PDU length limits.
    pub strict: bool,
    /// Listen backlog when used as an acceptor.
    pub backlog: u32,
    /// Per-read timeout for association I/O.
    pub read_timeout: Option<Duration>,
    /// Per-write timeout for association I/O.
    pub write_timeout: Option<Duration>,
    /// Timeout for outbound TCP connect (SCU).
    pub connection_timeout: Option<Duration>,
    /// TLS server configuration (SCP).
    #[cfg(feature = "tls")]
    pub tls_server_config: Option<std::sync::Arc<rustls::ServerConfig>>,
    /// TLS client configuration (SCU).
    #[cfg(feature = "tls")]
    pub tls_client_config: Option<std::sync::Arc<rustls::ClientConfig>>,
    /// TLS server name for SCU SNI.
    #[cfg(feature = "tls")]
    pub tls_server_name: Option<String>,
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

impl Connection {
    /// Creates a connection with default settings (`0.0.0.0:11111`).
    pub fn new() -> Self {
        Self {
            hostname: "0.0.0.0".to_string(),
            port: 11111,
            max_pdu_length: 16_378,
            strict: false,
            backlog: 50,
            read_timeout: None,
            write_timeout: None,
            connection_timeout: None,
            #[cfg(feature = "tls")]
            tls_server_config: None,
            #[cfg(feature = "tls")]
            tls_client_config: None,
            #[cfg(feature = "tls")]
            tls_server_name: None,
        }
    }

    /// Sets the TCP port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the maximum PDU length.
    pub fn max_pdu_length(mut self, max_pdu_length: u32) -> Self {
        self.max_pdu_length = max_pdu_length;
        self
    }

    /// Enables strict PDU length enforcement.
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Sets the listen backlog.
    pub fn backlog(mut self, backlog: u32) -> Self {
        self.backlog = backlog;
        self
    }

    /// Sets the read timeout for association I/O.
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    /// Sets the write timeout for association I/O.
    pub fn write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = Some(timeout);
        self
    }

    /// Sets the TCP connect timeout for SCU associations.
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    #[cfg(feature = "tls")]
    /// Sets the TLS server configuration for SCP associations.
    pub fn tls_server_config(mut self, config: std::sync::Arc<rustls::ServerConfig>) -> Self {
        self.tls_server_config = Some(config);
        self
    }

    #[cfg(feature = "tls")]
    /// Sets the TLS client configuration for SCU associations.
    pub fn tls_client_config(mut self, config: std::sync::Arc<rustls::ClientConfig>) -> Self {
        self.tls_client_config = Some(config);
        self
    }

    #[cfg(feature = "tls")]
    /// Sets the TLS server name (SNI) for SCU connections.
    pub fn tls_server_name(mut self, name: impl Into<String>) -> Self {
        self.tls_server_name = Some(name.into());
        self
    }

    /// Resolves the local socket address for binding.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        let addr = format!("{}:{}", self.hostname, self.port);
        addr.to_socket_addrs()
            .map_err(|source| Error::Io { source })?
            .next()
            .ok_or_else(|| Error::InvalidCommand {
                message: format!("could not resolve address {addr}"),
            })
    }
}
