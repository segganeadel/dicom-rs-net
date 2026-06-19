# Architecture

## Layering

```
Application (PACS, gateways, CLI)
        |
    dicom-net   ← DIMSE messages, SCP/SCU, device model
        |
    dicom-ul    ← A-ASSOCIATE, P-DATA-TF, presentation contexts
        |
      TCP/TLS
```

## Device model (dcm4che-style)

Inspired by DICOM Part 15 Annex H and dcm4che's `Device` hierarchy:

```
Device
 ├── Connection[]          (host, port, PDU limits, timeouts)
 └── ApplicationEntity[]   (AE title, acceptor/initiator roles)
      ├── TransferCapability[]  (SOP class + SCU/SCP role + transfer syntaxes)
      └── ServiceRegistry       (per-AE DIMSE SCP handlers)
```

Multi-AE on one TCP port: `Device::bind_connections` reads each `AssociationRQ`, routes by **called AE title** to the matching `ApplicationEntity`, then completes negotiation with that AE's capabilities via vendored `dicom-ul::establish_async_with_rq` (see `vendor/dicom-ul/`).

## Modules

| Module | Responsibility |
|--------|----------------|
| `dimse` | Command fields, request/response encoding, `DimseMessage` |
| `association` | `AssociationContext` snapshot; SCP `dimse_loop` |
| `service` | `DicomService` trait and `ServiceRegistry` (SOP class routing, `*` promiscuous) |
| `scp` | C-ECHO and C-STORE SCP services |
| `scu` | `Client` and `ScuAssociation` for outbound DIMSE |
| `device` | `Device`, `Connection`, `ApplicationEntity`, `TransferCapability`; deprecated `DeviceBuilder` facade |

## SCP request flow

1. Accept TCP connection; read `AssociationRQ` PDU.
2. Resolve called AE title → `ApplicationEntity` on the bound `Connection`.
3. Negotiate association with per-AE `ServerAssociationOptions` (vendored `establish_async_with_rq`).
4. Read P-DATA PDUs, assemble command and data PDVs into `DimseMessage`.
5. Resolve SOP class through the AE's `ServiceRegistry`.
6. Invoke service handler (`CEchoService` or streaming `CStoreService`).
7. Encode response command and send P-DATA PDUs.

## SCU request flow

1. Configure an initiator `ApplicationEntity` with `TransferCapability` entries (or use `Client` facade).
2. Build `ClientAssociationOptions` from SCU capabilities and/or scanned files.
3. `establish_with_async` with proposed presentation contexts.
4. For C-ECHO: send command PDV, await response, release association.
5. For C-STORE: per file — encode command + dataset (transcode if needed), send P-DATA, await C-STORE-RSP.
6. Release association when all operations complete.

## Design principles

- **Async-first** — handlers are `async`; I/O stays on Tokio.
- **Streaming-friendly** — C-STORE SCP receives via `DatasetReader`; SCU chunks large sends by PDU size.
- **Thin over UL** — association state comes from `dicom-ul`; this crate does not reimplement PDU framing.
