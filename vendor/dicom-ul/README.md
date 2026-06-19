# Vendored `dicom-ul`

This is a patched copy of [`dicom-ul` 0.9.1](https://github.com/Enet4/dicom-rs/tree/master/ul) from [DICOM-rs](https://github.com/Enet4/dicom-rs).

## Why vendored?

`dicom-net` multi-AE SCP routing reads each `AssociationRQ`, selects an `ApplicationEntity` by called AE title, then completes negotiation with that AE's transfer capabilities. That requires splitting association read from negotiation:

- `ServerAssociationOptions::process_association_rq`
- `ServerAssociationOptions::establish_async_with_rq`
- `NegotiatedOptions` made public

Upstream `dicom-ul` 0.9.1 does not expose these APIs yet. This vendor keeps the sibling `dicom-rs` checkout read-only while `dicom-rs-net` depends on the patched UL layer.

## Patch scope

Changes are limited to:

- `src/association/mod.rs` — `NegotiatedOptions` visibility
- `src/association/server.rs` — `process_association_rq`, `establish_async_with_rq`, `establish_async` refactor

## Maintenance

When bumping DICOM-rs dependency versions:

1. Re-copy `ul/` from upstream into `vendor/dicom-ul/`
2. Re-apply the patch above
3. Update path dependencies in `Cargo.toml` if the sibling layout changes

Remove this vendor once the patch is merged and released upstream; then point workspace `dicom-ul` back at `../dicom-rs/ul` or crates.io.
