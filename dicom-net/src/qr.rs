//! Query/Retrieve SOP classes and query levels.

use dicom_dictionary_std::uids;

/// Query/Retrieve level for C-FIND, C-MOVE, and C-GET identifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryRetrieveLevel {
    /// Patient level.
    Patient,
    /// Study level.
    Study,
    /// Series level.
    Series,
    /// Image (instance) level.
    Image,
}

impl QueryRetrieveLevel {
    /// Returns the DICOM CS value for this level.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Patient => "PATIENT",
            Self::Study => "STUDY",
            Self::Series => "SERIES",
            Self::Image => "IMAGE",
        }
    }

    /// Returns levels allowed for Study Root information model.
    pub fn study_root_levels() -> &'static [Self] {
        &[Self::Study, Self::Series, Self::Image]
    }

    /// Returns whether this level is allowed for Study Root Q/R.
    pub fn is_study_root(self) -> bool {
        matches!(self, Self::Study | Self::Series | Self::Image)
    }
}

impl std::str::FromStr for QueryRetrieveLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim_end_matches([' ', '\0']) {
            "PATIENT" => Ok(Self::Patient),
            "STUDY" => Ok(Self::Study),
            "SERIES" => Ok(Self::Series),
            "IMAGE" => Ok(Self::Image),
            _ => Err(()),
        }
    }
}

/// Study Root Query/Retrieve FIND SOP class UID.
pub const STUDY_ROOT_FIND: &str = uids::STUDY_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_FIND;
/// Study Root Query/Retrieve MOVE SOP class UID.
pub const STUDY_ROOT_MOVE: &str = uids::STUDY_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_MOVE;
/// Study Root Query/Retrieve GET SOP class UID.
pub const STUDY_ROOT_GET: &str = uids::STUDY_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_GET;
/// Patient Root Query/Retrieve FIND SOP class UID.
pub const PATIENT_ROOT_FIND: &str = uids::PATIENT_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_FIND;
/// Patient Root Query/Retrieve MOVE SOP class UID.
pub const PATIENT_ROOT_MOVE: &str = uids::PATIENT_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_MOVE;
/// Patient Root Query/Retrieve GET SOP class UID.
pub const PATIENT_ROOT_GET: &str = uids::PATIENT_ROOT_QUERY_RETRIEVE_INFORMATION_MODEL_GET;

/// All standard Study Root Q/R SOP class UIDs.
pub static STUDY_ROOT_QR_SYNTAXES: &[&str] = &[STUDY_ROOT_FIND, STUDY_ROOT_MOVE, STUDY_ROOT_GET];
