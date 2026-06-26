//! Generates grouped SOP class lists from dcm4che-dict-cuids.js.
//!
//! Run from repo root:
//!   cargo run --manifest-path dicom-net/scripts/Cargo.toml

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;

const SOURCE_URL: &str = "https://raw.githubusercontent.com/dcm4che/dcm4chee-arc-light/cd496ad7080b22f20a3f15ba73b0b8aa6c0059dc/dcm4chee-arc-ui2/src/app/constants/dcm4che-dict-cuids.js";
const SOURCE_COMMIT: &str = "cd496ad7080b22f20a3f15ba73b0b8aa6c0059dc";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "PascalCase")]
enum SopClassContext {
    Verification,
    MediaStorageDirectory,
    StorageCommitment,
    QueryFind,
    QueryMove,
    QueryGet,
    Mpps,
    Worklist,
    Print,
    Logging,
    MediaCreation,
    Display,
    Workflow,
    Storage,
    Private,
    Other,
}

impl SopClassContext {
    fn as_str(self) -> &'static str {
        match self {
            Self::Verification => "Verification",
            Self::MediaStorageDirectory => "MediaStorageDirectory",
            Self::StorageCommitment => "StorageCommitment",
            Self::QueryFind => "QueryFind",
            Self::QueryMove => "QueryMove",
            Self::QueryGet => "QueryGet",
            Self::Mpps => "Mpps",
            Self::Worklist => "Worklist",
            Self::Print => "Print",
            Self::Logging => "Logging",
            Self::MediaCreation => "MediaCreation",
            Self::Display => "Display",
            Self::Workflow => "Workflow",
            Self::Storage => "Storage",
            Self::Private => "Private",
            Self::Other => "Other",
        }
    }

    fn doc_comment(self) -> &'static str {
        match self {
            Self::Verification => "C-ECHO verification SOP classes.",
            Self::MediaStorageDirectory => "DICOMDIR media storage directory SOP classes.",
            Self::StorageCommitment => "Storage commitment push/pull SOP classes.",
            Self::QueryFind => "C-FIND query/retrieve information model SOP classes.",
            Self::QueryMove => "C-MOVE query/retrieve information model SOP classes.",
            Self::QueryGet => "C-GET and WADO retrieve SOP classes.",
            Self::Mpps => "Modality performed procedure step SOP classes.",
            Self::Worklist => "Modality worklist SOP classes.",
            Self::Print => "DICOM print management SOP classes.",
            Self::Logging => "Audit and event logging SOP classes.",
            Self::MediaCreation => "Media creation management SOP classes.",
            Self::Display => "Display system SOP classes.",
            Self::Workflow => "Detached management and notification workflow SOP classes.",
            Self::Storage => "Image and object storage SOP classes (C-STORE).",
            Self::Private => "Vendor-private SOP classes outside the DICOM UID root.",
            Self::Other => "Unclassified SOP classes reserved for future use.",
        }
    }

    fn rust_variant(self) -> &'static str {
        match self {
            Self::Verification => "Verification",
            Self::MediaStorageDirectory => "MediaStorageDirectory",
            Self::StorageCommitment => "StorageCommitment",
            Self::QueryFind => "QueryFind",
            Self::QueryMove => "QueryMove",
            Self::QueryGet => "QueryGet",
            Self::Mpps => "Mpps",
            Self::Worklist => "Worklist",
            Self::Print => "Print",
            Self::Logging => "Logging",
            Self::MediaCreation => "MediaCreation",
            Self::Display => "Display",
            Self::Workflow => "Workflow",
            Self::Storage => "Storage",
            Self::Private => "Private",
            Self::Other => "Other",
        }
    }

    fn slice_name(self) -> &'static str {
        match self {
            Self::Verification => "VERIFICATION_SOP_CLASS_UIDS",
            Self::MediaStorageDirectory => "MEDIA_STORAGE_DIRECTORY_SOP_CLASS_UIDS",
            Self::StorageCommitment => "STORAGE_COMMITMENT_SOP_CLASS_UIDS",
            Self::QueryFind => "QUERY_FIND_SOP_CLASS_UIDS",
            Self::QueryMove => "QUERY_MOVE_SOP_CLASS_UIDS",
            Self::QueryGet => "QUERY_GET_SOP_CLASS_UIDS",
            Self::Mpps => "MPPS_SOP_CLASS_UIDS",
            Self::Worklist => "WORKLIST_SOP_CLASS_UIDS",
            Self::Print => "PRINT_SOP_CLASS_UIDS",
            Self::Logging => "LOGGING_SOP_CLASS_UIDS",
            Self::MediaCreation => "MEDIA_CREATION_SOP_CLASS_UIDS",
            Self::Display => "DISPLAY_SOP_CLASS_UIDS",
            Self::Workflow => "WORKFLOW_SOP_CLASS_UIDS",
            Self::Storage => "STORAGE_SOP_CLASS_UIDS",
            Self::Private => "PRIVATE_SOP_CLASS_UIDS",
            Self::Other => "OTHER_SOP_CLASS_UIDS",
        }
    }
}

