# acadifc

IFCCAD package handling, canonicalisation and conformance support.

```toml
[dependencies]
acadifc = "0.5"
```

The crate exposes canonical value encoding, fingerprints, package diagnostics
and conformance manifest verification through `acadifc::ifccad`.

DWG/DXF and ACIS records belong to `cadcodec`. Geometry and ACIS B-rep
conversion belong to `cadkernel`.
