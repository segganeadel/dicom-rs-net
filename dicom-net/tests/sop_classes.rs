//! Tests for dcm4che SOP class dictionary generation and classification.

use dicom_dictionary_std::uids;
use dicom_net::qr::STUDY_ROOT_FIND;
use dicom_net::sop_classes::{
    ALL_SOP_CLASS_UIDS, C_STORE_SOP_CLASS_UIDS, QUERY_FIND_SOP_CLASS_UIDS,
    SopClassContext, STORAGE_SOP_CLASS_UIDS,
};

const EXPECTED_TOTAL: usize = 375;

#[test]
fn all_sop_class_count_matches_dictionary() {
    assert_eq!(ALL_SOP_CLASS_UIDS.len(), EXPECTED_TOTAL);
}

#[test]
fn c_store_includes_standard_storage_classes() {
    assert!(C_STORE_SOP_CLASS_UIDS.contains(&uids::MR_IMAGE_STORAGE));
    assert!(C_STORE_SOP_CLASS_UIDS.contains(&uids::SECONDARY_CAPTURE_IMAGE_STORAGE));
    assert!(C_STORE_SOP_CLASS_UIDS.contains(&uids::CT_IMAGE_STORAGE));
}

#[test]
fn query_find_includes_study_root() {
    assert!(QUERY_FIND_SOP_CLASS_UIDS.contains(&STUDY_ROOT_FIND));
}

#[test]
fn verification_is_not_in_c_store_list() {
    assert!(!C_STORE_SOP_CLASS_UIDS.contains(&uids::VERIFICATION));
}

#[test]
fn classified_json_matches_expected_count() {
    let json = include_str!("../data/sop-classes-classified.json");
    let entries: Vec<serde_json::Value> = serde_json::from_str(json).expect("parse json");
    assert_eq!(entries.len(), EXPECTED_TOTAL);
}

#[test]
fn storage_context_slice_matches_storage_uids() {
    assert_eq!(
        SopClassContext::Storage.uids().len(),
        STORAGE_SOP_CLASS_UIDS.len()
    );
}

#[test]
fn all_uids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for uid in ALL_SOP_CLASS_UIDS {
        assert!(seen.insert(*uid), "duplicate uid: {uid}");
    }
}
