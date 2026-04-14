use acadrust::io::dwg::DwgReader;
use std::collections::HashMap;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });
    let mut reader = DwgReader::from_file(&path).expect("Failed to open DWG");
    let doc = reader.read().expect("Failed to read DWG");

    println!("=== Document Stats for: {} ===", path);
    println!("Entities: {}", doc.entities().count());
    println!("Objects: {}", doc.objects.len());
    println!("Block records: {}", doc.block_records.iter().count());
    println!("Layers: {}", doc.layers.iter().count());
    println!("Handle seed: {:#X}", doc.header.handle_seed);

    // Count entity types by variant name
    let mut entity_counts: HashMap<&str, usize> = HashMap::new();
    for e in doc.entities() {
        let name = match e {
            acadrust::entities::EntityType::Point(_) => "Point",
            acadrust::entities::EntityType::Line(_) => "Line",
            acadrust::entities::EntityType::Circle(_) => "Circle",
            acadrust::entities::EntityType::Arc(_) => "Arc",
            acadrust::entities::EntityType::Ellipse(_) => "Ellipse",
            acadrust::entities::EntityType::Text(_) => "Text",
            acadrust::entities::EntityType::MText(_) => "MText",
            acadrust::entities::EntityType::Solid(_) => "Solid",
            acadrust::entities::EntityType::Face3D(_) => "Face3D",
            acadrust::entities::EntityType::Insert(_) => "Insert",
            acadrust::entities::EntityType::LwPolyline(_) => "LwPolyline",
            acadrust::entities::EntityType::Spline(_) => "Spline",
            acadrust::entities::EntityType::Hatch(_) => "Hatch",
            acadrust::entities::EntityType::Viewport(_) => "Viewport",
            acadrust::entities::EntityType::Dimension(_) => "Dimension",
            acadrust::entities::EntityType::Polyline2D(_) => "Polyline2D",
            acadrust::entities::EntityType::Polyline3D(_) => "Polyline3D",
            acadrust::entities::EntityType::PolyfaceMesh(_) => "PolyfaceMesh",
            acadrust::entities::EntityType::PolygonMesh(_) => "PolygonMesh",
            acadrust::entities::EntityType::Seqend(_) => "Seqend",
            acadrust::entities::EntityType::Block(_) => "Block",
            acadrust::entities::EntityType::BlockEnd(_) => "BlockEnd",
            acadrust::entities::EntityType::Solid3D(_) => "Solid3D",
            acadrust::entities::EntityType::Region(_) => "Region",
            acadrust::entities::EntityType::Body(_) => "Body",
            acadrust::entities::EntityType::Mesh(_) => "Mesh",
            acadrust::entities::EntityType::MLine(_) => "MLine",
            acadrust::entities::EntityType::Leader(_) => "Leader",
            acadrust::entities::EntityType::MultiLeader(_) => "MultiLeader",
            acadrust::entities::EntityType::RasterImage(_) => "RasterImage",
            acadrust::entities::EntityType::Wipeout(_) => "Wipeout",
            acadrust::entities::EntityType::Ole2Frame(_) => "Ole2Frame",
            acadrust::entities::EntityType::Table(_) => "Table",
            acadrust::entities::EntityType::Underlay(_) => "Underlay",
            acadrust::entities::EntityType::Tolerance(_) => "Tolerance",
            acadrust::entities::EntityType::Shape(_) => "Shape",
            acadrust::entities::EntityType::Ray(_) => "Ray",
            acadrust::entities::EntityType::XLine(_) => "XLine",
            acadrust::entities::EntityType::AttributeDefinition(_) => "AttDef",
            acadrust::entities::EntityType::AttributeEntity(_) => "AttEntity",
            acadrust::entities::EntityType::Polyline(_) => "Polyline(old)",
            acadrust::entities::EntityType::Unknown(_) => "Unknown",
        };
        *entity_counts.entry(name).or_default() += 1;
    }
    println!("\n=== Entity Types ===");
    let mut counts: Vec<_> = entity_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in &counts {
        println!("  {:20} {}", name, count);
    }

    // Count object types
    let mut object_counts: HashMap<&str, usize> = HashMap::new();
    for (_, obj) in doc.objects.iter() {
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
            acadrust::objects::ObjectType::SortEntitiesTable(_) => "SortEntTable",
            acadrust::objects::ObjectType::DictionaryVariable(_) => "DictVariable",
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
        *object_counts.entry(name).or_default() += 1;
    }
    println!("\n=== Object Types ===");
    let mut counts: Vec<_> = object_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1));
    let mut total = 0;
    for (name, count) in &counts {
        println!("  {:20} {}", name, count);
        total += *count;
    }
    println!("  {:20} {}", "TOTAL", total);

    // Count skipped (un-writable) objects
    let skipped: usize = doc.objects.iter()
        .filter(|(_, obj)| matches!(obj,
            acadrust::objects::ObjectType::GeoData(_)
            | acadrust::objects::ObjectType::SpatialFilter(_)
            | acadrust::objects::ObjectType::VisualStyle(_)
            | acadrust::objects::ObjectType::Material(_)
            | acadrust::objects::ObjectType::TableStyle(_)
            | acadrust::objects::ObjectType::Unknown { .. }
        ))
        .count();
    println!("\nSkipped (un-writable) objects: {}", skipped);
}
