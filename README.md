# dicom-rs-net

Production-oriented DIMSE networking for [DICOM-rs](https://github.com/Enet4/dicom-rs).

This repository hosts **`dicom-net`**, a crate that sits above [`dicom-ul`](https://docs.rs/dicom-ul) (associations and PDUs) and provides:

- DIMSE message types (command + data pairing)
- SCP service traits and routing
- SCU client APIs
- A device/listener abstraction for long-running deployments

## Status

Early alpha — SCP and SCU (C-ECHO, C-STORE with transcoding) are functional. APIs are unstable.
See [`docs/`](docs/) for architecture, roadmap, and [interop testing](docs/INTEROP.md).

## Tools

| Binary | Description |
|--------|-------------|
| `storescp` | C-STORE SCP (receive DICOM files) |
| `storescu` | C-STORE SCU (send DICOM files) |

## Workspace layout

This crate depends on a sibling checkout of [dicom-rs](https://github.com/Enet4/dicom-rs):

```
parent/
  dicom-rs/
  dicom-rs-net/
```

## Workspace

```bash
cargo check
cargo test
```

## License

MIT OR Apache-2.0
