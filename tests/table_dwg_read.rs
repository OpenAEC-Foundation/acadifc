//! Verifies the ACAD_TABLE DWG reader against a real, AutoCAD-authored fixture.
//!
//! A table is INSERT-derived; the reader reads its shared insert base
//! (transform + block reference) so the entity is positioned and linked to the
//! anonymous block that renders its cells. Per-cell content is intentionally
//! not parsed — the object reader seeks to each object by offset, so reading
//! only the base is safe and never desyncs the following objects.

use acadrust::entities::EntityType;
use acadrust::types::Handle;
use acadrust::DwgReader;

#[test]
fn blocks_and_tables_metric_yields_linked_tables() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/roundtrip/blocks_and_tables_metric.dwg"
    );
    if !std::path::Path::new(path).exists() {
        eprintln!("fixture blocks_and_tables_metric.dwg missing — skipping");
        return;
    }

    let mut reader = DwgReader::from_file(path).expect("open blocks_and_tables_metric.dwg");
    let doc = reader.read().expect("read blocks_and_tables_metric.dwg");

    let tables: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            EntityType::Table(t) => Some(t),
            _ => None,
        })
        .collect();

    // This drawing contains two tables.
    assert_eq!(tables.len(), 2, "expected two ACAD_TABLE entities");

    for t in &tables {
        // Each table must be positioned and carry a valid, resolvable block
        // reference — that anonymous block renders the table's cells.
        let block = t
            .block_record_handle
            .expect("table missing block_record_handle");
        assert_ne!(block, Handle::NULL, "table block handle is null");
        assert!(
            doc.block_records.iter().any(|b| b.handle == block),
            "table block_record_handle {block:?} does not resolve to a block record"
        );
        // A real table sits at a finite location with a unit-Z normal here.
        assert!(t.insertion_point.x.is_finite() && t.insertion_point.y.is_finite());
        assert_eq!(t.normal.z, 1.0, "unexpected table normal");
    }
}
