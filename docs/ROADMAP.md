# Roadmap

## Phase 0 — Skeleton (current)

- [x] Workspace and crate layout
- [x] DIMSE types, service trait, registry stub
- [x] Device and SCU stubs returning `NotImplemented`

## Phase 1 — SCP MVP

- [ ] P-DATA read loop pairing command/data PDVs
- [ ] C-ECHO SCP via `DeviceBuilder::run`
- [ ] Integration tests against `echoscu` / `dicom-ul` test harness

## Phase 2 — C-STORE SCP

- [ ] Streaming C-STORE receive
- [ ] Transfer syntax negotiation per presentation context
- [ ] Pluggable storage backend trait

## Phase 3 — SCU

- [ ] Association + C-ECHO client
- [ ] C-STORE SCU with progress hooks

## Phase 4 — Query/retrieve

- [ ] C-FIND / C-MOVE / C-GET (priority TBD with adopters)

## Phase 5 — Production hardening

- [ ] TLS (`tls` feature)
- [ ] Timeouts, backpressure, metrics (`tracing`)
- [ ] Documented stability policy and semver
