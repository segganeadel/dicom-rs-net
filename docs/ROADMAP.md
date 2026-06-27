# Roadmap

## Status

**Phases 0-5:** complete -- DIMSE SCP/SCU, Device model, Study Root Q/R, TLS, observability.

**Phase 6a (PACS integration):** complete -- object-safe `CStoreSink`, Q/R CLI binaries.

**Phase 6b (Interop & API maturity):** complete -- optional interop CI; API **0.2.0** (`DeviceBuilder` removed, `qr` tier frozen).

**Phase 6c (SOP class registry):** complete -- grouped dcm4che dictionary; API **0.2.1** (patch, no API break).

**Later (Phase 7):** Patient Root Q/R; upstream `dicom-ul` multi-AE merge.

HTTP / FHIR / DICOMweb / **pacs-frontend** -> [pacs Phase 3](../../pacs/docs/ROADMAP.md).

### Cross-repo phase map

| dicom-rs-net | pacs | Focus |
|--------------|------|-------|
| Phases 0-5 | Foundation | DIMSE protocol engine |
| Phase 6a (done) | Phase 1 (done) | Archive integration + Q/R CLIs |
| Phase 6b (done) | Phase 1 (done) | Interop CI, API stabilization (0.2.0) |
| Phase 6c (done) | Phase 1 ingest maturity | Full storage SOP dict; `storescu` ingest without promiscuous |
| Phase 7 | Phase 2 | Patient Root Q/R |
| - | Phase 3 (nearly complete) | `pacs-web`, admin UI, OSS viewers |

## Phase 0 - Skeleton

- [x] Workspace and crate layout
- [x] DIMSE types, service trait, registry stub
- [x] Device and SCU stubs returning `NotImplemented`

## Phase 1 - SCP MVP

- [x] P-DATA read loop pairing command/data PDVs
- [x] C-ECHO SCP via device listener
- [x] Integration tests against `dicom-ul` test harness

## Phase 2 - C-STORE SCP

- [x] Streaming C-STORE receive
- [x] Transfer syntax negotiation per presentation context
- [x] Pluggable storage backend trait (`CStoreSink`)

## Phase 3 - SCU

- [x] Association + C-ECHO client (`Client::echo`, `ScuAssociation`)
- [x] C-STORE SCU with progress hooks (`storescu` CLI + `indicatif`)
- [x] Transcoding via `transcode` feature (`dicom-pixeldata`)
- [x] `storescu` binary
- [x] Interop docs and `#[ignore]` external tests
- [x] CI workflow (sibling `dicom-rs` checkout)

## Phase 3.5 - Device model

- [x] `Device` / `Connection` / `ApplicationEntity` / `TransferCapability` hierarchy
- [x] Multi-AE routing on one TCP port (`Device::bind_connections`)
- [x] Vendored `dicom-ul` with `process_association_rq` + `establish_async_with_rq` (pending upstream merge)
- [x] `ApplicationEntity` SCU (`connect`, `echo`, `store_files`)
- [x] `DeviceBuilder` deprecated facade; `storescp` migrated to `Device` API
- [x] Integration tests: `device_multi_ae`, `device_scu_ae`

## Phase 4 - Query/retrieve

- [x] Study Root C-FIND SCP/SCU (`CFindService`, `ApplicationEntity::find`, `find_roundtrip`)
- [x] Study Root C-MOVE SCP/SCU with move-destination map (`cmove_roundtrip`)
- [x] Study Root C-GET SCP/SCU on same association (`cget_roundtrip`)
- [x] Q/R DIMSE builders, status codes, `dimse_loop` multi-response dispatch
- [x] `qr` module: SOP UIDs, `QueryRetrieveLevel`, `TransferCapability` helpers

## Phase 5 - Production hardening

- [x] TLS on `Connection` (`tls` feature) with SCP/SCU wiring
- [x] SCU/SCP timeouts, `connection_timeout`, listen `backlog`, `max_concurrent_associations`
- [x] Structured `tracing` spans and `dicom_net.metrics` events for associations and Q/R
- [x] `docs/STABILITY.md` stability policy

## Phase 6a - PACS integration & tooling

- [x] Object-safe `CStoreSink` via `DatasetStream` (`Arc<dyn CStoreSink>`)
- [x] `findscu`, `movescu`, `getscu` CLI binaries

## Phase 6b - Interop & API maturity

- [x] Optional dcm4che interop CI job (`interop` feature or `DICOM_NET_INTEROP=1`; extend `dicom-net/tests/interop.rs`)
- [x] API stabilization **0.2.0** after **pacs** Phase 1 validation (`docs/STABILITY.md`: remove `DeviceBuilder`, freeze `qr` tier)

## Phase 6c - SOP class registry

- [x] Vendor dcm4che UI dictionary (`dicom-net/data/dcm4che-dict-cuids.js`)
- [x] Generator + `SopClassContext` grouped slices (`gen_sop_classes`, `sop_classes.rs`)
- [x] C-STORE negotiation via `C_STORE_SOP_CLASS_UIDS` (storage + DICOMDIR)
- [x] `transfer.rs` facade; `CStoreService` + `default_storage_scp_capabilities` rewired
- [x] Optional Q/R capability helpers on `ApplicationEntity`
- [x] Classification audit JSON, drift tests, README regen docs
- [x] C-STORE SCU: fragment large datasets across multiple P-DATA PDUs
- [x] Windows loopback firewall script for integration tests

## Phase 7 - Q/R extensions & upstream

- [ ] Patient Root C-FIND / C-MOVE / C-GET SCP/SCU (SOP UIDs in `dicom-net/src/qr.rs`; mirror Study Root services)
- [ ] Upstream merge: vendored `dicom-ul` multi-AE patch (ongoing maintenance, not blocking PACS Phase 1)
- [ ] Align PACS Q/R capabilities with `QUERY_FIND_*` / `QUERY_MOVE_*` slices (optional; PACS still uses explicit Study Root constants today)

HTTP/FHIR/DICOMweb, **pacs-frontend** (admin UI), and OSS viewer integration are **pacs** Phase 3 -- dicom-rs-net stays DIMSE-only.

See also: [pacs/docs/ROADMAP.md](../../pacs/docs/ROADMAP.md) for the archive product roadmap.
