//! Example: Serialize and deserialize CAD data with serde + JSON.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example serde_json --features serde
//! ```
//!
//! This example demonstrates several practical use-cases:
//!
//! 1. Serialize individual entities to JSON
//! 2. Serialize an entire `CadDocument` to JSON and back
//! 3. Serialize selected entity types for a web API response
//! 4. Primitive type serialization
//! 5. Full pipeline: DXF → JSON → DXF (read → serialize → deserialize → write)

use acadrust::entities::*;
use acadrust::types::{Color, Vector3};
use acadrust::{CadDocument, DxfWriter};

fn main() -> acadrust::Result<()> {
    // ── 1. Serialize individual entities ────────────────────────────

    println!("═══ 1. Individual Entity Serialization ═══\n");

    let line = Line::from_coords(0.0, 0.0, 0.0, 100.0, 50.0, 0.0);
    let json = serde_json::to_string_pretty(&line).unwrap();
    println!("Line as JSON:\n{}\n", json);

    let circle = Circle::from_coords(50.0, 25.0, 0.0, 15.0);
    let json = serde_json::to_string_pretty(&circle).unwrap();
    println!("Circle as JSON:\n{}\n", json);

    // Deserialize back
    let circle2: Circle = serde_json::from_str(&json).unwrap();
    println!(
        "Deserialized circle: center={:?}, radius={}\n",
        circle2.center, circle2.radius
    );

    // ── 2. Full document round-trip ────────────────────────────────

    println!("═══ 2. Full Document Round-trip ═══\n");

    let mut doc = CadDocument::new();

    // Add some entities
    let mut line = Line::from_coords(0.0, 0.0, 0.0, 200.0, 100.0, 0.0);
    line.common.color = Color::from_index(1); // Red
    doc.add_entity(EntityType::Line(line))?;

    let mut circle = Circle::from_coords(100.0, 50.0, 0.0, 30.0);
    circle.common.color = Color::from_index(3); // Green
    doc.add_entity(EntityType::Circle(circle))?;

    let arc = Arc::from_center_radius_angles(
        Vector3::new(50.0, 50.0, 0.0),
        25.0,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    doc.add_entity(EntityType::Arc(arc))?;

    // Serialize the entire document
    let doc_json = serde_json::to_string_pretty(&doc).unwrap();
    println!("Document JSON length: {} bytes", doc_json.len());

    // Deserialize back
    let doc2: CadDocument = serde_json::from_str(&doc_json).unwrap();
    println!(
        "Deserialized document: {} entities, version={:?}\n",
        doc2.entities().count(),
        doc2.version
    );

    // ── 3. Extract entities as a JSON array (web API style) ────────

    println!("═══ 3. Web API Style — Entity List ═══\n");

    // Collect entities into a Vec for serialization
    let entities: Vec<&EntityType> = doc.entities().collect();
    let api_json = serde_json::to_string_pretty(&entities).unwrap();
    println!("Entities array JSON:\n{}\n", api_json);

    // ── 4. Serialize types directly ────────────────────────────────

    println!("═══ 4. Primitive Types ═══\n");

    let point = Vector3::new(42.0, 17.5, -3.0);
    println!("Vector3: {}", serde_json::to_string(&point).unwrap());

    let color = Color::Rgb {
        r: 255,
        g: 128,
        b: 0,
    };
    println!("Color:   {}", serde_json::to_string(&color).unwrap());

    // ── 5. Full pipeline: DXF → JSON → DXF ────────────────────────

    println!("\n═══ 5. Full Pipeline: DXF → JSON → DXF ═══\n");

    // Build a document with several entities (simulates reading a file)
    let mut source = CadDocument::new();

    let mut line = Line::from_coords(10.0, 20.0, 0.0, 300.0, 150.0, 0.0);
    line.common.color = Color::from_index(1);
    source.add_entity(EntityType::Line(line))?;

    let mut circle = Circle::from_coords(150.0, 75.0, 0.0, 40.0);
    circle.common.color = Color::from_index(5);
    source.add_entity(EntityType::Circle(circle))?;

    let arc = Arc::from_center_radius_angles(
        Vector3::new(80.0, 80.0, 0.0),
        20.0,
        0.0,
        std::f64::consts::PI,
    );
    source.add_entity(EntityType::Arc(arc))?;

    println!(
        "Source document: {} entities, version={:?}",
        source.entities().count(),
        source.version
    );

    // ── Serialize to JSON (e.g. send to a web client / store in DB) ──
    let json = serde_json::to_string(&source).unwrap();
    println!("Serialized to JSON: {} bytes", json.len());

    // ── Deserialize back (e.g. received from web client) ──
    let restored: CadDocument = serde_json::from_str(&json).unwrap();
    println!(
        "Deserialized back:  {} entities, version={:?}",
        restored.entities().count(),
        restored.version
    );

    // ── Write the restored document to DXF ──
    let output_path = "target/serde_roundtrip.dxf";
    DxfWriter::new(restored.clone()).write_to_file(output_path)?;
    let file_size = std::fs::metadata(output_path).unwrap().len();
    println!("Written to {output_path}: {file_size} bytes");

    // ── Verify: read the written file back ──
    let verify = acadrust::DxfReader::from_file(output_path)?.read()?;
    println!(
        "Verified:           {} entities, version={:?}",
        verify.entities().count(),
        verify.version
    );

    println!("\nDone!");

    Ok(())
}
