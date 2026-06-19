//! DIMSE message pairing (command + optional data).

use super::CommandField;

/// High-level DIMSE operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimse {
    /// C-ECHO
    CEcho,
    /// C-STORE
    CStore,
    /// C-FIND
    CFind,
    /// C-MOVE
    CMove,
    /// C-GET
    CGet,
    /// C-CANCEL
    CCancel,
}

/// A parsed DIMSE message (command dataset and optional payload).
#[derive(Debug, Clone)]
pub struct DimseMessage {
    /// Parsed command field.
    pub command_field: CommandField,
    /// DIMSE operation.
    pub dimse: Dimse,
    /// Message ID from the command dataset.
    pub message_id: u16,
    /// Presentation context ID for this operation.
    pub presentation_context_id: u8,
    /// Affected SOP class UID, when present in the command.
    pub affected_sop_class_uid: Option<String>,
    /// Affected SOP instance UID, when present in the command.
    pub affected_sop_instance_uid: Option<String>,
    /// Encoded command dataset bytes (Implicit VR Little Endian).
    pub command: Vec<u8>,
}

impl DimseMessage {
    /// Creates a message from a known command field and raw command bytes.
    pub fn new(
        command_field: CommandField,
        message_id: u16,
        presentation_context_id: u8,
        command: Vec<u8>,
    ) -> Self {
        Self {
            dimse: command_field.dimse(),
            command_field,
            message_id,
            presentation_context_id,
            affected_sop_class_uid: None,
            affected_sop_instance_uid: None,
            command,
        }
    }
}
