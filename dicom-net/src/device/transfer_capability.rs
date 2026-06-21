//! Transfer capabilities advertised by an application entity.

use dicom_dictionary_std::uids;
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;

use crate::transfer::ABSTRACT_SYNTAXES;

/// Role of an application entity for a given SOP class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Service class user (initiator).
    Scu,
    /// Service class provider (acceptor).
    Scp,
}

/// A SOP class, role, and supported transfer syntaxes.
#[derive(Debug, Clone)]
pub struct TransferCapability {
    /// Abstract syntax (SOP class) UID.
    pub sop_class: String,
    /// Whether this AE acts as SCU or SCP for the SOP class.
    pub role: Role,
    /// Transfer syntax UIDs offered or requested.
    pub transfer_syntaxes: Vec<String>,
}

impl TransferCapability {
    /// Creates a new transfer capability.
    pub fn new(sop_class: impl Into<String>, role: Role, transfer_syntaxes: Vec<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role,
            transfer_syntaxes,
        }
    }

    /// Verification SOP as SCP with implicit and explicit VR little endian.
    pub fn verification_scp() -> Self {
        Self {
            sop_class: uids::VERIFICATION.to_string(),
            role: Role::Scp,
            transfer_syntaxes: vec![
                uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string(),
                uids::EXPLICIT_VR_LITTLE_ENDIAN.to_string(),
            ],
        }
    }

    /// Verification SOP as SCU.
    pub fn verification_scu() -> Self {
        Self {
            sop_class: uids::VERIFICATION.to_string(),
            role: Role::Scu,
            transfer_syntaxes: vec![
                uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string(),
                uids::EXPLICIT_VR_LITTLE_ENDIAN.to_string(),
            ],
        }
    }

    /// Storage SOP as SCP with the given transfer syntaxes.
    pub fn storage_scp(sop_class: impl Into<String>, transfer_syntaxes: Vec<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scp,
            transfer_syntaxes,
        }
    }

    /// Storage SOP as SCU with the given transfer syntaxes.
    pub fn storage_scu(sop_class: impl Into<String>, transfer_syntaxes: Vec<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scu,
            transfer_syntaxes,
        }
    }

    /// Study Root Q/R FIND as SCP.
    pub fn query_retrieve_find_scp(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scp,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Study Root Q/R FIND as SCU.
    pub fn query_retrieve_find_scu(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scu,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Study Root Q/R MOVE as SCP.
    pub fn query_retrieve_move_scp(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scp,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Study Root Q/R MOVE as SCU.
    pub fn query_retrieve_move_scu(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scu,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Study Root Q/R GET as SCP.
    pub fn query_retrieve_get_scp(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scp,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Study Root Q/R GET as SCU.
    pub fn query_retrieve_get_scu(sop_class: impl Into<String>) -> Self {
        Self {
            sop_class: sop_class.into(),
            role: Role::Scu,
            transfer_syntaxes: default_qr_transfer_syntaxes(),
        }
    }

    /// Builds SCP capabilities from a static list of abstract syntax UIDs.
    pub fn scp_from_static_list(syntaxes: &[&str], uncompressed_only: bool) -> Vec<Self> {
        let transfer_syntaxes = default_transfer_syntaxes(uncompressed_only);
        syntaxes
            .iter()
            .map(|s| Self::storage_scp(*s, transfer_syntaxes.clone()))
            .collect()
    }
}

/// Default transfer syntax UIDs for storage SCP capabilities.
pub fn default_transfer_syntaxes(uncompressed_only: bool) -> Vec<String> {
    if uncompressed_only {
        vec![
            uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string(),
            uids::EXPLICIT_VR_LITTLE_ENDIAN.to_string(),
        ]
    } else {
        TransferSyntaxRegistry
            .iter()
            .filter(|ts| !ts.is_unsupported())
            .map(|ts| ts.uid().to_string())
            .collect()
    }
}

fn default_qr_transfer_syntaxes() -> Vec<String> {
    vec![
        uids::IMPLICIT_VR_LITTLE_ENDIAN.to_string(),
        uids::EXPLICIT_VR_LITTLE_ENDIAN.to_string(),
    ]
}

/// Builds default storage SCP capabilities from [`ABSTRACT_SYNTAXES`].
pub fn default_storage_scp_capabilities(uncompressed_only: bool) -> Vec<TransferCapability> {
    TransferCapability::scp_from_static_list(ABSTRACT_SYNTAXES, uncompressed_only)
}
