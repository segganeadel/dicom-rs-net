# Planning

## Current scope

**dicom-rs-net** is the DIMSE protocol layer for [DICOM-rs](https://github.com/Enet4/dicom-rs). Study Root C-FIND / C-MOVE / C-GET are implemented. Persistent archive storage lives in the separate **pacs** repository.

## Design decisions

| Topic | Decision | Rationale |
|-------|----------|-----------|
| UL dependency | Path/git dependency on `dicom-rs` `ul` crate | Single source of truth for associations |
| Async runtime | Tokio | Matches `dicom-ul` async feature |
| Service model | `async_trait` + `Arc<dyn DicomService>` | Ergonomic multi-SOP registration |
| Storage hooks | `CStoreSink`, `CFindSink`, `CRetrieveSink` traits | Archive logic stays in application crates |
| Promiscuous SCP | `"*"` in `sop_classes()` | Mirrors common PACS “accept all storage” mode |
| Error crate | `snafu` | Consistent with ecosystem crates |

## Non-goals (library)

- Built-in persistent archive (see **pacs** repo)
- DICOMweb / WADO (see **pacs** Phase 3)
- HL7 / IHE workflow profiles

## Related projects

| Repo | Role |
|------|------|
| **pacs** | Archive product: storage, index, `pacs-server` |
| **dicom-rs** | Core dataset model and encoding |
