# dicom-net

DIMSE networking layer for DICOM-rs.

## Overview

`dicom-net` pairs upper-layer PDUs into DIMSE messages and exposes traits for implementing SCP services and SCU clients.

## Example (planned)

```rust
use dicom_net::prelude::*;
```

## Features

- `async` (default) — async-first APIs built on Tokio and `dicom-ul` async associations
- `tls` — TLS support via `dicom-ul/async-tls`

## Documentation

Repository-level docs live in the parent `docs/` directory.