#[derive(Serialize)]
struct ClassifiedEntry {
    uid: String,
    name: String,
    context: SopClassContext,
}

fn classify(uid: &str, name: &str) -> SopClassContext {
    if uid == "1.2.840.10008.1.1" || name.starts_with("Verification SOP") {
        return SopClassContext::Verification;
    }
    if name.contains("Media Storage Directory Storage") {
        return SopClassContext::MediaStorageDirectory;
    }
    if name.contains("Storage Commitment") {
        return SopClassContext::StorageCommitment;
    }
    if name.ends_with(" - FIND") || name.contains("Information Model - FIND") {
        return SopClassContext::QueryFind;
    }
    if name.ends_with(" - MOVE") {
        return SopClassContext::QueryMove;
    }
    if name.ends_with(" - GET")
        || name.ends_with(" - C-GET")
        || name.contains("WADO Retrieve")
        || name.ends_with(" Query")
        || name.contains("Information Model - GET")
    {
        return SopClassContext::QueryGet;
    }
    if name.contains("Modality Performed Procedure Step") {
        return SopClassContext::Mpps;
    }
    if name.contains("Worklist") {
        return SopClassContext::Worklist;
    }
    if uid.starts_with("1.2.840.10008.5.1.1.") {
        return SopClassContext::Print;
    }
    if name.contains("Logging SOP Class") || name.contains("Event Logging") {
        return SopClassContext::Logging;
    }
    if name.contains("Media Creation") {
        return SopClassContext::MediaCreation;
    }
    if name.contains("Display System") {
        return SopClassContext::Display;
    }
    if (name.contains("Detached") && name.contains("Management"))
        || (name.contains("Notification SOP Class") && !name.contains("Storage"))
    {
        return SopClassContext::Workflow;
    }
    if name.contains("Storage") {
        return SopClassContext::Storage;
    }
    if !uid.starts_with("1.2.840.10008.") {
        return SopClassContext::Private;
    }
    SopClassContext::Other
}

