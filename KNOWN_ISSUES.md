# Known Issues

A running list of acadrust parser limitations that have been investigated
but not yet fixed. Each entry should have a concrete repro file or test
case so the next person picking it up doesn't start from zero.

---

## DWG R2007 (AC1021) HATCH bit-stream misalignment

**Status:** open
**Affects:** all HATCH entities in any AC1021 (R2007 / R2007-R2010) DWG
**Symptom:** Most hatches in an R2007 file parse as empty (`paths = 0`)
or as junk (`paths = 100000`, `normal = (0,0,0)`, `elevation` reads back
as an absurd value like `-3.9e75`, `flags` contains garbage bits). The
~14% that report non-zero `edges` still carry corrupted side-fields, so
even those hatches won't render with valid boundaries downstream.
**Not affected:** AC1027 (R2013) and AC1032 (R2018) files. Other entity
types (LINE, CIRCLE, ARC, INSERT, WIPEOUT, 3DFACE, …) parse correctly
in R2007 files — only the HATCH-specific bit stream is wrong.

### Reproduction

`anteen.dwg` (downloaded sample) is AC1021. Run the project's
`inspect_hatch` example (or any equivalent walker) on it:

```
$ examples/inspect_hatch.exe anteen.dwg
…
Totals: hatches=279  zero_paths=240  zero_edges=241  valid_with_edges=38
```

For comparison, the same example on `sample-file-R.dwg` (AC1027):

```
Totals: hatches=393  zero_paths=7  zero_edges=7  valid_with_edges=386
```

### What is known

- `read_common_entity_data` is fine for AC1021 — every other entity type
  in the file decodes correctly, which means the cursor is correctly
  positioned when `read_hatch` is called.
- `read_hatch` itself uses `version.r2004_plus()` for the gradient
  block and `version.r2010_plus()` for spline fit points. The R2010+
  branch is correctly *not* taken for R2007 (so it's not over-reading
  there). Yet the values that come out are inconsistent with the file.
- libredwg's `objects.spec` (HATCH decoder) reads the gradient flags as
  `BL` (acadrust matches) and the tint as `BD` (acadrust matches). So
  the most obvious field-type confusion is ruled out.
- The misalignment is *consistent* across all 279 hatches in
  `anteen.dwg`, so this isn't junk data — it's a real version-specific
  layout the current reader is unaware of. Likely a missing R2007-only
  field somewhere between the common entity preamble and the boundary
  paths, or a different encoding of one of the gradient / pattern
  fields specific to R2007.

### Next steps when this is picked up

1. Capture the byte offset where `read_hatch` starts for one of the
   broken hatches (e.g. `0x50E98` in `anteen.dwg`).
2. Walk the bit cursor field-by-field with logging and compare against
   the byte slice using a hex dump.
3. Cross-reference with libredwg's HATCH decoder for any
   `R2007_only` / `R2010_skip` switches we don't have. The bit-stream
   spec in `objects.spec` is the authoritative reference; the
   `decode.c` paths for AC1021 vs AC1024 are the diff to read.
4. If a field difference is found, add a `version.is_r2007()` branch
   in `read_hatch` / `read_hatch_boundary_path`.

### Workaround for downstream consumers

Drop hatches whose `paths` is empty, or whose `normal` has
`(x² + y² + z² − 1).abs() > 0.21` (the same corrupt-entity guard
H7CAD already uses for polylines). The remaining ~14% of "valid"
hatches in the file still have junk side-fields and probably
shouldn't be rendered either; treating any R2007 hatch as suspect
until the parser is fixed is the safer choice.
