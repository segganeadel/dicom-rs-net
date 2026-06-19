# Planning

## MVP definition

**Goal:** A minimal async SCP that accepts associations and responds to C-ECHO, with a clear extension path to C-STORE.

### Milestones

1. **M1 — Types compile, docs in place** — `cargo check`, public module boundaries frozen at coarse granularity.
2. **M2 — Echo SCP** — `DeviceBuilder::run` listens, routes verification SOP to a default or user service.
3. **M3 — Registry** — Multiple SOP classes; promiscuous `*` handler for lab / gateway scenarios.
4. **M4 — First SCU** — `Client::echo` for connectivity checks.

## Decisions

| Topic | Decision | Rationale |
|-------|----------|-----------|
| UL dependency | Path/git dependency on `dicom-rs` `ul` crate | Single source of truth for associations |
| Async runtime | Tokio | Matches `dicom-ul` async feature |
| Service model | `async_trait` + `Arc<dyn DicomService>` | Ergonomic multi-SOP registration |
| Promiscuous SCP | `"*"` in `sop_classes()` | Mirrors common PACS “accept all storage” mode |
| Error crate | `snafu` | Consistent with ecosystem crates |

## Non-goals (MVP)

- Full Q/R implementation
- DICOMweb or WADO
- Built-in persistent archive (storage is application responsibility)
