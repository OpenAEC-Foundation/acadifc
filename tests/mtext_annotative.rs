//! A plain MTEXT must not come back flagged annotative after a DXF round-trip.
//!
//! DXF carries MTEXT annotativeness via the annotation context (an
//! ObjectContextData / XDATA association), not an entity-level flag, so the
//! reader never sets `is_annotative` on the entity. A freshly-created,
//! non-annotative MTEXT must therefore read back with `is_annotative == false`
//! — regressing the old struct default of `true` would mark every imported DXF
//! MTEXT annotative and over-scale it in annotation-scaled viewports.

use std::io::Cursor;

use acadrust::entities::{EntityType, MText};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn dxf_roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("DXF write failed");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("DXF reader init failed")
        .read()
        .expect("DXF read failed")
}

#[test]
fn plain_mtext_not_annotative_after_dxf_roundtrip() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    doc.add_entity(EntityType::MText(MText::with_value(
        "Plain",
        Vector3::new(1.0, 2.0, 0.0),
    )))
    .unwrap();

    let rt = dxf_roundtrip(&doc);
    let mtext = rt
        .entities()
        .find_map(|e| {
            if let EntityType::MText(t) = e {
                Some(t)
            } else {
                None
            }
        })
        .expect("MTEXT survived DXF round-trip");

    assert!(
        !mtext.is_annotative,
        "a plain MTEXT must read back non-annotative from DXF"
    );
}