fn parse_dictionary(js: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    for line in js.lines() {
        let line = line.trim();
        if !line.starts_with('"') {
            continue;
        }
        let Some((uid_part, name_part)) = line.split_once("\":\"") else {
            continue;
        };
        let uid = uid_part.trim_start_matches('"').to_string();
        let name = name_part
            .trim_end_matches("\",")
            .trim_end_matches('"')
            .to_string();
        if !uid.is_empty() && !name.is_empty() {
            entries.insert(uid, name);
        }
    }
    entries
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest_dir.join("../data");
    let src_dir = manifest_dir.join("../src");

    let js_path = data_dir.join("dcm4che-dict-cuids.js");
    let js = fs::read_to_string(&js_path).unwrap_or_else(|e| {
        panic!("read {}: {e}", js_path.display());
    });

    let dict = parse_dictionary(&js);
    let mut classified: Vec<ClassifiedEntry> = dict
        .iter()
        .map(|(uid, name)| ClassifiedEntry {
            uid: uid.clone(),
            name: name.clone(),
            context: classify(uid, name),
        })
        .collect();

    classified.sort_by(|a, b| a.uid.cmp(&b.uid));

    let json_path = data_dir.join("sop-classes-classified.json");
    let json = serde_json::to_string_pretty(&classified).expect("serialize json");
    fs::write(&json_path, json).expect("write json");

    let mut by_context: BTreeMap<SopClassContext, Vec<&str>> = BTreeMap::new();
    for entry in &classified {
        by_context
            .entry(entry.context)
            .or_default()
            .push(entry.uid.as_str());
    }

    let all_uids: Vec<&str> = classified.iter().map(|e| e.uid.as_str()).collect();

    let mut rust = String::new();
    rust.push_str("// @generated by gen-sop-classes; do not edit by hand.\n");
    rust.push_str(&format!("// Source: {SOURCE_URL}\n"));
    rust.push_str(&format!("// Commit: {SOURCE_COMMIT}\n"));
    rust.push_str(&format!("// Total UIDs: {}\n\n", all_uids.len()));

    rust.push_str(
        "/// Context group for a dcm4che dictionary SOP class.\n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n\
         #[non_exhaustive]\n\
         pub enum SopClassContext {\n",
    );
    for ctx in [
        SopClassContext::Verification,
        SopClassContext::MediaStorageDirectory,
        SopClassContext::StorageCommitment,
        SopClassContext::QueryFind,
        SopClassContext::QueryMove,
        SopClassContext::QueryGet,
        SopClassContext::Mpps,
        SopClassContext::Worklist,
        SopClassContext::Print,
        SopClassContext::Logging,
        SopClassContext::MediaCreation,
        SopClassContext::Display,
        SopClassContext::Workflow,
        SopClassContext::Storage,
        SopClassContext::Private,
        SopClassContext::Other,
    ] {
        rust.push_str(&format!("    /// {}\n    {},\n", ctx.doc_comment(), ctx.rust_variant()));
    }
    rust.push_str("}\n\n");

    rust.push_str("impl SopClassContext {\n");
    rust.push_str("    /// Returns the static UID slice for this context.\n");
    rust.push_str("    pub const fn uids(self) -> &'static [&'static str] {\n");
    rust.push_str("        match self {\n");
    for ctx in [
        SopClassContext::Verification,
        SopClassContext::MediaStorageDirectory,
        SopClassContext::StorageCommitment,
        SopClassContext::QueryFind,
        SopClassContext::QueryMove,
        SopClassContext::QueryGet,
        SopClassContext::Mpps,
        SopClassContext::Worklist,
        SopClassContext::Print,
        SopClassContext::Logging,
        SopClassContext::MediaCreation,
        SopClassContext::Display,
        SopClassContext::Workflow,
        SopClassContext::Storage,
        SopClassContext::Private,
        SopClassContext::Other,
    ] {
        rust.push_str(&format!(
            "            Self::{} => {},\n",
            ctx.rust_variant(),
            ctx.slice_name()
        ));
    }
    rust.push_str("        }\n    }\n}\n\n");

    rust.push_str(&format!(
        "/// All SOP class UIDs from the dcm4che dictionary ({} entries).\n",
        all_uids.len()
    ));
    rust.push_str("pub static ALL_SOP_CLASS_UIDS: &[&str] = &[\n");
    for uid in &all_uids {
        rust.push_str(&format!("    \"{uid}\",\n"));
    }
    rust.push_str("];\n\n");

    for ctx in [
        SopClassContext::Verification,
        SopClassContext::MediaStorageDirectory,
        SopClassContext::StorageCommitment,
        SopClassContext::QueryFind,
        SopClassContext::QueryMove,
        SopClassContext::QueryGet,
        SopClassContext::Mpps,
        SopClassContext::Worklist,
        SopClassContext::Print,
        SopClassContext::Logging,
        SopClassContext::MediaCreation,
        SopClassContext::Display,
        SopClassContext::Workflow,
        SopClassContext::Storage,
        SopClassContext::Private,
        SopClassContext::Other,
    ] {
        let uids = by_context.get(&ctx).map(|v| v.as_slice()).unwrap_or(&[]);
        rust.push_str(&format!(
            "/// `{}` context ({} entries).\n",
            ctx.as_str(),
            uids.len()
        ));
        rust.push_str(&format!("pub static {}: &[&str] = &[\n", ctx.slice_name()));
        for uid in uids {
            rust.push_str(&format!("    \"{uid}\",\n"));
        }
        rust.push_str("];\n\n");
    }

    let storage_receive: Vec<&str> = classified
        .iter()
        .filter(|e| {
            e.context == SopClassContext::Storage
                || e.context == SopClassContext::MediaStorageDirectory
        })
        .map(|e| e.uid.as_str())
        .collect();
    rust.push_str(&format!(
        "/// Storage SOP classes accepted for C-STORE ({} entries).\n",
        storage_receive.len()
    ));
    rust.push_str("pub static C_STORE_SOP_CLASS_UIDS: &[&str] = &[\n");
    for uid in &storage_receive {
        rust.push_str(&format!("    \"{uid}\",\n"));
    }
    rust.push_str("];\n");

    let out_path = src_dir.join("sop_classes.rs");
    fs::write(&out_path, rust).expect("write sop_classes.rs");

    println!(
        "Wrote {} ({} UIDs) and {}",
        out_path.display(),
        all_uids.len(),
        json_path.display()
    );
    for (ctx, uids) in &by_context {
        println!("  {}: {}", ctx.as_str(), uids.len());
    }
}
