# acadifc

Rust foundations for the open IFCCAD exchange format.

## What is IFCCAD?

IFCCAD brings IFC-style project and building semantics together with CAD
drawings in an IFCX-based package architecture. A package combines:

- one **IFCX** document containing the semantic graph and resource references;
- one or more **IFCDR** resources containing drawing data; and
- optional **IFCPR** resources preserving source-format information that cannot
  yet be represented natively.

The IFCX graph can contain only a CAD drawing set, or a broader project,
building, and product model from which drawing resources are generated. IFCDR
resources may therefore be authored directly, imported from CAD, generated
from building elements, or retained as a cache.

IFCCAD sits between IFC semantics and CAD exchange. It is not the same as
support for conventional IFC files, though conventional IFC import and export
can become part of the wider workflow.

## Current status

acadifc currently provides:

- language-neutral canonical value encoding and SHA-256 fingerprints;
- conformance manifests, vectors, fixtures, and verification helpers;
- the initial IFCDR registry and IFCPR schema; and
- structured package diagnostics and internal directory-package foundations.

The public package loader, a complete IFCCAD vocabulary within IFCX,
production IFCDR codecs, the future `.ifccad` container, CAD conversion, and
conventional IFC integration are still under development. The IFCCAD modules
are part of the core crate and do not require a Cargo feature.

## Using the current API

```toml
[dependencies]
acadifc = { git = "https://github.com/OpenAEC-Foundation/acadifc.git" }
```

```rust
use acadifc::ifccad::canonicalization::{fingerprint, CanonicalValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = CanonicalValue::String("drawing-001".to_owned());
    let digest = fingerprint(&value)?;
    println!("{digest}");
    Ok(())
}
```

The API is still evolving. Canonicalisation, conformance support, and the
stable diagnostic vocabulary are currently public; directory-package loading
remains internal until its contract is mature.

## Repository layout

- [`src/ifccad`](src/ifccad) contains the Rust implementation.
- [`ifccad/schemas`](ifccad/schemas) contains the active language-neutral
  schemas.
- [`ifccad/conformance`](ifccad/conformance) contains versioned, immutable
  conformance collections.

CAD codecs and geometry are developed independently:

- [cadcodec](https://github.com/HakanSeven12/cadcodec) handles DWG, DXF, and
  ACIS records.
- [cadkernel](https://github.com/HakanSeven12/cadkernel) provides geometry and
  ACIS B-rep conversion.

acadifc does not re-export these crates. Future CAD exchange will integrate
them at the conversion boundary while keeping the IFCCAD format model
independent.

## License

MPL-2.0 — see [LICENSE](LICENSE).
