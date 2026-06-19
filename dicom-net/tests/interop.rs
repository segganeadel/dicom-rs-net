//! Manual interop tests against external DICOM tools (dcm4che, dcmtk).
//!
//! These tests are ignored by default because they require external binaries
//! and a manually started SCP. See `docs/INTEROP.md` for recipes.

/// dcm4che storescu -> dicom-rs-net storescp
#[test]
#[ignore = "requires running storescp and dcm4che storescu on PATH"]
fn interop_dcm4che_storescu_to_rust_storescp() {
    // Manual: start `cargo run -p storescp -- -v --promiscuous -p 11111 -o /tmp/received`
    // Then: `storescu -c STORESCP@127.0.0.1:11111 /path/to/*.DCM`
    assert!(true);
}

/// dicom-rs-net storescu -> dcm4che or dicom-rs storescp
#[test]
#[ignore = "requires running external storescp and built storescu"]
fn interop_rust_storescu_to_external_storescp() {
    // Manual: start external SCP on 11112, then:
    // `cargo run -p storescu -- STORESCP@127.0.0.1:11112 /path/to/file.dcm`
    assert!(true);
}
