//! Interoperability tests against external DICOM tools (dcm4che, dcmtk).
//!
//! Default: ignored (manual). Set `DICOM_NET_INTEROP=1` and have tools on PATH to run in CI.
//!
//! See `docs/INTEROP.md` for recipes.

use std::process::Command;

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--help")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore = "requires running storescp and dcm4che storescu on PATH"]
fn interop_dcm4che_storescu_to_rust_storescp() {
    if std::env::var("DICOM_NET_INTEROP").ok().as_deref() != Some("1") {
        return;
    }
    assert!(
        tool_available("storescu"),
        "DICOM_NET_INTEROP=1 but storescu not on PATH"
    );
    // Manual: start `cargo run -p storescp -- -v --promiscuous -p 11111 -o /tmp/received`
    // Then: `storescu -c STORESCP@127.0.0.1:11111 /path/to/*.DCM`
}

#[test]
#[ignore = "requires running external storescp and built storescu"]
fn interop_rust_storescu_to_external_storescp() {
    if std::env::var("DICOM_NET_INTEROP").ok().as_deref() != Some("1") {
        return;
    }
    assert!(
        tool_available("storescp"),
        "DICOM_NET_INTEROP=1 but storescp not on PATH"
    );
    // Manual: start external SCP on 11112, then:
    // `cargo run -p storescu -- STORESCP@127.0.0.1:11112 /path/to/file.dcm`
}

#[test]
fn interop_rust_qr_tools_build() {
    // Smoke check that Q/R CLI crates are workspace members.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("Cargo.toml");
    let text = std::fs::read_to_string(manifest).unwrap();
    assert!(text.contains("findscu"));
    assert!(text.contains("movescu"));
    assert!(text.contains("getscu"));
}
