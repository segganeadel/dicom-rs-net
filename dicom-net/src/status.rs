//! DICOM status codes for DIMSE responses (PS3.7 Annex C).

/// A DIMSE response status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Status(pub u16);

impl Status {
    /// Success.
    pub const SUCCESS: Self = Self(0x0000);

    /// Warning: coerced dataset.
    pub const COERCE: Self = Self(0x0001);

    /// Failure: cannot understand.
    pub const CANNOT_UNDERSTAND: Self = Self(0xC000);

    /// Failure: processing failure.
    pub const PROCESSING_FAILURE: Self = Self(0x0110);

    /// Failure: SOP class not supported.
    pub const SOP_CLASS_NOT_SUPPORTED: Self = Self(0x0112);

    /// Returns whether this status indicates success (high bit clear in category).
    pub const fn is_success(self) -> bool {
        self.0 & 0xFF00 == 0
    }

    /// Returns whether this status indicates a warning.
    pub const fn is_warning(self) -> bool {
        self.0 & 0xFF00 == 0x0100
    }

    /// Returns whether this status indicates failure.
    pub const fn is_failure(self) -> bool {
        self.0 & 0xFF00 == 0xC000
    }
}

impl From<u16> for Status {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_categories() {
        assert!(Status::SUCCESS.is_success());
        assert!(Status::from(0x0100).is_warning());
        assert!(Status::CANNOT_UNDERSTAND.is_failure());
    }
}
