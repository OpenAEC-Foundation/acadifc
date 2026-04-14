/// Compare parsed LINE entity fields between original and roundtripped DWG.
///
/// For each LINE handle, parse the entity from both files and compare
/// the raw field values (thickness, normal, coordinates, etc.)
/// to identify which field causes the byte-level difference.

use acadrust::io::dwg::dwg_stream_readers::object_reader::DwgObjectReader;
use acadrust::io::dwg::dwg_stream_readers::object_reader::entities;
use acadrust::DwgReader;
use std::collections::HashMap;

fn load_reader(path: &str) -> Option<(DwgObjectReader, acadrust::types::DxfVersion)> {
    let mut reader = DwgReader::from_file(path).ok()?;
    let info = reader.read_file_header().ok()?;
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).ok()?;
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf).ok()?;
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).ok()?;
    let dxf_version = acadrust::types::DxfVersion::from_version_string(&info.version_string);
    let obj_reader = DwgObjectReader::new(objects_buf, dxf_version, handle_map).ok()?;
    Some((obj_reader, dxf_version))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: diag_line_fields <original.dwg> <roundtrip.dwg> [max_show]");
        std::process::exit(2);
    }
    let orig_path = &args[1];
    let rt_path = &args[2];
    let max_show: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(5);

    let (orig_reader, _orig_ver) = load_reader(orig_path).expect("Failed to load original");
    let (rt_reader, _rt_ver) = load_reader(rt_path).expect("Failed to load RT");

    // Find common handles
    let orig_handles: Vec<u64> = orig_reader.handles();
    let rt_handle_set: std::collections::HashSet<u64> = rt_reader.handles().into_iter().collect();
    let mut common: Vec<u64> = orig_handles.into_iter()
        .filter(|h| rt_handle_set.contains(h))
        .collect();
    common.sort();

    let mut shown = 0;
    let mut total_lines = 0;
    let mut thickness_diffs = 0;
    let mut normal_diffs = 0;
    let mut coord_diffs = 0;
    let mut entmode_diffs = 0;
    let mut color_diffs = 0;
    let mut lw_diffs = 0;
    let mut lt_scale_diffs = 0;
    let mut invisible_diffs = 0;
    let mut reactor_diffs = 0;
    let mut xdic_diffs = 0;
    let mut graphic_diffs = 0;

    for &handle in &common {
        // Read from original
        let offset_orig = match orig_reader.offset_for(handle) {
            Some(o) => o as usize,
            None => continue,
        };
        let offset_rt = match rt_reader.offset_for(handle) {
            Some(o) => o as usize,
            None => continue,
        };

        // Parse type code
        let orig_result = orig_reader.read_record_at(offset_orig);
        let rt_result = rt_reader.read_record_at(offset_rt);

        let (orig_tc, mut orig_mr) = match orig_result {
            Ok(v) => v,
            Err(_) => continue,
        };
        let (rt_tc, mut rt_mr) = match rt_result {
            Ok(v) => v,
            Err(_) => continue,
        };

        if orig_tc != 19 { continue; } // Only LINE entities
        total_lines += 1;

        // Parse common entity data
        let orig_common = orig_reader.read_common_entity_data(&mut orig_mr, orig_tc);
        let rt_common = rt_reader.read_common_entity_data(&mut rt_mr, rt_tc);

        // Parse LINE-specific data
        let orig_line = entities::read_line(&mut orig_mr, orig_reader.version());
        let rt_line = entities::read_line(&mut rt_mr, rt_reader.version());

        // Compare fields
        let mut diffs = Vec::new();

        if orig_common.entity_mode != rt_common.entity_mode {
            diffs.push(format!("entity_mode: {} vs {}", orig_common.entity_mode, rt_common.entity_mode));
            entmode_diffs += 1;
        }
        if orig_common.color != rt_common.color {
            diffs.push(format!("color: {:?} vs {:?}", orig_common.color, rt_common.color));
            color_diffs += 1;
        }
        if orig_common.line_weight != rt_common.line_weight {
            diffs.push(format!("line_weight: {} vs {}", orig_common.line_weight, rt_common.line_weight));
            lw_diffs += 1;
        }
        if (orig_common.linetype_scale - rt_common.linetype_scale).abs() > 1e-15 {
            diffs.push(format!("lt_scale: {} vs {}", orig_common.linetype_scale, rt_common.linetype_scale));
            lt_scale_diffs += 1;
        }
        if orig_common.invisible != rt_common.invisible {
            diffs.push(format!("invisible: {} vs {}", orig_common.invisible, rt_common.invisible));
            invisible_diffs += 1;
        }
        if orig_common.reactors.len() != rt_common.reactors.len() {
            diffs.push(format!("reactor_count: {} vs {}", orig_common.reactors.len(), rt_common.reactors.len()));
            reactor_diffs += 1;
        }
        if orig_common.xdictionary_handle != rt_common.xdictionary_handle {
            diffs.push(format!("xdic: {:?} vs {:?}", orig_common.xdictionary_handle, rt_common.xdictionary_handle));
            xdic_diffs += 1;
        }
        if orig_common.has_graphic != rt_common.has_graphic {
            diffs.push(format!("graphic: {} vs {}", orig_common.has_graphic, rt_common.has_graphic));
            graphic_diffs += 1;
        }

        // LINE-specific
        if (orig_line.start.x - rt_line.start.x).abs() > 1e-15
            || (orig_line.start.y - rt_line.start.y).abs() > 1e-15
            || (orig_line.start.z - rt_line.start.z).abs() > 1e-15
            || (orig_line.end.x - rt_line.end.x).abs() > 1e-15
            || (orig_line.end.y - rt_line.end.y).abs() > 1e-15
            || (orig_line.end.z - rt_line.end.z).abs() > 1e-15
        {
            diffs.push(format!("coords: start({},{},{}) end({},{},{}) vs start({},{},{}) end({},{},{})",
                orig_line.start.x, orig_line.start.y, orig_line.start.z,
                orig_line.end.x, orig_line.end.y, orig_line.end.z,
                rt_line.start.x, rt_line.start.y, rt_line.start.z,
                rt_line.end.x, rt_line.end.y, rt_line.end.z));
            coord_diffs += 1;
        }
        if (orig_line.thickness - rt_line.thickness).abs() > 1e-15 {
            diffs.push(format!("thickness: {} vs {}", orig_line.thickness, rt_line.thickness));
            thickness_diffs += 1;
        }
        if (orig_line.normal.x - rt_line.normal.x).abs() > 1e-15
            || (orig_line.normal.y - rt_line.normal.y).abs() > 1e-15
            || (orig_line.normal.z - rt_line.normal.z).abs() > 1e-15
        {
            diffs.push(format!("normal: ({},{},{}) vs ({},{},{})",
                orig_line.normal.x, orig_line.normal.y, orig_line.normal.z,
                rt_line.normal.x, rt_line.normal.y, rt_line.normal.z));
            normal_diffs += 1;
        }

        if !diffs.is_empty() && shown < max_show {
            println!("HANDLE {:#X} LINE diffs:", handle);
            for d in &diffs {
                println!("  {}", d);
            }
            shown += 1;
        }
    }

    println!("\n=== LINE FIELD COMPARISON SUMMARY ===");
    println!("Total LINE entities: {}", total_lines);
    println!("entity_mode diffs: {}", entmode_diffs);
    println!("color diffs: {}", color_diffs);
    println!("line_weight diffs: {}", lw_diffs);
    println!("lt_scale diffs: {}", lt_scale_diffs);
    println!("invisible diffs: {}", invisible_diffs);
    println!("reactor_count diffs: {}", reactor_diffs);
    println!("xdictionary diffs: {}", xdic_diffs);
    println!("graphic diffs: {}", graphic_diffs);
    println!("coordinate diffs: {}", coord_diffs);
    println!("thickness diffs: {}", thickness_diffs);
    println!("normal diffs: {}", normal_diffs);
}
