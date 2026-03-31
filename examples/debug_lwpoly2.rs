use acadrust::DwgReader;
use acadrust::entities::EntityType;
use std::collections::HashSet;

fn main() {
    let input_path = "tests/issue14/General.dwg";
    let mut reader = DwgReader::from_file(input_path).expect("open");
    let doc = reader.read().expect("read");

    // Failing handles from BricsCAD RECOVER+AUDIT
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

    // Collect all LwPolylines
    let mut fail_props = Vec::new();
    let mut pass_props = Vec::new();

    for entity in doc.entities() {
        if let EntityType::LwPolyline(lw) = entity {
            let c = entity.common();
            let h = c.handle.value();
            let is_fail = failing.contains(&h);

            let has_bulges = lw.vertices.iter().any(|v| v.bulge != 0.0);
            let has_widths = lw.vertices.iter().any(|v| v.start_width != 0.0 || v.end_width != 0.0);
            let has_normal = lw.normal.x != 0.0 || lw.normal.y != 0.0 || (lw.normal.z - 1.0).abs() > 1e-10;

            let props = format!(
                "h={:#X} fail={} verts={} closed={} cw={} elev={} thick={} normal=({:.3},{:.3},{:.3}) bulge={} widths={} color={:?} transp={:?} layer='{}' lw={:?} lt_scale={} invis={} react={} xdict={:?} xdata_empty={}",
                h, is_fail, lw.vertices.len(), lw.is_closed, lw.constant_width,
                lw.elevation, lw.thickness, lw.normal.x, lw.normal.y, lw.normal.z,
                has_bulges, has_widths, c.color, c.transparency,
                c.layer, c.line_weight, c.linetype_scale, c.invisible,
                c.reactors.len(), c.xdictionary_handle, c.extended_data.is_empty()
            );

            if is_fail {
                fail_props.push(props);
            } else {
                pass_props.push(props);
            }
        }
    }

    println!("=== FAILING LwPolylines ({}) ===", fail_props.len());
    for p in &fail_props {
        println!("{}", p);
    }

    println!("\n=== PASSING LwPolylines (first 20 of {}) ===", pass_props.len());
    for p in pass_props.iter().take(20) {
        println!("{}", p);
    }

    // Summary statistics
    println!("\n=== SUMMARY ===");
    println!("Total failing: {}", fail_props.len());
    println!("Total passing: {}", pass_props.len());
}
