//! Diagnostic: count handles read vs handles written
//!
//! Reads the samplekitchen DWG file, counts all object handles, entity handles,
//! and then performs a DWG write and reports handle map size.

use acadrust::{DwgReader, DwgWriter};
use std::io::Cursor;

fn main() {
    let path = "tests/roundtrip/samplekitchen.dwg";
    
    // Read
    let mut reader = DwgReader::from_file(path).expect("Failed to open DWG");
    let doc = reader.read().expect("Failed to read DWG");
    
    eprintln!("=== Document Statistics ===");
    eprintln!("Version: {:?}", doc.version);
    eprintln!("Entities: {}", doc.entities().count());
    eprintln!("Objects: {}", doc.objects.len());
    eprintln!("Block records: {}", doc.block_records.len());
    eprintln!("Layers: {}", doc.layers.len());
    eprintln!("Text styles: {}", doc.text_styles.len());
    eprintln!("Line types: {}", doc.line_types.len());
    eprintln!("Dim styles: {}", doc.dim_styles.len());
    eprintln!("Views: {}", doc.views.len());
    eprintln!("Vports: {}", doc.vports.len());
    eprintln!("UCSs: {}", doc.ucss.len());
    eprintln!("App IDs: {}", doc.app_ids.len());
    eprintln!("Classes: {}", doc.classes.len());
    eprintln!("Handle seed: 0x{:X}", doc.header.handle_seed);

    // Count entity types
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in doc.entities() {
        let name = match e {
            acadrust::entities::EntityType::Line(_) => "Line",
            acadrust::entities::EntityType::Circle(_) => "Circle",
            acadrust::entities::EntityType::Arc(_) => "Arc",
            acadrust::entities::EntityType::Ellipse(_) => "Ellipse",
            acadrust::entities::EntityType::Text(_) => "Text",
            acadrust::entities::EntityType::MText(_) => "MText",
            acadrust::entities::EntityType::Point(_) => "Point",
            acadrust::entities::EntityType::Solid(_) => "Solid",
            acadrust::entities::EntityType::Face3D(_) => "Face3D",
            acadrust::entities::EntityType::Insert(_) => "Insert",
            acadrust::entities::EntityType::LwPolyline(_) => "LwPolyline",
            acadrust::entities::EntityType::Spline(_) => "Spline",
            acadrust::entities::EntityType::Hatch(_) => "Hatch",
            acadrust::entities::EntityType::Viewport(_) => "Viewport",
            acadrust::entities::EntityType::Dimension(_) => "Dimension",
            acadrust::entities::EntityType::Block(_) => "Block",
            acadrust::entities::EntityType::BlockEnd(_) => "BlockEnd",
            acadrust::entities::EntityType::Seqend(_) => "Seqend",
            acadrust::entities::EntityType::Polyline2D(_) => "Polyline2D",
            acadrust::entities::EntityType::Polyline3D(_) => "Polyline3D",
            acadrust::entities::EntityType::PolyfaceMesh(_) => "PolyfaceMesh",
            acadrust::entities::EntityType::PolygonMesh(_) => "PolygonMesh",
            acadrust::entities::EntityType::Solid3D(_) => "Solid3D",
            acadrust::entities::EntityType::Region(_) => "Region",
            acadrust::entities::EntityType::Body(_) => "Body",
            acadrust::entities::EntityType::Leader(_) => "Leader",
            acadrust::entities::EntityType::Tolerance(_) => "Tolerance",
            acadrust::entities::EntityType::Ray(_) => "Ray",
            acadrust::entities::EntityType::XLine(_) => "XLine",
            acadrust::entities::EntityType::Shape(_) => "Shape",
            acadrust::entities::EntityType::MultiLeader(_) => "MultiLeader",
            acadrust::entities::EntityType::Mesh(_) => "Mesh",
            acadrust::entities::EntityType::MLine(_) => "MLine",
            acadrust::entities::EntityType::RasterImage(_) => "RasterImage",
            acadrust::entities::EntityType::Wipeout(_) => "Wipeout",
            acadrust::entities::EntityType::Ole2Frame(_) => "Ole2Frame",
            acadrust::entities::EntityType::AttributeDefinition(_) => "AttDef",
            acadrust::entities::EntityType::AttributeEntity(_) => "Attrib",
            acadrust::entities::EntityType::Polyline(_) => "Polyline(old)",
            acadrust::entities::EntityType::Table(_) => "Table",
            acadrust::entities::EntityType::Underlay(_) => "Underlay",
            acadrust::entities::EntityType::Unknown(u) => {
                eprintln!("  Unknown entity: {} (DWG type: {})", u.dxf_name, u.dwg_type_code);
                "Unknown"
            }
        };
        *type_counts.entry(name.to_string()).or_default() += 1;
    }
    eprintln!("\n=== Entity Type Breakdown ===");
    let mut sorted: Vec<_> = type_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in sorted {
        eprintln!("  {}: {}", name, count);
    }

    // Count object types
    let mut obj_type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, obj) in &doc.objects {
        let name = match obj {
            acadrust::objects::ObjectType::Dictionary(_) => "Dictionary",
            acadrust::objects::ObjectType::Layout(_) => "Layout",
            acadrust::objects::ObjectType::XRecord(_) => "XRecord",
            acadrust::objects::ObjectType::Group(_) => "Group",
            acadrust::objects::ObjectType::MLineStyle(_) => "MLineStyle",
            acadrust::objects::ObjectType::MultiLeaderStyle(_) => "MLeaderStyle",
            acadrust::objects::ObjectType::ImageDefinition(_) => "ImageDef",
            acadrust::objects::ObjectType::ImageDefinitionReactor(_) => "ImageDefReactor",
            acadrust::objects::ObjectType::PlotSettings(_) => "PlotSettings",
            acadrust::objects::ObjectType::Scale(_) => "Scale",
            acadrust::objects::ObjectType::SortEntitiesTable(_) => "SortEntsTable",
            acadrust::objects::ObjectType::DictionaryVariable(_) => "DictVar",
            acadrust::objects::ObjectType::RasterVariables(_) => "RasterVars",
            acadrust::objects::ObjectType::DictionaryWithDefault(_) => "DictWithDefault",
            acadrust::objects::ObjectType::PlaceHolder(_) => "PlaceHolder",
            acadrust::objects::ObjectType::BookColor(_) => "BookColor",
            acadrust::objects::ObjectType::WipeoutVariables(_) => "WipeoutVars",
            acadrust::objects::ObjectType::GeoData(_) => "GeoData",
            acadrust::objects::ObjectType::SpatialFilter(_) => "SpatialFilter",
            acadrust::objects::ObjectType::VisualStyle(_) => "VisualStyle",
            acadrust::objects::ObjectType::Material(_) => "Material",
            acadrust::objects::ObjectType::TableStyle(_) => "TableStyle",
            acadrust::objects::ObjectType::Unknown { .. } => "Unknown",
        };
        *obj_type_counts.entry(name.to_string()).or_default() += 1;
    }
    eprintln!("\n=== Object Type Breakdown ===");
    let mut sorted: Vec<_> = obj_type_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in sorted {
        eprintln!("  {}: {}", name, count);
    }

    // Count total entity handles across all block records
    let mut total_entity_handles = 0usize;
    for br in doc.block_records.iter() {
        total_entity_handles += br.entity_handles.len();
    }
    eprintln!("\n=== Block Record Entity Handles ===");
    eprintln!("Total entity_handles across all blocks: {}", total_entity_handles);
    
    // Count how many entity handles can't be resolved
    let mut unresolved = 0;
    for br in doc.block_records.iter() {
        for eh in &br.entity_handles {
            if doc.get_entity(*eh).is_none() {
                unresolved += 1;
            }
        }
    }
    eprintln!("Unresolved entity handles: {}", unresolved);

    // Count sub-entities in polyface meshes and inserts
    let mut total_pface_verts = 0usize;
    let mut total_pface_faces = 0usize;
    let mut total_insert_attrs = 0usize;
    let mut total_poly2d_verts = 0usize;
    let mut total_poly3d_verts = 0usize;
    let mut total_polymesh_verts = 0usize;
    for e in doc.entities() {
        match e {
            acadrust::entities::EntityType::PolyfaceMesh(pf) => {
                total_pface_verts += pf.vertices.len();
                total_pface_faces += pf.faces.len();
            }
            acadrust::entities::EntityType::Insert(ins) => {
                total_insert_attrs += ins.attributes.len();
            }
            acadrust::entities::EntityType::Polyline2D(p) => {
                total_poly2d_verts += p.vertices.len();
            }
            acadrust::entities::EntityType::Polyline3D(p) => {
                total_poly3d_verts += p.vertices.len();
            }
            acadrust::entities::EntityType::PolygonMesh(p) => {
                total_polymesh_verts += p.vertices.len();
            }
            _ => {}
        }
    }
    eprintln!("\n=== Sub-entity Counts ===");
    eprintln!("PolyfaceMesh vertices: {}", total_pface_verts);
    eprintln!("PolyfaceMesh faces: {}", total_pface_faces);
    eprintln!("Insert attributes: {}", total_insert_attrs);
    eprintln!("Polyline2D vertices: {}", total_poly2d_verts);
    eprintln!("Polyline3D vertices: {}", total_poly3d_verts);
    eprintln!("PolygonMesh vertices: {}", total_polymesh_verts);
    
    // Estimate total object records the writer should produce
    let table_controls = 9; // block, layer, style, ltype, view, ucs, vport, appid, dimstyle
    let table_entries = doc.layers.len() + doc.text_styles.len() + doc.line_types.len()
        + doc.dim_styles.len() + doc.views.len() + doc.ucss.len() + doc.vports.len()
        + doc.app_ids.len() + doc.block_records.len();
    let block_begin_end = doc.block_records.len() * 2; // BLOCK + ENDBLK per record
    let seqend_polyface = 239; // one SEQEND per polyface mesh
    let sub_entities = total_pface_verts + total_pface_faces + seqend_polyface
        + total_insert_attrs + total_poly2d_verts + total_poly3d_verts + total_polymesh_verts;
    let objects = doc.objects.len();
    let main_entities = doc.entities().count();
    
    let est_total = table_controls + table_entries + block_begin_end + main_entities + sub_entities + objects;
    eprintln!("\n=== Estimated Total Object Records ===");
    eprintln!("Table controls: {}", table_controls);
    eprintln!("Table entries: {}", table_entries);
    eprintln!("Block begin/end: {}", block_begin_end);
    eprintln!("Main entities: {}", main_entities);
    eprintln!("Sub-entities (verts+faces+seqend+attrs): {}", sub_entities);
    eprintln!("Non-graphical objects: {}", objects);
    eprintln!("ESTIMATED TOTAL: {}", est_total);

    // Now perform a write and check handle map
    eprintln!("\n=== DWG Write Statistics ===");
    let mut buf = Cursor::new(Vec::new());
    match DwgWriter::write_to_writer(&mut buf, &doc) {
        Ok(()) => {
            let written = buf.into_inner();
            eprintln!("Written file size: {} bytes", written.len());
        }
        Err(e) => {
            eprintln!("Write error: {}", e);
        }
    }
}
