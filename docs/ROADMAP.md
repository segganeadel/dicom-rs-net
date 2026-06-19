# Roadmap

## Phase 0 — Skeleton

- [x] Workspace and crate layout
- [x] DIMSE types, service trait, registry stub
- [x] Device and SCU stubs returning `NotImplemented`

## Phase 1 — SCP MVP

- [x] P-DATA read loop pairing command/data PDVs
- [x] C-ECHO SCP via device listener
- [x] Integration tests against `dicom-ul` test harness

## Phase 2 — C-STORE SCP

- [x] Streaming C-STORE receive
- [x] Transfer syntax negotiation per presentation context
- [x] Pluggable storage backend trait (`CStoreSink`)

## Phase 3 — SCU

- [x] Association + C-ECHO client (`Client::echo`, `ScuAssociation`)
- [x] C-STORE SCU with progress hooks (`storescu` CLI + `indicatif`)
- [x] Transcoding via `transcode` feature (`dicom-pixeldata`)
- [x] `storescu` binary
- [x] Interop docs and `#[ignore]` external tests
- [x] CI workflow (sibling `dicom-rs` checkout)

## Phase 3.5 — Device model

- [x] `Device` / `Connection` / `ApplicationEntity` / `TransferCapability` hierarchy
- [x] Multi-AE routing on one TCP port (`Device::bind_connections`)
- [x] Vendored `dicom-ul` with `process_association_rq` + `establish_async_with_rq` (pending upstream merge)
- [x] `ApplicationEntity` SCU (`connect`, `echo`, `store_files`)
- [x] `DeviceBuilder` deprecated facade; `storescp` migrated to `Device` API
- [x] Integration tests: `device_multi_ae`, `device_scu_ae`

## Phase 4 — Query/retrieve

- [x] Study Root C-FIND SCP/SCU (`CFindService`, `ApplicationEntity::find`, `find_roundtrip`)
- [x] Study Root C-MOVE SCP/SCU with move-destination map (`cmove_roundtrip`)
- [x] Study Root C-GET SCP/SCU on same association (`cget_roundtrip`)
- [x] Q/R DIMSE builders, status codes, `dimse_loop` multi-response dispatch
- [x] `qr` module: SOP UIDs, `QueryRetrieveLevel`, `TransferCapability` helpers

## Phase 5 — Production hardening

- [x] TLS on `Connection` (`tls` feature) with SCP/SCU wiring
- [x] SCU/SCP timeouts, `connection_timeout`, listen `backlog`, `max_concurrent_associations`
- [x] Structured `tracing` spans and `dicom_net.metrics` events for associations and Q/R
- [x] `docs/STABILITY.md` stability policy
