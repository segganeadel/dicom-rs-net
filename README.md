# dicom-rs-net

Production-oriented DIMSE networking for [DICOM-rs](https://github.com/Enet4/dicom-rs).

This repository hosts **`dicom-net`**, a crate that sits above [`dicom-ul`](https://docs.rs/dicom-ul) (associations and PDUs) and provides:

- DIMSE message types (command + data pairing)
- SCP service traits and routing
- SCU client APIs
- A device/listener abstraction for long-running deployments

## Status

Early skeleton — APIs are unstable. See [`docs/`](docs/) for architecture and roadmap.

## Workspace

```bash
cargo check
```

## License

MIT OR Apache-2.0
