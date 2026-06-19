//! DICOM DIMSE command field values (PS3.7).

/// Raw command-field tag value from a DIMSE command dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum CommandField {
    /// C-STORE-RQ
    CStoreRq = 0x0001,
    /// C-STORE-RSP
    CStoreRsp = 0x8001,
    /// C-GET-RQ
    CGetRq = 0x0010,
    /// C-GET-RSP
    CGetRsp = 0x8010,
    /// C-FIND-RQ
    CFindRq = 0x0020,
    /// C-FIND-RSP
    CFindRsp = 0x8020,
    /// C-MOVE-RQ
    CMoveRq = 0x0021,
    /// C-MOVE-RSP
    CMoveRsp = 0x8021,
    /// C-ECHO-RQ
    CEchoRq = 0x0030,
    /// C-ECHO-RSP
    CEchoRsp = 0x8030,
    /// C-CANCEL-RQ
    CCancelRq = 0x0FFF,
}

impl CommandField {
    /// Parses a command field from its on-wire value.
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(Self::CStoreRq),
            0x8001 => Some(Self::CStoreRsp),
            0x0010 => Some(Self::CGetRq),
            0x8010 => Some(Self::CGetRsp),
            0x0020 => Some(Self::CFindRq),
            0x8020 => Some(Self::CFindRsp),
            0x0021 => Some(Self::CMoveRq),
            0x8021 => Some(Self::CMoveRsp),
            0x0030 => Some(Self::CEchoRq),
            0x8030 => Some(Self::CEchoRsp),
            0x0FFF => Some(Self::CCancelRq),
            _ => None,
        }
    }

    /// Returns the underlying command-field value.
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    /// Returns whether this field denotes a response PDU.
    pub const fn is_response(self) -> bool {
        self.as_u16() & 0x8000 != 0
    }

    /// Maps this command field to the corresponding DIMSE operation.
    pub const fn dimse(self) -> super::Dimse {
        match self {
            Self::CStoreRq | Self::CStoreRsp => super::Dimse::CStore,
            Self::CGetRq | Self::CGetRsp => super::Dimse::CGet,
            Self::CFindRq | Self::CFindRsp => super::Dimse::CFind,
            Self::CMoveRq | Self::CMoveRsp => super::Dimse::CMove,
            Self::CEchoRq | Self::CEchoRsp => super::Dimse::CEcho,
            Self::CCancelRq => super::Dimse::CCancel,
        }
    }
}
