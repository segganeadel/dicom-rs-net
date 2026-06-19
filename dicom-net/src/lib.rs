//! # dicom-net
//!
//! DIMSE networking layer for [DICOM-rs](https://github.com/Enet4/dicom-rs).
//!
//! This crate sits **above** [`dicom-ul`] (associations and PDUs) and **below**
//! applications such as PACS components, gateways, and CLI tools.
//!
//! ## Goals
//!
//! - Pair command and data PDVs into DIMSE messages
//! - Provide SCP service traits and an SCU client API
//! - Offer a listener/device abstraction for production deployments
//! - Stay async-first and streaming-friendly
//!
//! ## Status
//!
//! **Early planning / skeleton.** APIs are unstable and most modules are stubs.
//! See the repository `docs/` folder for architecture and roadmap.
//!
//! [`dicom-ul`]: https://docs.rs/dicom-ul

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod association;
pub mod device;
pub mod dimse;
pub mod error;
pub mod scp;
pub mod scu;
pub mod service;
pub mod status;
pub mod transfer;

pub mod prelude {
    //! Commonly used types.
    pub use crate::association::AssociationContext;
    pub use crate::dimse::{Dimse, DimseMessage};
    pub use crate::error::{Error, Result};
    pub use crate::service::ServiceRegistry;
    pub use crate::status::Status;
}
