# acadifc

> **This repository is no longer under active development.** Active IFCCAD
> development has moved to
> [OpenAEC-Foundation/ifccad](https://github.com/OpenAEC-Foundation/ifccad).

acadifc is retained as a public migration record for the initial Rust work on
IFCCAD. Do not use this repository as the basis for new integrations or format
development.

## Where development continues

- [ifccad](https://github.com/OpenAEC-Foundation/ifccad) owns the standalone
  Rust implementation and language-neutral format contract for IFCCAD.
- [IFCCAD prototype](https://github.com/OpenAEC-Foundation/IFC-CAD) remains
  the model-first Python reference implementation and format laboratory.
- [cadcodec](https://github.com/HakanSeven12/cadcodec) provides DWG and DXF
  codecs and the CAD document model.
- [cadkernel](https://github.com/HakanSeven12/cadkernel) provides geometry and
  ACIS B-rep functionality.
- [acadrust](https://github.com/hakanaktt/acadrust) retains the earlier
  acadrust project and its attribution.

The primary `ifccad` crate deliberately remains independent of CAD codecs and
geometry engines. A future `ifccad-cad-document` companion crate is intended
to own conversion between loaded IFCCAD packages and the CAD runtime model.

## Why this repository remains available

acadifc began from imported acadrust history and temporarily hosted the first
Rust IFCCAD foundations. The clean IFCCAD repository was created from acadifc
commit `b0ba408f07e21fc540bff30b7430fdc4297524bf` without importing the unrelated
acadrust history.

The active repository records the detailed transition in its
[provenance document](https://github.com/OpenAEC-Foundation/ifccad/blob/main/PROVENANCE.md).
This repository remains readable so historical commits, pull requests, and
attribution can still be inspected. Existing consumers pinned to a historical
commit can continue to resolve it, but should plan migration to the active
repositories above.

## License

MPL-2.0 — see [LICENSE](LICENSE).
