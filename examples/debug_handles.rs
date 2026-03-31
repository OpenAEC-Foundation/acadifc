use acadrust::DwgReader;
use acadrust::types::Handle;

fn main() {
    let input_path = "tests/issue14/General.dwg";
    let mut reader = DwgReader::from_file(input_path).expect("Failed to open");
    let doc = reader.read().expect("Failed to read");

    // Check header handles
    println!("=== HEADER HANDLES ===");
    println!("CTABLESTYLE:      {:?}", doc.header.tablestyle_handle);
    println!("CMLEADERSTYLE:    {:?}", doc.header.mleaderstyle_handle);
    println!("CVIEWDETAILSTYLE: {:?}", doc.header.viewdetailstyle_handle);
    println!("CVIEWSECTIONSTYLE:{:?}", doc.header.viewsectionstyle_handle);

    // Check what's at specific handles from BricsCAD errors
    let check = [0x66u64, 0x89, 0x1B7, 0x1B8, 0xD7, 0xD8, 0xE5];
    println!("\n=== OBJECT TYPES AT FAILING HANDLES ===");
    for h in &check {
        let handle = Handle::from(*h);
        match doc.objects.get(&handle) {
            Some(obj) => println!("Handle {:#X}: {:?}", h, obj_type_name(obj)),
            None => println!("Handle {:#X}: NOT IN objects map", h),
        }
    }

    // Also print ALL MLeaderStyle objects
    println!("\n=== ALL MLEADERSTYLE OBJECTS ===");
    for (handle, obj) in &doc.objects {
        if let acadrust::objects::ObjectType::MultiLeaderStyle(s) = obj {
            println!("  Handle {:?}: name='{}' owner={:?} content_type={} property_changed={}", 
                handle, s.description, s.owner_handle, s.content_type as i16, s.property_changed);
        }
    }

    // Print ALL Dictionary entries for dict at 0x66 and 0xD7
    for dict_handle in [0x66u64, 0xD7] {
        let handle = Handle::from(dict_handle);
        if let Some(acadrust::objects::ObjectType::Dictionary(d)) = doc.objects.get(&handle) {
            println!("\n=== DICTIONARY {:#X} ENTRIES ===", dict_handle);
            for (name, entry_handle) in &d.entries {
                let obj_info = match doc.objects.get(entry_handle) {
                    Some(obj) => obj_type_name(obj).to_string(),
                    None => "NOT FOUND".to_string(),
                };
                println!("  '{}' -> {:?} ({})", name, entry_handle, obj_info);
            }
        }
    }
}

fn obj_type_name(obj: &acadrust::objects::ObjectType) -> &'static str {
    match obj {
        acadrust::objects::ObjectType::Dictionary(_) => "Dictionary",
        acadrust::objects::ObjectType::DictionaryWithDefault(_) => "DictionaryWithDefault",
        acadrust::objects::ObjectType::DictionaryVariable(_) => "DictionaryVariable",
        acadrust::objects::ObjectType::Layout(_) => "Layout",
        acadrust::objects::ObjectType::XRecord(_) => "XRecord",
        acadrust::objects::ObjectType::Group(_) => "Group",
        acadrust::objects::ObjectType::MLineStyle(_) => "MLineStyle",
        acadrust::objects::ObjectType::MultiLeaderStyle(_) => "MultiLeaderStyle",
        acadrust::objects::ObjectType::ImageDefinition(_) => "ImageDefinition",
        acadrust::objects::ObjectType::ImageDefinitionReactor(_) => "ImageDefinitionReactor",
        acadrust::objects::ObjectType::PlotSettings(_) => "PlotSettings",
        acadrust::objects::ObjectType::Scale(_) => "Scale",
        acadrust::objects::ObjectType::SortEntitiesTable(_) => "SortEntitiesTable",
        acadrust::objects::ObjectType::RasterVariables(_) => "RasterVariables",
        acadrust::objects::ObjectType::PlaceHolder(_) => "PlaceHolder",
        acadrust::objects::ObjectType::BookColor(_) => "BookColor",
        acadrust::objects::ObjectType::WipeoutVariables(_) => "WipeoutVariables",
        acadrust::objects::ObjectType::GeoData(_) => "GeoData",
        acadrust::objects::ObjectType::SpatialFilter(_) => "SpatialFilter",
        acadrust::objects::ObjectType::VisualStyle(_) => "VisualStyle",
        acadrust::objects::ObjectType::Material(_) => "Material",
        acadrust::objects::ObjectType::TableStyle(_) => "TableStyle",
        acadrust::objects::ObjectType::Unknown { .. } => "Unknown",
    }
}
