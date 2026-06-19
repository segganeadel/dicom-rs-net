//! Error types for the DIMSE networking layer.

use snafu::Snafu;

/// Crate-wide result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised by the DIMSE networking layer.
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    /// An error propagated from the upper layer (`dicom-ul`).
    #[snafu(display("upper layer error: {source}"))]
    Ul {
        /// Source error.
        source: dicom_ul::association::Error,
    },

    /// Failed to read or parse a DIMSE command dataset.
    #[snafu(display("invalid DIMSE command: {message}"))]
    InvalidCommand {
        /// Human-readable detail.
        message: String,
    },

    /// Command field does not match the expected DIMSE operation.
    #[snafu(display("unexpected DIMSE operation: expected {expected:?}, got {got:#06x}"))]
    UnexpectedDimse {
        /// Expected operation.
        expected: crate::dimse::Dimse,
        /// Raw command field value.
        got: u16,
    },

    /// No service is registered for the requested SOP class.
    #[snafu(display("no service registered for SOP class {sop_class_uid}"))]
    UnknownSopClass {
        /// Requested SOP class UID.
        sop_class_uid: String,
    },

    /// No presentation context matches the request.
    #[snafu(display("no presentation context for abstract syntax {abstract_syntax}"))]
    MissingPresentationContext {
        /// Abstract syntax UID.
        abstract_syntax: String,
    },

    /// The peer returned a failure status.
    #[snafu(display("DIMSE failure status {status:#06x}"))]
    DimseFailure {
        /// Status code from the response.
        status: u16,
    },

    /// I/O error while handling an association.
    #[snafu(display("I/O error: {source}"))]
    Io {
        /// Source error.
        source: std::io::Error,
    },

    /// Catch-all for not-yet-implemented functionality.
    #[snafu(display("not implemented: {feature}"))]
    NotImplemented {
        /// Feature name.
        feature: &'static str,
    },

    /// Failed to encode a DIMSE response.
    #[snafu(display("failed to encode DIMSE response: {message}"))]
    EncodeResponse {
        /// Human-readable detail.
        message: String,
    },

    /// Remote address was not configured on the client.
    #[snafu(display("remote address not configured"))]
    RemoteNotConfigured,

    /// Could not read a DICOM file from disk.
    #[snafu(display("failed to read DICOM file {path}: {message}"))]
    ReadFile {
        /// File path.
        path: String,
        /// Human-readable detail.
        message: String,
    },

    /// No compatible presentation context was negotiated for a file.
    #[snafu(display("no presentation context for SOP class {sop_class}"))]
    NoPresentationContext {
        /// SOP class UID.
        sop_class: String,
    },

    /// Transcoding is required but disabled or not possible.
    #[snafu(display("transcoding required but not available: {message}"))]
    TranscodeRequired {
        /// Human-readable detail.
        message: String,
    },

    /// Transcoding operation failed.
    #[snafu(display("transcoding failed: {message}"))]
    Transcode {
        /// Human-readable detail.
        message: String,
    },

    /// Unsupported transfer syntax in a file.
    #[snafu(display("unsupported transfer syntax: {uid}"))]
    UnsupportedTransferSyntax {
        /// Transfer syntax UID.
        uid: String,
    },
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}
