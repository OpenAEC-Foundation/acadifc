use acadrust::DwgReader;
use acadrust::entities::EntityType;
use std::collections::HashSet;

fn main() {
    let input_path = "tests/issue14/General.dwg";
    let mut reader = DwgReader::from_file(input_path).expect("open");
    let doc = reader.read().expect("read");

    let failing: HashSet<u64> = [
        0x241B6, 0x241B9, 0x241BF, 0x241CE, 0x241E6, 0x2420D, 0x24211,
        0x24217, 0x2421C, 0x2421F, 0x24223, 0x24226, 0x2422E, 0x24237,
        0x2423A, 0x2423C, 0x2423E, 0x24242, 0x24248, 0x2424B, 0x2424E,
        0x24251, 0x2425D, 0x2425F, 0x24266, 0x24270, 0x2427C, 0x2427F,
        0x24285, 0x24288, 0x24295, 0x24298, 0x2429B, 0x2429E, 0x242A1,
        0x242AC, 0x242AF, 0x242B1, 0x242B6, 0x242BC, 0x242EA, 0x242F1,
        0x24302, 0x24303, 0x24304, 0x24305, 0x2430E, 0x2430F, 0x24312,
        0x24315, 0x24318, 0x2431B, 0x2431E, 0x24321, 0x24324, 0x24327,
        0x2432A, 0x2432D, 0x24330, 0x24333, 0x24336, 0x24339, 0x2433C,
        0x2433F, 0x24342, 0x24345,
    ].iter().copied().collect();

    // Collect ALL entities with handles >= 0x24000, sorted
    let mut high_entities: Vec<(u64, &str, bool)> = Vec::new();
    for entity in doc.entities() {
        let h = entity.common().handle.value();
        if h >= 0x24000 {
            let type_name = match entity {
                EntityType::LwPolyline(_) => "LwPolyline",
                EntityType::MText(_) => "MText",
                EntityType::Line(_) => "Line",
                EntityType::Arc(_) => "Arc",
                EntityType::Hatch(_) => "Hatch",
                EntityType::Spline(_) => "Spline",
                EntityType::Insert(_) => "Insert",
                _ => "Other",
            };
            let is_fail = matches!(entity, EntityType::LwPolyline(_)) && failing.contains(&h);
            high_entities.push((h, type_name, is_fail));
        }
    }
    high_entities.sort_by_key(|&(h, _, _)| h);

    println!("=== Entities with handle >= 0x24000 ({} total) ===", high_entities.len());
    for &(h, tn, fail) in &high_entities {
        let marker = if fail { " *** FAIL" } else { "" };
        println!("  {:#X}: {}{}", h, tn, marker);
    }

    // Also show the HIGHEST passing LwPolyline handle
    let mut max_passing_lw = 0u64;
    let mut min_failing_lw = u64::MAX;
    for entity in doc.entities() {
        if let EntityType::LwPolyline(_) = entity {
            let h = entity.common().handle.value();
            if failing.contains(&h) {
                if h < min_failing_lw { min_failing_lw = h; }
            } else {
                if h > max_passing_lw { max_passing_lw = h; }
            }
        }
    }
    println!("\nHighest passing LwPolyline: {:#X}", max_passing_lw);
    println!("Lowest failing LwPolyline: {:#X}", min_failing_lw);
    
    // Check if any LwPolylines BETWEEN min_failing and max_failing PASS
    let max_failing_lw = *failing.iter().max().unwrap();
    let pass_in_range: Vec<u64> = doc.entities().filter_map(|e| {
        if let EntityType::LwPolyline(_) = e {
            let h = e.common().handle.value();
            if h >= min_failing_lw && h <= max_failing_lw && !failing.contains(&h) {
                return Some(h);
            }
        }
        None
    }).collect();
    println!("Passing LwPolylines in failing range [{:#X}..{:#X}]: {:?}", 
        min_failing_lw, max_failing_lw, pass_in_range);
}
