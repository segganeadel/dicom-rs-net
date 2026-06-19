# Stability policy

`dicom-net` is currently at **0.x**. Breaking API changes are allowed but should follow a short deprecation cycle when practical.

## Versioning

- **0.x** — Semver applies to the public API surface documented below. Minor releases may add features; patch releases are bug fixes only.
- **1.0** — Will commit to semver for the stable tiers listed here.

## Minimum Rust version (MSRV)

The workspace MSRV is **1.85** (see root `Cargo.toml`). CI enforces this.

## Public API tiers

| Tier | Modules / types | Stability intent |
|------|-----------------|------------------|
| Stable intent | `prelude`, `device::{Device, Connection, ApplicationEntity, TransferCapability}`, `scu::{Client, ScuAssociation}`, `scp` service types | Intended for application use; breaking changes require deprecation |
| Evolving | `qr`, Q/R helpers on `ApplicationEntity` | Feature-complete but may refine before 1.0 |
| Internal | `association::dimse_loop`, `service::registry` internals | Not covered by semver; may change without notice |

## Deprecation

- `DeviceBuilder` is deprecated in favor of `Device` + `ApplicationEntity` + `Connection`. It remains as a compatibility facade until 0.2 or 1.0.
- Deprecated items are annotated with `#[deprecated]` and documented in release notes.

## Features

| Feature | Purpose |
|---------|---------|
| `async` (default) | Tokio-based SCP/SCU |
| `transcode` (default) | C-STORE transcoding via `dicom-pixeldata` |
| `tls` | TLS on `Connection` via `dicom-ul` async TLS |

## Reporting issues

Report regressions or API concerns on the project repository. Include `dicom-net` version, enabled features, and a minimal reproduction when possible.
