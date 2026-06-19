# Architecture

## Layering

```
Application (PACS, gateways, CLI)
        |
    dicom-net   ← DIMSE messages, SCP/SCU, device listener
        |
    dicom-ul    ← A-ASSOCIATE, P-DATA-TF, presentation contexts
        |
      TCP/TLS
```

## Modules

| Module | Responsibility |
|--------|----------------|
| `dimse` | Command fields, `DimseMessage` (command + optional dataset bytes) |
| `association` | `AssociationContext` snapshot for handlers |
| `service` | `DicomService` trait and `ServiceRegistry` (SOP class routing, `*` promiscuous) |
| `scp` | Re-exports for SCP implementers |
| `scu` | `Client` for outbound DIMSE |
| `device` | `DeviceBuilder` listener orchestration |

## Request flow (target)

1. Accept TCP connection and perform association via `dicom-ul`.
2. Read P-DATA PDUs, assemble command and data PDVs into `DimseMessage`.
3. Resolve SOP class through `ServiceRegistry`.
4. Invoke `DicomService::handle` with `AssociationContext` and payload.
5. Encode response command (and data) and send P-DATA PDUs.

## Design principles

- **Async-first** — handlers are `async`; I/O stays on Tokio.
- **Streaming-friendly** — large C-STORE payloads should not require full buffering (future work).
- **Thin over UL** — association state comes from `dicom-ul`; this crate does not reimplement PDU framing.
