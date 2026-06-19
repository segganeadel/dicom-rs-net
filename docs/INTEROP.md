# Interoperability testing

Manual recipes for verifying dicom-rs-net against external DICOM implementations.

## Prerequisites

- Sibling checkout of [dicom-rs](https://github.com/Enet4/dicom-rs) (for path dependencies)
- dcm4che tools on `PATH` (`storescu`, optional `storescp`)
- Sample DICOM files (e.g. `C:\Users\adels\Documents\DICOM_EXAMPLES`)

Default AE titles and ports used below:

| Role | AE title | Port |
|------|----------|------|
| dicom-rs-net `storescp` | `STORESCP` | `11111` |
| dcm4che `storescu` calling AE | `STORESCU` | — |

## dcm4che storescu → dicom-rs-net storescp

**Terminal 1** — start Rust SCP:

```powershell
cd dicom-rs-net
cargo run -p storescp -- -v --promiscuous -p 11111 -o test-received
```

**Terminal 2** — send files with dcm4che:

```powershell
storescu -c STORESCP@127.0.0.1:11111 C:\Users\adels\Documents\DICOM_EXAMPLES\*.DCM
```

Expect: `storescu` exits 0; `test-received/` contains `<SOPInstanceUID>.dcm` files.

Use `--promiscuous` on the SCP for X-Ray Angiographic and Radiofluoroscopic SOP classes not in the default allowlist.

## dicom-rs-net storescu → external storescp

**Terminal 1** — start external SCP (dcm4che example on port 11112):

```powershell
storescp -b STORESCP:11112 -od C:\temp\dcm4che-received
```

**Terminal 2** — send with Rust SCU:

```powershell
cargo run -p storescu -- STORESCP@127.0.0.1:11112 C:\Users\adels\Documents\DICOM_EXAMPLES\MRBRAIN.DCM
```

## C-ECHO connectivity

```powershell
# Rust SCP running on 11111
cargo run -p dicom-net --example echo_client  # if added
# Or use integration test:
cargo test -p dicom-net echo_roundtrip
```

Programmatic C-ECHO from Rust:

```rust
use dicom_net::scu::Client;

Client::new()
    .calling_ae("STORESCU")
    .called_ae("STORESCP")
    .remote("STORESCP@127.0.0.1:11111")
    .echo()
    .await?;
```

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Association rejected | Match AE title and port on both sides |
| No presentation context | Use `--promiscuous` on SCP for unknown SOP classes |
| Transcode failure | Ensure `transcode` feature is enabled (default); or use `--never-transcode` and matching TS |
| Address in use | Stop prior SCP or change port on both sides |
