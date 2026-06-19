//! SCU (service class user) client API.

use std::net::SocketAddr;

use crate::error::{Error, Result};

/// DIMSE client stub for calling remote SCPs.
#[derive(Debug, Default)]
pub struct Client {
    calling_ae: Option<String>,
    called_ae: Option<String>,
    remote_addr: Option<SocketAddr>,
}

impl Client {
    /// Creates a new unconfigured client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the calling AE title.
    pub fn calling_ae(mut self, ae_title: impl Into<String>) -> Self {
        self.calling_ae = Some(ae_title.into());
        self
    }

    /// Sets the called AE title.
    pub fn called_ae(mut self, ae_title: impl Into<String>) -> Self {
        self.called_ae = Some(ae_title.into());
        self
    }

    /// Sets the remote SCP address.
    pub fn connect_to(mut self, addr: SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    /// Establishes an association and performs C-ECHO.
    pub async fn echo(self) -> Result<()> {
        let _ = (self.calling_ae, self.called_ae, self.remote_addr);
        Err(Error::NotImplemented { feature: "C-ECHO SCU" })
    }
}
