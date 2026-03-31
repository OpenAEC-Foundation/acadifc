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

    let mut pass_closed = 0;
    let mut pass_open = 0;
    let mut fail_closed = 0;
    let mut fail_open = 0;
    let mut pass_bylayer = 0;
    let mut pass_rgb = 0;
    let mut fail_bylayer = 0;
    
    // Also track layers of failing vs passing
    let mut fail_layers: std::collections::HashMap<String, usize> = Default::default();
    let mut pass_layers: std::collections::HashMap<String, usize> = Default::default();
    
    // Track constant_width
    let mut pass_has_cw = 0;
    let mut fail_has_cw = 0;
    
    for entity in doc.entities() {
        if let EntityType::LwPolyline(lw) = entity {
            let h = entity.common().handle.value();
            let is_fail = failing.contains(&h);
            
            if is_fail {
                if lw.is_closed { fail_closed += 1; } else { fail_open += 1; }
                if matches!(entity.common().color, acadrust::types::Color::ByLayer) { fail_bylayer += 1; }
                if lw.constant_width != 0.0 { fail_has_cw += 1; }
                *fail_layers.entry(entity.common().layer.clone()).or_default() += 1;
            } else {
                if lw.is_closed { pass_closed += 1; } else { pass_open += 1; }
                if matches!(entity.common().color, acadrust::types::Color::ByLayer) { pass_bylayer += 1; }
                if matches!(entity.common().color, acadrust::types::Color::Rgb { .. }) { pass_rgb += 1; }
                if lw.constant_width != 0.0 { pass_has_cw += 1; }
                *pass_layers.entry(entity.common().layer.clone()).or_default() += 1;
            }
        }
    }
    
    println!("PASSING: closed={} open={} bylayer={} rgb={} has_cw={}", 
        pass_closed, pass_open, pass_bylayer, pass_rgb, pass_has_cw);
    println!("FAILING: closed={} open={} bylayer={} has_cw={}", 
        fail_closed, fail_open, fail_bylayer, fail_has_cw);
    
    println!("\nPassing layers:");
    let mut pass_vec: Vec<_> = pass_layers.into_iter().collect();
    pass_vec.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (l, c) in &pass_vec { println!("  {}: {}", l, c); }
    
    println!("\nFailing layers:");
    let mut fail_vec: Vec<_> = fail_layers.into_iter().collect();
    fail_vec.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (l, c) in &fail_vec { println!("  {}: {}", l, c); }
}
