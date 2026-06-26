# Stability policy

`dicom-net` is currently at **0.x**. Breaking API changes are allowed but should follow a short deprecation cycle when practical.

## Versioning

- **0.x** — Semver applies to the public API surface documented below. Minor releases may add features; patch releases are bug fixes only.
- **1.0** — Will commit to semver for the stable tiers listed here.

**0.2.0** removes the deprecated `DeviceBuilder` facade. Use `Device` + `ApplicationEntity` + `Connection` instead.

## Minimum Rust version (MSRV)

The workspace MSRV is **1.85** (see root `Cargo.toml`). CI enforces this.

## Public API tiers

| Tier | Modules / types | Stability intent |
|------|-----------------|------------------|
| Stable intent | `prelude`, `device::{Device, Connection, ApplicationEntity, TransferCapability}`, `scu::{Client, ScuAssociation}`, `scp` service types | Intended for application use; breaking changes require deprecation |
| Frozen (0.2+) | `qr`, Q/R helpers on `ApplicationEntity` | Feature-complete; API frozen until 1.0 review |
| Internal | `association::dimse_loop`, `service::registry` internals | Not covered by semver; may change without notice |

## Deprecation

- `DeviceBuilder` was removed in **0.2.0**. Migrate to `Device` + `ApplicationEntity` + `Connection`.
- Deprecated items are annotated with `#[deprecated]` and documented in release notes.

## Features

| Feature | Purpose |
|---------|---------|
| `async` (default) | Tokio-based SCP/SCU |
| `transcode` (default) | C-STORE transcoding via `dicom-pixeldata` |
| `tls` | TLS on `Connection` via `dicom-ul` async TLS |

## Reporting issues

Report regressions or API concerns on the project repository. Include `dicom-net` version, enabled features, and a minimal reproduction when possible.
