# dicom-net

DIMSE networking layer for DICOM-rs.

## Overview

`dicom-net` pairs upper-layer PDUs into DIMSE messages and exposes traits for implementing SCP services and SCU clients. The device model follows a dcm4che-style hierarchy: `Device` → `Connection` + `ApplicationEntity` → `TransferCapability`.

## SCP example (device model)

```rust
use std::sync::Arc;

use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::scp::{CEchoService, CStoreService, FileCStoreSink};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut device = Device::new();
    let conn_index = device.add_connection(Connection::new().port(11111));

    let mut ae = ApplicationEntity::new("STORESCP")
        .acceptor(true)
        .add_connection(conn_index);
    ae.add_default_storage_capabilities();
    ae.register_service(Arc::new(CEchoService::new()));
    ae.register_cstore(Arc::new(CStoreService::new(Arc::new(FileCStoreSink::new("./received")))));
    device.add_application_entity(ae);

    Arc::new(device).bind_connections().await?;
    std::future::pending::<()>().await;
}
```

Multiple application entities can share one `Connection`; associations are routed by called AE title.

## SCU example (`ApplicationEntity`)

```rust
use std::path::PathBuf;

use dicom_net::device::{ApplicationEntity, Connection};
use dicom_net::scu::StoreOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::new();
    let ae = ApplicationEntity::new("STORESCU").initiator(true);

    ae.echo(&conn, "STORESCP@127.0.0.1:11111").await?;

    let mut ae = ApplicationEntity::new("STORESCU").initiator(true);
    ae.add_scu_storage_capabilities_from_files(&[PathBuf::from("image.dcm")], false)?;
    let sent = ae.store_files(
        &conn,
        "STORESCP@127.0.0.1:11111",
        &[PathBuf::from("image.dcm")],
        &StoreOptions::default(),
    )
    .await?;
    println!("sent {sent} file(s)");
    Ok(())
}
```

## SCU example (`Client` facade)

```rust
use dicom_net::scu::{Client, StoreOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Client::new()
        .calling_ae("STORESCU")
        .called_ae("STORESCP")
        .remote("STORESCP@127.0.0.1:11111")
        .echo()
        .await?;

    let sent = Client::new()
        .remote("STORESCP@127.0.0.1:11111")
        .store_files(&["image.dcm".into()], &StoreOptions::default())
        .await?;
    println!("sent {sent} file(s)");
    Ok(())
}
```

## Query/Retrieve example

```rust
use std::sync::Arc;

use dicom_net::device::transfer_capability::TransferCapability;
use dicom_net::device::{ApplicationEntity, Connection, Device};
use dicom_net::qr::STUDY_ROOT_FIND;
use dicom_net::scp::{CFindService, StaticCFindSink};

// SCP: register CFindService on an ApplicationEntity (see tests/find_roundtrip.rs).
// SCU:
let conn = Connection::new();
let mut ae = ApplicationEntity::new("FINDSCU").initiator(true);
ae.add_scu_capability(TransferCapability::query_retrieve_find_scu(STUDY_ROOT_FIND));
let matches = ae.find(&conn, "FINDSCP@127.0.0.1:11111", None).await?;
```

## Features

- `async` (default) — async-first APIs built on Tokio and `dicom-ul` async associations
- `transcode` (default) — C-STORE SCU transcoding via `dicom-pixeldata`
- `tls` — TLS on `Connection` (`tls_server_config` / `tls_client_config`) via `dicom-ul/async-tls`

## Documentation

Repository-level docs live in the parent `docs/` directory, including [`STABILITY.md`](../docs/STABILITY.md) and [`ROADMAP.md`](../docs/ROADMAP.md).
