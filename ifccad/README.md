# IFCCAD Format Contract

This directory contains the language-neutral IFCCAD format contract. The Rust
implementation lives separately in `src/ifccad` and is part of acadifc's core
API.

An IFCCAD package combines one IFCX document with one or more IFCDR drawing
resources and optional IFCPR preservation resources. The IFCX graph may
describe only a CAD drawing set, or also a project, building, and products from
which drawings are generated. The contract is being developed incrementally;
the schemas in this directory do not yet cover that complete vocabulary.

## Directories

- `schemas/` contains the authoritative schemas under active development.
- `conformance/next/`, when present, tests the active contract.
- `conformance/<version>/` contains an immutable, self-contained release of
  fixtures, vectors, expected outcomes, and the schemas applicable to that
  release.

Active schemas may move ahead of the latest released conformance suite. When a
new suite version is released, its applicable schemas are copied into that
versioned directory and frozen with the rest of the suite.

The provenance file in each imported suite records its historical source.
Ongoing IFCCAD contract development takes place in this repository.
