/// Count all object and entity types in a DWG document.
/// Usage: cargo run --example diag_object_counts -- path/to/file.dwg

use acadrust::io::dwg::DwgReader;
use std::collections::HashMap;
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });

    let bytes = std::fs::read(&path).expect("Failed to read file");
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let doc = reader.read().expect("Failed to read DWG");
    
    // Get notifications for handle/object info
    for n in doc.notifications.iter() {
        println!("[NOTIFICATION] {:?}: {}", n.notification_type, n.message);
    }

    // Count entity types
    let mut entity_counts: HashMap<&str, usize> = HashMap::new();
    let mut polyface_vertx_total = 0usize;
    let mut polyface_face_total = 0usize;
    let mut insert_attrib_total = 0usize;
    for entity in doc.entities() {
        let name = entity_type_name(entity);
        *entity_counts.entry(name).or_default() += 1;
        if let acadrust::entities::EntityType::PolyfaceMesh(e) = entity {
            polyface_vertx_total += e.vertices.len();
            polyface_face_total += e.faces.len();
        }
        if let acadrust::entities::EntityType::Insert(e) = entity {
            insert_attrib_total += e.attributes.len();
        }
    }

    // Count object types
    let mut object_counts: HashMap<String, usize> = HashMap::new();
    for (_, obj) in &doc.objects {
        let name = object_type_name(obj);
        *object_counts.entry(name).or_default() += 1;
    }

    // Count table entries
    let table_counts = vec![
        ("BlockRecords", doc.block_records.len()),
        ("Layers", doc.layers.len()),
        ("TextStyles", doc.text_styles.len()),
        ("LineTypes", doc.line_types.len()),
        ("Views", doc.views.len()),
        ("UCSs", doc.ucss.len()),
        ("VPorts", doc.vports.len()),
        ("AppIds", doc.app_ids.len()),
        ("DimStyles", doc.dim_styles.len()),
    ];

    let total_entities: usize = entity_counts.values().sum();
    let total_objects: usize = object_counts.values().sum();
    let total_table_entries: usize = table_counts.iter().map(|(_, c)| c).sum();

    println!("=== Object Type Counts: {} ===", path);
    println!("\n--- Entities ({} total) ---", total_entities);
    let mut ec: Vec<_> = entity_counts.iter().collect();
    ec.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in &ec {
        println!("  {:30} {}", name, count);
    }

    println!("\n--- Objects ({} total) ---", total_objects);
    let mut oc: Vec<_> = object_counts.iter().collect();
    oc.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in &oc {
        println!("  {:30} {}", name, count);
    }

    println!("\n--- Table Entries ({} total) ---", total_table_entries);
    for (name, count) in &table_counts {
        println!("  {:30} {}", name, count);
    }

    // Table controls: 9 (block, layer, style, ltype, view, ucs, vport, appid, dimstyle)
    let table_controls = 9;
    // Sub-entities: each PolyfaceMesh has N vertices + N faces + 1 SEQEND 
    // Each Insert with attribs has N attribs + 1 SEQEND
    let polyface_seqends = entity_counts.get("PolyfaceMesh").copied().unwrap_or(0);
    let polyface_subs = polyface_vertx_total + polyface_face_total + polyface_seqends;
    let insert_seqends = if insert_attrib_total > 0 { 
        entity_counts.get("Insert").copied().unwrap_or(0) 
    } else { 0 };
    let insert_subs = insert_attrib_total + insert_seqends;
    // Block entities: each BlockRecord also has a BlockBegin and BlockEnd in the DWG
    let block_markers = table_counts.iter().find(|(n,_)| *n == "BlockRecords").map(|(_,c)| c * 2).unwrap_or(0);
    
    let total_written = total_entities + total_objects + total_table_entries 
        + table_controls + polyface_subs + insert_subs + block_markers;
    println!("\n--- Summary ---");
    println!("  Total entities:      {}", total_entities);
    println!("  Total objects:       {}", total_objects);
    println!("  Table entries:       {}", total_table_entries);
    println!("  Table controls:      {}", table_controls);
    println!("  PolyfaceMesh subs:   {} ({} vertices + {} faces + {} seqend)", 
        polyface_subs, polyface_vertx_total, polyface_face_total, polyface_seqends);
    println!("  Insert subs:         {} ({} attribs + {} seqend)",
        insert_subs, insert_attrib_total, insert_seqends);
    println!("  Block markers:       {} ({} BlockRecords × 2)",
        block_markers, table_counts.iter().find(|(n,_)| *n == "BlockRecords").map(|(_,c)| *c).unwrap_or(0));
    println!("  Grand total written: {}", total_written);

    // Perform roundtrip write and check output size
    {
        use acadrust::io::dwg::DwgWriter;
        use std::io::Cursor;
        let mut buf = Cursor::new(Vec::new());
        DwgWriter::write_to_writer(&mut buf, &doc).expect("Failed to write DWG");
        let written_bytes = buf.into_inner();
        println!("\n--- Roundtrip Write ---");
        println!("  Written file size:   {} bytes", written_bytes.len());
        println!("  Original file size:  {} bytes", bytes.len());

        // Read back and count
        let mut reader2 = DwgReader::from_stream(Cursor::new(&written_bytes));
        let doc2 = reader2.read().expect("Failed to read roundtripped DWG");
        let mut rt_entity_count = 0usize;
        let mut rt_object_count = doc2.objects.len();
        for _ in doc2.entities() {
            rt_entity_count += 1;
        }
        println!("  RT entities:         {}", rt_entity_count);
        println!("  RT objects:          {}", rt_object_count);
    }
    
    // Count classes
    println!("\n--- Classes ({}) ---", doc.classes.len());
    for cls in doc.classes.iter() {
        println!("  {} (number={})", cls.dxf_name, cls.class_number);
    }
}

fn entity_type_name(e: &acadrust::entities::EntityType) -> &'static str {
    match e {
        acadrust::entities::EntityType::Line(_) => "Line",
        acadrust::entities::EntityType::Circle(_) => "Circle",
        acadrust::entities::EntityType::Arc(_) => "Arc",
        acadrust::entities::EntityType::Point(_) => "Point",
        acadrust::entities::EntityType::Solid3D(_) => "Solid3D",
        acadrust::entities::EntityType::Insert(_) => "Insert",
        acadrust::entities::EntityType::MText(_) => "MText",
        acadrust::entities::EntityType::Text(_) => "Text",
        acadrust::entities::EntityType::LwPolyline(_) => "LwPolyline",
        acadrust::entities::EntityType::Polyline(_) => "Polyline",
        acadrust::entities::EntityType::Polyline3D(_) => "Polyline3D",
        acadrust::entities::EntityType::PolyfaceMesh(_) => "PolyfaceMesh",
        acadrust::entities::EntityType::PolygonMesh(_) => "PolygonMesh",
        acadrust::entities::EntityType::Spline(_) => "Spline",
        acadrust::entities::EntityType::Ellipse(_) => "Ellipse",
        acadrust::entities::EntityType::Dimension(_) => "Dimension",
        acadrust::entities::EntityType::Block(_) => "Block",
        acadrust::entities::EntityType::Seqend(_) => "Seqend",
        acadrust::entities::EntityType::Hatch(_) => "Hatch",
        acadrust::entities::EntityType::Leader(_) => "Leader",
        acadrust::entities::EntityType::Viewport(_) => "Viewport",
        acadrust::entities::EntityType::Face3D(_) => "Face3D",
        acadrust::entities::EntityType::Solid(_) => "Solid",
        acadrust::entities::EntityType::Ray(_) => "Ray",
        acadrust::entities::EntityType::XLine(_) => "XLine",
        acadrust::entities::EntityType::MultiLeader(_) => "MultiLeader",
        acadrust::entities::EntityType::RasterImage(_) => "RasterImage",
        acadrust::entities::EntityType::Shape(_) => "Shape",
        acadrust::entities::EntityType::Table(_) => "Table",
        acadrust::entities::EntityType::Tolerance(_) => "Tolerance",
        acadrust::entities::EntityType::AttributeDefinition(_) => "AttDef",
        acadrust::entities::EntityType::AttributeEntity(_) => "AttEntity",
        acadrust::entities::EntityType::Wipeout(_) => "Wipeout",
        acadrust::entities::EntityType::Ole2Frame(_) => "Ole2Frame",
        acadrust::entities::EntityType::Underlay(_) => "Underlay",
        acadrust::entities::EntityType::Mesh(_) => "Mesh",
        acadrust::entities::EntityType::MLine(_) => "MLine",
        acadrust::entities::EntityType::Region(_) => "Region",
        acadrust::entities::EntityType::Body(_) => "Body",
        _ => "Unknown",
    }
}

fn object_type_name(obj: &acadrust::objects::ObjectType) -> String {
    match obj {
        acadrust::objects::ObjectType::Dictionary(_) => "Dictionary".into(),
        acadrust::objects::ObjectType::Layout(_) => "Layout".into(),
        acadrust::objects::ObjectType::XRecord(_) => "XRecord".into(),
        acadrust::objects::ObjectType::Group(_) => "Group".into(),
        acadrust::objects::ObjectType::MLineStyle(_) => "MLineStyle".into(),
        acadrust::objects::ObjectType::MultiLeaderStyle(_) => "MultiLeaderStyle".into(),
        acadrust::objects::ObjectType::ImageDefinition(_) => "ImageDefinition".into(),
        acadrust::objects::ObjectType::ImageDefinitionReactor(_) => "ImageDefReactor".into(),
        acadrust::objects::ObjectType::PlotSettings(_) => "PlotSettings".into(),
        acadrust::objects::ObjectType::Scale(_) => "Scale".into(),
        acadrust::objects::ObjectType::SortEntitiesTable(_) => "SortEntitiesTable".into(),
        acadrust::objects::ObjectType::DictionaryVariable(_) => "DictionaryVariable".into(),
        acadrust::objects::ObjectType::RasterVariables(_) => "RasterVariables".into(),
        acadrust::objects::ObjectType::DictionaryWithDefault(_) => "DictWithDefault".into(),
        acadrust::objects::ObjectType::PlaceHolder(_) => "PlaceHolder".into(),
        acadrust::objects::ObjectType::BookColor(_) => "BookColor".into(),
        acadrust::objects::ObjectType::WipeoutVariables(_) => "WipeoutVariables".into(),
        acadrust::objects::ObjectType::GeoData(_) => "GeoData [SKIP]".into(),
        acadrust::objects::ObjectType::SpatialFilter(_) => "SpatialFilter [SKIP]".into(),
        acadrust::objects::ObjectType::VisualStyle(_) => "VisualStyle [SKIP]".into(),
        acadrust::objects::ObjectType::Material(_) => "Material [SKIP]".into(),
        acadrust::objects::ObjectType::TableStyle(_) => "TableStyle [SKIP]".into(),
        acadrust::objects::ObjectType::Unknown { .. } => "Unknown [SKIP]".into(),
    }
}
