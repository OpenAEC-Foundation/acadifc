/// Comprehensive DWG read-back test: reads every generated DWG file and reports results.
///
/// For each file, it checks:
///   1. File opens without error
///   2. Document reads without error
///   3. Document version matches expected version from directory name
///   4. At least one entity is present (except VIEWPORT which may be paper-space only)
///   5. Entity type matches the expected type from the filename
///   6. All entity handles are valid (non-null)
///   7. All entity common data is well-formed (layer, color, etc.)
///   8. Entity-specific geometric data is valid (non-NaN, reasonable values)
///   9. Tables are populated (layers, linetypes, text styles, dim styles, etc.)
///  10. Standard table entries exist (layer "0", linetypes "ByLayer"/"ByBlock"/"Continuous")
///  11. Block records *Model_Space and *Paper_Space exist
///  12. No error-level notifications were generated during reading
///
/// Run after `cargo run --example gen_all_entities_all_versions`
use acadrust::entities::EntityType;
use acadrust::io::dwg::DwgReader;
use acadrust::notification::NotificationType;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

const VERSIONS: &[&str] = &[
    "AC1012", "AC1014", "AC1015", "AC1018",
    "AC1021", "AC1024", "AC1027", "AC1032",
];

/// Tracks per-version statistics for the final report.
struct VersionStats {
    version: String,
    ok: u32,
    fail: u32,
    total_entities: usize,
    total_warnings: usize,
    total_file_bytes: u64,
    elapsed_ms: u128,
}

/// Helper that prints to stdout and also writes to a file.
macro_rules! out {
    ($file:expr, $($arg:tt)*) => {{
        let line = format!($($arg)*);
        println!("{}", line);
        let _ = writeln!($file, "{}", line);
    }};
}

fn main() {
    let root = "target/entities_dwg";
    let report_path = "target/entities_dwg/test_report.txt";
    // Ensure parent dir exists
    let _ = std::fs::create_dir_all("target/entities_dwg");
    let mut report_file = std::fs::File::create(report_path)
        .expect("failed to create report file");

    let mut total = 0u32;
    let mut ok = 0u32;
    let mut fail = 0u32;
    let mut errors: Vec<String> = Vec::new();
    let mut version_stats: Vec<VersionStats> = Vec::new();
    let mut entity_type_counts: HashMap<String, u32> = HashMap::new();
    let global_start = Instant::now();

    out!(report_file, "╔══════════════════════════════════════════════════════════════════════════╗");
    out!(report_file, "║              COMPREHENSIVE DWG READ-BACK TEST                           ║");
    out!(report_file, "╠══════════════════════════════════════════════════════════════════════════╣");
    out!(report_file, "║  Checks: version, tables, handles, geometry, entity data, notifications ║");
    out!(report_file, "╚══════════════════════════════════════════════════════════════════════════╝\n");

    for ver in VERSIONS {
        let dir = format!("{}/{}", root, ver);
        let dir_path = std::path::Path::new(&dir);
        if !dir_path.exists() {
            out!(report_file, "⚠  {} — directory not found, skipping", ver);
            continue;
        }

        let mut ver_stats = VersionStats {
            version: ver.to_string(),
            ok: 0,
            fail: 0,
            total_entities: 0,
            total_warnings: 0,
            total_file_bytes: 0,
            elapsed_ms: 0,
        };
        let ver_start = Instant::now();

        let mut ver_files: Vec<std::path::PathBuf> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "dwg").unwrap_or(false) {
                    ver_files.push(path);
                }
            }
        }
        ver_files.sort();

        out!(report_file, "── {} ({} files) ──────────────────────────────────────────────", ver, ver_files.len());

        for path in &ver_files {
            total += 1;
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            let entity_type = extract_entity_type(&fname);
            let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            ver_stats.total_file_bytes += file_size;

            match read_and_validate(path, &entity_type, ver) {
                Ok(report) => {
                    out!(report_file, "  ✓ {} ({} bytes)", fname, file_size);
                    for line in &report.detail_lines {
                        out!(report_file, "      {}", line);
                    }
                    ok += 1;
                    ver_stats.ok += 1;
                    ver_stats.total_entities += report.entity_count;
                    ver_stats.total_warnings += report.warning_count;
                    for t in &report.entity_type_names {
                        *entity_type_counts.entry(t.clone()).or_insert(0) += 1;
                    }
                }
                Err(e) => {
                    let msg = format!("{}: {}", fname, e);
                    out!(report_file, "  ✗ {} ({} bytes)", fname, file_size);
                    out!(report_file, "      ERROR: {}", e);
                    errors.push(msg);
                    fail += 1;
                    ver_stats.fail += 1;
                }
            }
        }

        ver_stats.elapsed_ms = ver_start.elapsed().as_millis();
        out!(report_file,
            "  ── {} summary: {} OK, {} FAIL | {} entities, {} warnings, {:.1} KB in {:.0}ms\n",
            ver,
            ver_stats.ok,
            ver_stats.fail,
            ver_stats.total_entities,
            ver_stats.total_warnings,
            ver_stats.total_file_bytes as f64 / 1024.0,
            ver_stats.elapsed_ms,
        );
        version_stats.push(ver_stats);
    }

    let global_elapsed = global_start.elapsed();

    // ── Final summary ──
    out!(report_file, "╔══════════════════════════════════════════════════════════════════════════╗");
    out!(report_file, "║  FINAL RESULTS                                                         ║");
    out!(report_file, "╠══════════════════════════════════════════════════════════════════════════╣");
    out!(report_file,
        "║  Files tested: {:>4}  |  OK: {:>4}  |  FAIL: {:>4}  |  Time: {:.1}s          ║",
        total, ok, fail, global_elapsed.as_secs_f64()
    );
    out!(report_file, "╚══════════════════════════════════════════════════════════════════════════╝");

    // Per-version breakdown
    if !version_stats.is_empty() {
        out!(report_file, "\n── PER-VERSION BREAKDOWN ───────────────────────────────────────────────");
        out!(report_file,
            "  {:<8} {:>5} {:>5} {:>5} {:>8} {:>8} {:>10}",
            "Version", "Files", "OK", "Fail", "Entities", "Warnings", "Size (KB)"
        );
        out!(report_file, "  {}", "─".repeat(60));
        for vs in &version_stats {
            out!(report_file,
                "  {:<8} {:>5} {:>5} {:>5} {:>8} {:>8} {:>10.1}",
                vs.version,
                vs.ok + vs.fail,
                vs.ok,
                vs.fail,
                vs.total_entities,
                vs.total_warnings,
                vs.total_file_bytes as f64 / 1024.0,
            );
        }
        let total_entities: usize = version_stats.iter().map(|v| v.total_entities).sum();
        let total_warnings: usize = version_stats.iter().map(|v| v.total_warnings).sum();
        let total_bytes: u64 = version_stats.iter().map(|v| v.total_file_bytes).sum();
        out!(report_file, "  {}", "─".repeat(60));
        out!(report_file,
            "  {:<8} {:>5} {:>5} {:>5} {:>8} {:>8} {:>10.1}",
            "TOTAL", total, ok, fail, total_entities, total_warnings,
            total_bytes as f64 / 1024.0,
        );
    }

    // Entity type coverage
    if !entity_type_counts.is_empty() {
        out!(report_file, "\n── ENTITY TYPE COVERAGE ────────────────────────────────────────────────");
        let mut sorted_types: Vec<_> = entity_type_counts.iter().collect();
        sorted_types.sort_by_key(|(name, _)| (*name).clone());
        let per_row = 4;
        for chunk in sorted_types.chunks(per_row) {
            let items: Vec<String> = chunk
                .iter()
                .map(|(name, count)| format!("{:<18} {:>3}", name, count))
                .collect();
            out!(report_file, "  {}", items.join("  "));
        }
        out!(report_file,
            "  ── {} distinct entity types across {} files",
            entity_type_counts.len(),
            total
        );
    }

    // Failures
    if !errors.is_empty() {
        out!(report_file, "\n── FAILURES ({}) ──────────────────────────────────────────────────", errors.len());
        for (i, e) in errors.iter().enumerate() {
            out!(report_file, "  {:>3}. {}", i + 1, e);
        }
    }

    // Flush and report file location
    let _ = report_file.flush();
    let abs_report = std::fs::canonicalize(report_path).unwrap_or_else(|_| report_path.into());
    println!("\nReport written to: {}", abs_report.display());

    if fail > 0 {
        std::process::exit(1);
    }
}

/// Report returned on successful validation of a single file.
struct ValidationReport {
    detail_lines: Vec<String>,
    entity_count: usize,
    warning_count: usize,
    entity_type_names: Vec<String>,
}

fn read_and_validate(
    path: &std::path::Path,
    expected_type: &str,
    expected_version: &str,
) -> Result<ValidationReport, String> {
    let read_start = Instant::now();
    let mut details: Vec<String> = Vec::new();

    // ── Step 1: Open ──
    let mut reader = DwgReader::from_file(path)
        .map_err(|e| format!("open failed: {}", e))?;

    // ── Step 2: Read ──
    let doc = reader.read()
        .map_err(|e| format!("read failed: {}", e))?;

    let read_ms = read_start.elapsed().as_millis();

    // ── Step 3: Verify document version ──
    let doc_version = doc.version.as_str();
    if doc_version != expected_version {
        return Err(format!(
            "version mismatch: expected {} but document reports {}",
            expected_version, doc_version
        ));
    }
    details.push(format!("version: {} (read in {}ms)", doc_version, read_ms));

    // ── Step 4: Collect table counts ──
    let layer_count = doc.layers.len();
    let linetype_count = doc.line_types.len();
    let text_style_count = doc.text_styles.len();
    let dim_style_count = doc.dim_styles.len();
    let app_id_count = doc.app_ids.len();
    let vport_count = doc.vports.len();
    let view_count = doc.views.len();
    let ucs_count = doc.ucss.len();
    let block_record_count = doc.block_records.len();
    let object_count = doc.objects.len();
    let entity_count = doc.entity_count();
    let class_count = doc.classes.len();

    details.push(format!(
        "tables: {} layers, {} LTs, {} styles, {} dimstyles, {} appids, {} vports",
        layer_count, linetype_count, text_style_count, dim_style_count,
        app_id_count, vport_count,
    ));
    if view_count > 0 || ucs_count > 0 {
        details.push(format!("        {} views, {} UCSs", view_count, ucs_count));
    }

    // ── Step 5: Validate required tables ──
    if layer_count == 0 {
        return Err("no layers found".to_string());
    }
    if linetype_count == 0 {
        return Err("no linetypes found".to_string());
    }

    // ── Step 6: Check standard table entries ──
    let has_layer0 = doc.layers.iter().any(|l| l.name == "0");
    if !has_layer0 {
        return Err("layer '0' not found".to_string());
    }

    // Verify layer 0 properties
    if let Some(layer0) = doc.layers.get("0") {
        if layer0.handle.is_null() {
            return Err("layer '0' has null handle".to_string());
        }
        details.push(format!(
            "layer 0: handle={:#X}, color={:?}, lt=\"{}\"",
            layer0.handle.value(), layer0.color, layer0.line_type,
        ));
    }

    // Check standard linetypes
    let lt_names: Vec<String> = doc.line_types.iter().map(|lt| lt.name.clone()).collect();
    let expected_lts = ["ByLayer", "ByBlock", "Continuous"];
    for name in &expected_lts {
        let found = doc.line_types.get(name).is_some();
        if !found {
            details.push(format!("⚠ standard linetype '{}' not found (have: [{}])",
                name, lt_names.join(", ")));
        }
    }

    // List all layers
    let layer_names: Vec<String> = doc.layers.iter().map(|l| l.name.clone()).collect();
    if layer_names.len() <= 8 {
        details.push(format!("layers: [{}]", layer_names.join(", ")));
    } else {
        details.push(format!(
            "layers: [{}... +{} more]",
            layer_names[..6].join(", "),
            layer_names.len() - 6,
        ));
    }

    // ── Step 7: Check block records ──
    let model_space = doc.block_records.get("*Model_Space");
    let paper_space = doc.block_records.get("*Paper_Space");

    if model_space.is_none() {
        return Err("block record '*Model_Space' not found".to_string());
    }
    if paper_space.is_none() {
        return Err("block record '*Paper_Space' not found".to_string());
    }

    let ms_entities = model_space.map(|b| b.entities.len()).unwrap_or(0);
    let ps_entities = paper_space.map(|b| b.entities.len()).unwrap_or(0);
    let custom_blocks: Vec<String> = doc.block_records.iter()
        .filter(|b| !b.is_layout())
        .map(|b| b.name.clone())
        .collect();

    details.push(format!(
        "blocks: {} total, MS={} entities, PS={} entities{}",
        block_record_count, ms_entities, ps_entities,
        if custom_blocks.is_empty() {
            String::new()
        } else {
            format!(", custom: [{}]", custom_blocks.join(", "))
        },
    ));

    // ── Step 8: Check entities (VIEWPORT may have 0 in model space) ──
    if entity_count == 0 && expected_type != "VIEWPORT" {
        return Err(format!("no entities found (expected {})", expected_type));
    }

    // ── Step 9: Verify entity types match ──
    let entity_types: Vec<String> = doc.entities()
        .map(|e| get_entity_type_name(e).to_uppercase())
        .collect();

    let type_matched = if expected_type == "HATCH_LINES" || expected_type == "HATCH_SOLID" {
        entity_types.iter().any(|t: &String| t.contains("HATCH"))
    } else if expected_type == "DIMENSION" {
        entity_types.iter().any(|t: &String| t.contains("DIMENSION"))
    } else if expected_type == "POLYLINE2D" {
        entity_types.iter().any(|t: &String| t.contains("POLYLINE") || t.contains("LWPOLYLINE"))
    } else if expected_type == "POLYLINE3D" {
        entity_types.iter().any(|t: &String| t.contains("POLYLINE"))
    } else if expected_type == "POLYFACE" {
        entity_types.iter().any(|t: &String| t.contains("POLYFACE") || t.contains("POLYLINE"))
    } else if expected_type == "VIEWPORT" {
        true // viewport may not appear as a model-space entity
    } else if expected_type == "INSERT" {
        entity_types.iter().any(|t: &String| t.contains("INSERT"))
    } else if expected_type == "MULTILEADER" {
        entity_types.iter().any(|t: &String| t.contains("MULTILEADER") || t.contains("MULTI"))
    } else {
        entity_types.iter().any(|t: &String| t.contains(expected_type))
    };

    if !type_matched && entity_count > 0 {
        return Err(format!(
            "expected '{}' but found: [{}]",
            expected_type,
            entity_types.join(", ")
        ));
    }

    // ── Step 10: Validate every entity's common data & geometry ──
    let mut handle_set: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut null_handle_count = 0usize;
    let mut duplicate_handle_count = 0usize;
    let mut invisible_count = 0usize;
    let mut non_default_layer_count = 0usize;
    let mut colored_count = 0usize;
    let mut geometry_issues: Vec<String> = Vec::new();

    for ent in doc.entities() {
        let common = ent.common();

        // Handle validation
        if common.handle.is_null() {
            null_handle_count += 1;
        } else if !handle_set.insert(common.handle.value()) {
            duplicate_handle_count += 1;
        }

        // Layer sanity
        if common.layer != "0" {
            non_default_layer_count += 1;
        }

        // Color tracking
        match common.color {
            acadrust::types::Color::ByLayer => {}
            _ => colored_count += 1,
        }

        // Visibility
        if common.invisible {
            invisible_count += 1;
        }

        // Entity-specific geometry validation
        let geo_issue = validate_entity_geometry(ent);
        if let Some(issue) = geo_issue {
            geometry_issues.push(issue);
        }
    }

    // ── Entity data summary ──
    for ent in doc.entities() {
        let data = format_entity_data(ent);
        if !data.is_empty() {
            details.push(format!("  → {}", data));
        }
    }

    details.push(format!(
        "entities: {} total, types: [{}]",
        entity_count,
        entity_types.join(", "),
    ));

    // Handle stats
    let valid_handles = handle_set.len();
    let mut handle_detail = format!("{} valid handles", valid_handles);
    if null_handle_count > 0 {
        handle_detail.push_str(&format!(", {} null", null_handle_count));
    }
    if duplicate_handle_count > 0 {
        handle_detail.push_str(&format!(", {} duplicates", duplicate_handle_count));
    }
    details.push(format!("handles: {}", handle_detail));

    // Entity property stats
    let mut prop_parts: Vec<String> = Vec::new();
    if colored_count > 0 {
        prop_parts.push(format!("{} colored", colored_count));
    }
    if non_default_layer_count > 0 {
        prop_parts.push(format!("{} non-layer-0", non_default_layer_count));
    }
    if invisible_count > 0 {
        prop_parts.push(format!("{} invisible", invisible_count));
    }
    if !prop_parts.is_empty() {
        details.push(format!("properties: {}", prop_parts.join(", ")));
    }

    // Geometry issues (non-fatal, report as warnings)
    if !geometry_issues.is_empty() {
        let max_show = 3;
        for issue in geometry_issues.iter().take(max_show) {
            details.push(format!("⚠ geometry: {}", issue));
        }
        if geometry_issues.len() > max_show {
            details.push(format!(
                "⚠ ... +{} more geometry warnings",
                geometry_issues.len() - max_show
            ));
        }
    }

    if duplicate_handle_count > 0 {
        return Err(format!(
            "{} duplicate entity handles detected",
            duplicate_handle_count
        ));
    }

    // ── Step 11: Check objects ──
    if object_count > 0 {
        details.push(format!("objects: {} non-graphical objects", object_count));
    }

    // ── Step 12: Check classes ──
    if class_count > 0 {
        details.push(format!("classes: {} DXF class definitions", class_count));
    }

    // ── Step 13: Check header variables ──
    let header = &doc.header;
    let mut header_parts: Vec<String> = Vec::new();
    if header.handle_seed > 0 {
        header_parts.push(format!("seed={:#X}", header.handle_seed));
    }
    header_parts.push(format!("ltscale={}", header.linetype_scale));
    header_parts.push(format!("texthgt={}", header.text_height));
    if header.measurement != 0 {
        header_parts.push(format!("metric={}", header.measurement));
    }
    details.push(format!("header: {}", header_parts.join(", ")));

    // ── Step 14: Check notifications ──
    let notifications = &doc.notifications;
    let warning_count = notifications.len();
    let error_notifs = notifications.of_type(NotificationType::Error);
    let warning_notifs = notifications.of_type(NotificationType::Warning);
    let not_impl_notifs = notifications.of_type(NotificationType::NotImplemented);

    if !notifications.is_empty() {
        details.push(format!(
            "notifications: {} total ({} errors, {} warnings, {} not-implemented)",
            warning_count,
            error_notifs.len(),
            warning_notifs.len(),
            not_impl_notifs.len(),
        ));
        // Show first few notifications for context
        let max_notifs = 3;
        for notif in notifications.iter().take(max_notifs) {
            details.push(format!("  [{}] {}", notif.notification_type, notif.message));
        }
        if warning_count > max_notifs {
            details.push(format!("  ... +{} more notifications", warning_count - max_notifs));
        }
    }

    // Error notifications are fatal
    if !error_notifs.is_empty() {
        let msgs: Vec<String> = error_notifs.iter().take(5).map(|n| n.message.clone()).collect();
        return Err(format!(
            "{} error notifications: [{}]",
            error_notifs.len(),
            msgs.join("; "),
        ));
    }

    let unique_types: Vec<String> = {
        let mut t = entity_types.clone();
        t.sort();
        t.dedup();
        t
    };

    Ok(ValidationReport {
        detail_lines: details,
        entity_count,
        warning_count,
        entity_type_names: unique_types,
    })
}

/// Validates entity-specific geometric data; returns Some(issue) if bad.
fn validate_entity_geometry(entity: &EntityType) -> Option<String> {
    match entity {
        EntityType::Line(l) => {
            if has_nan_vec3(&l.start) || has_nan_vec3(&l.end) {
                return Some(format!(
                    "LINE has NaN coords: start=({},{},{}), end=({},{},{})",
                    l.start.x, l.start.y, l.start.z,
                    l.end.x, l.end.y, l.end.z,
                ));
            }
        }
        EntityType::Circle(c) => {
            if has_nan_vec3(&c.center) {
                return Some(format!("CIRCLE has NaN center"));
            }
            if c.radius.is_nan() || c.radius < 0.0 {
                return Some(format!("CIRCLE has invalid radius: {}", c.radius));
            }
        }
        EntityType::Arc(a) => {
            if has_nan_vec3(&a.center) {
                return Some(format!("ARC has NaN center"));
            }
            if a.radius.is_nan() || a.radius < 0.0 {
                return Some(format!("ARC has invalid radius: {}", a.radius));
            }
            if a.start_angle.is_nan() || a.end_angle.is_nan() {
                return Some(format!(
                    "ARC has NaN angles: start={}, end={}",
                    a.start_angle, a.end_angle,
                ));
            }
        }
        EntityType::Ellipse(e) => {
            if has_nan_vec3(&e.center) {
                return Some(format!("ELLIPSE has NaN center"));
            }
            if e.minor_axis_ratio.is_nan() || e.minor_axis_ratio <= 0.0 || e.minor_axis_ratio > 1.0
            {
                return Some(format!(
                    "ELLIPSE has invalid minor_axis_ratio: {}",
                    e.minor_axis_ratio
                ));
            }
        }
        EntityType::Text(t) => {
            if has_nan_vec3(&t.insertion_point) {
                return Some(format!("TEXT has NaN insertion point"));
            }
            if t.height.is_nan() || t.height <= 0.0 {
                return Some(format!("TEXT has invalid height: {}", t.height));
            }
        }
        EntityType::MText(m) => {
            if has_nan_vec3(&m.insertion_point) {
                return Some(format!("MTEXT has NaN insertion point"));
            }
            if m.height.is_nan() || m.height <= 0.0 {
                return Some(format!("MTEXT has invalid height: {}", m.height));
            }
        }
        EntityType::Point(p) => {
            if has_nan_vec3(&p.location) {
                return Some(format!("POINT has NaN location"));
            }
        }
        EntityType::Spline(s) => {
            if s.control_points.is_empty() && s.fit_points.is_empty() {
                return Some(format!(
                    "SPLINE has no control points and no fit points"
                ));
            }
            if s.degree < 1 {
                return Some(format!("SPLINE has invalid degree: {}", s.degree));
            }
        }
        EntityType::LwPolyline(lw) => {
            if lw.vertices.is_empty() {
                return Some(format!("LWPOLYLINE has no vertices"));
            }
        }
        EntityType::Insert(ins) => {
            if ins.block_name.is_empty() {
                return Some(format!("INSERT has empty block_name"));
            }
            if has_nan_vec3(&ins.insert_point) {
                return Some(format!("INSERT has NaN insert point"));
            }
        }
        EntityType::Ray(r) => {
            if has_nan_vec3(&r.base_point) || has_nan_vec3(&r.direction) {
                return Some(format!("RAY has NaN coordinates"));
            }
        }
        EntityType::XLine(x) => {
            if has_nan_vec3(&x.base_point) || has_nan_vec3(&x.direction) {
                return Some(format!("XLINE has NaN coordinates"));
            }
        }
        EntityType::Solid(s) => {
            if has_nan_vec3(&s.first_corner)
                || has_nan_vec3(&s.second_corner)
                || has_nan_vec3(&s.third_corner)
                || has_nan_vec3(&s.fourth_corner)
            {
                return Some(format!("SOLID has NaN corner coordinates"));
            }
        }
        EntityType::Face3D(f) => {
            if has_nan_vec3(&f.first_corner)
                || has_nan_vec3(&f.second_corner)
                || has_nan_vec3(&f.third_corner)
                || has_nan_vec3(&f.fourth_corner)
            {
                return Some(format!("3DFACE has NaN corner coordinates"));
            }
        }
        EntityType::Mesh(m) => {
            if m.vertices.is_empty() {
                return Some(format!("MESH has no vertices"));
            }
        }
        EntityType::Hatch(h) => {
            if h.paths.is_empty() {
                return Some(format!("HATCH has no boundary paths"));
            }
        }
        EntityType::Leader(l) => {
            if l.vertices.is_empty() {
                return Some(format!("LEADER has no vertices"));
            }
        }
        _ => {}
    }
    None
}

fn has_nan_vec3(v: &acadrust::types::Vector3) -> bool {
    v.x.is_nan() || v.y.is_nan() || v.z.is_nan()
}

fn fmt_vec3(v: &acadrust::types::Vector3) -> String {
    if v.z == 0.0 {
        format!("({:.4}, {:.4})", v.x, v.y)
    } else {
        format!("({:.4}, {:.4}, {:.4})", v.x, v.y, v.z)
    }
}

/// Returns a one-line summary of the entity's key data fields.
fn format_entity_data(entity: &EntityType) -> String {
    let ty = get_entity_type_name(entity);
    let common = entity.common();
    let handle = format!("{:#X}", common.handle.value());
    let layer = &common.layer;

    let data = match entity {
        EntityType::Line(l) => {
            format!("start={} end={} thickness={}", fmt_vec3(&l.start), fmt_vec3(&l.end), l.thickness)
        }
        EntityType::Circle(c) => {
            format!("center={} r={:.4} thickness={}", fmt_vec3(&c.center), c.radius, c.thickness)
        }
        EntityType::Arc(a) => {
            format!("center={} r={:.4} start={:.2}° end={:.2}°",
                fmt_vec3(&a.center), a.radius,
                a.start_angle.to_degrees(), a.end_angle.to_degrees())
        }
        EntityType::Ellipse(e) => {
            format!("center={} major={} ratio={:.4} param=[{:.2}..{:.2}]",
                fmt_vec3(&e.center), fmt_vec3(&e.major_axis),
                e.minor_axis_ratio, e.start_parameter, e.end_parameter)
        }
        EntityType::Point(p) => {
            format!("location={} thickness={}", fmt_vec3(&p.location), p.thickness)
        }
        EntityType::Text(t) => {
            format!("\"{}\" at={} h={:.4} rot={:.1}° style=\"{}\"",
                truncate_str(&t.value, 40), fmt_vec3(&t.insertion_point),
                t.height, t.rotation.to_degrees(), t.style)
        }
        EntityType::MText(m) => {
            format!("\"{}\" at={} h={:.4} w={:.1} rot={:.1}° style=\"{}\"",
                truncate_str(&m.value, 40), fmt_vec3(&m.insertion_point),
                m.height, m.rectangle_width, m.rotation.to_degrees(), m.style)
        }
        EntityType::Spline(s) => {
            format!("degree={} {} ctrl_pts, {} knots, {} fit_pts flags=[closed={},rational={},planar={}]",
                s.degree, s.control_points.len(), s.knots.len(), s.fit_points.len(),
                s.flags.closed, s.flags.rational, s.flags.planar)
        }
        EntityType::LwPolyline(lw) => {
            let pts: Vec<String> = lw.vertices.iter().take(4)
                .map(|v| format!("({:.4},{:.4})", v.location.x, v.location.y))
                .collect();
            let suffix = if lw.vertices.len() > 4 { format!("...+{}", lw.vertices.len() - 4) } else { String::new() };
            format!("{} verts [{}{}] closed={} width={:.2} elev={:.2}",
                lw.vertices.len(), pts.join(", "), suffix, lw.is_closed, lw.constant_width, lw.elevation)
        }
        EntityType::Polyline2D(p) => {
            format!("{} verts elev={:.2} thickness={:.2}",
                p.vertices.len(), p.elevation, p.thickness)
        }
        EntityType::Polyline3D(p) => {
            format!("{} verts", p.vertices.len())
        }
        EntityType::PolyfaceMesh(pf) => {
            format!("{} verts, {} faces elev={:.2}",
                pf.vertices.len(), pf.faces.len(), pf.elevation)
        }
        EntityType::Insert(ins) => {
            format!("block=\"{}\" at={} scale=({:.2},{:.2},{:.2}) rot={:.1}° rows={} cols={}",
                ins.block_name, fmt_vec3(&ins.insert_point),
                ins.x_scale, ins.y_scale, ins.z_scale,
                ins.rotation.to_degrees(), ins.row_count, ins.column_count)
        }
        EntityType::Ray(r) => {
            format!("base={} dir={}", fmt_vec3(&r.base_point), fmt_vec3(&r.direction))
        }
        EntityType::XLine(x) => {
            format!("base={} dir={}", fmt_vec3(&x.base_point), fmt_vec3(&x.direction))
        }
        EntityType::Solid(s) => {
            format!("corners: {} {} {} {} thickness={}",
                fmt_vec3(&s.first_corner), fmt_vec3(&s.second_corner),
                fmt_vec3(&s.third_corner), fmt_vec3(&s.fourth_corner), s.thickness)
        }
        EntityType::Face3D(f) => {
            format!("corners: {} {} {} {}",
                fmt_vec3(&f.first_corner), fmt_vec3(&f.second_corner),
                fmt_vec3(&f.third_corner), fmt_vec3(&f.fourth_corner))
        }
        EntityType::Mesh(m) => {
            format!("{} verts, {} faces, {} edges subdiv={}",
                m.vertices.len(), m.faces.len(), m.edges.len(), m.subdivision_level)
        }
        EntityType::Hatch(h) => {
            format!("{} paths solid={} pattern=\"{}\" angle={:.1}° scale={:.2}",
                h.paths.len(), h.is_solid,
                h.pattern.name, h.pattern_angle.to_degrees(), h.pattern_scale)
        }
        EntityType::Leader(l) => {
            format!("{} verts style=\"{}\" arrow={} text_h={:.2}",
                l.vertices.len(), l.dimension_style, l.arrow_enabled, l.text_height)
        }
        EntityType::MultiLeader(ml) => {
            let text = &ml.context.text_string;
            let text_part = if text.is_empty() { String::new() } else { format!(" text=\"{}\"", truncate_str(text, 30)) };
            format!("{} roots scale={:.2}{} arrowsz={:.2}",
                ml.context.leader_roots.len(), ml.context.scale_factor, text_part, ml.arrowhead_size)
        }
        EntityType::MLine(ml) => {
            format!("{} verts style=\"{}\" scale={:.2} start={}",
                ml.vertices.len(), ml.style_name, ml.scale_factor, fmt_vec3(&ml.start_point))
        }
        EntityType::Dimension(d) => {
            let base = d.base();
            let dim_type = match d {
                acadrust::entities::Dimension::Aligned(_) => "Aligned",
                acadrust::entities::Dimension::Linear(_) => "Linear",
                acadrust::entities::Dimension::Radius(_) => "Radius",
                acadrust::entities::Dimension::Diameter(_) => "Diameter",
                acadrust::entities::Dimension::Angular2Ln(_) => "Angular2Ln",
                acadrust::entities::Dimension::Angular3Pt(_) => "Angular3Pt",
                acadrust::entities::Dimension::Ordinate(_) => "Ordinate",
            };
            let text = if base.text.is_empty() { String::new() } else { format!(" text=\"{}\"", truncate_str(&base.text, 20)) };
            format!("{} defpt={} midpt={}{} meas={:.4} style=\"{}\"",
                dim_type, fmt_vec3(&base.definition_point), fmt_vec3(&base.text_middle_point),
                text, base.actual_measurement, base.style_name)
        }
        EntityType::Solid3D(s) => {
            let acis_len = if s.acis_data.is_binary { s.acis_data.sab_data.len() } else { s.acis_data.sat_data.len() };
            format!("ACIS v{:?} data={}bytes binary={}", s.acis_data.version, acis_len, s.acis_data.is_binary)
        }
        EntityType::Region(r) => {
            let acis_len = if r.acis_data.is_binary { r.acis_data.sab_data.len() } else { r.acis_data.sat_data.len() };
            format!("ACIS v{:?} data={}bytes binary={}", r.acis_data.version, acis_len, r.acis_data.is_binary)
        }
        EntityType::Body(b) => {
            let acis_len = if b.acis_data.is_binary { b.acis_data.sab_data.len() } else { b.acis_data.sat_data.len() };
            format!("ACIS v{:?} data={}bytes binary={}", b.acis_data.version, acis_len, b.acis_data.is_binary)
        }
        EntityType::Tolerance(t) => {
            format!("at={} dir={} text=\"{}\" style=\"{}\"",
                fmt_vec3(&t.insertion_point), fmt_vec3(&t.direction),
                truncate_str(&t.text, 30), t.dimension_style_name)
        }
        EntityType::Shape(s) => {
            format!("at={} size={:.2} name=\"{}\" num={} rot={:.1}° xscale={:.2}",
                fmt_vec3(&s.insertion_point), s.size, s.shape_name, s.shape_number,
                s.rotation.to_degrees(), s.relative_x_scale)
        }
        EntityType::Viewport(vp) => {
            format!("center={} {}x{} id={} view_h={:.2} lens={:.2}",
                fmt_vec3(&vp.center), vp.width, vp.height, vp.id,
                vp.view_height, vp.lens_length)
        }
        EntityType::Table(t) => {
            format!("at={} {} rows, {} cols",
                fmt_vec3(&t.insertion_point), t.rows.len(), t.columns.len())
        }
        EntityType::AttributeDefinition(ad) => {
            format!("tag=\"{}\" prompt=\"{}\" default=\"{}\" at={} h={:.2}",
                ad.tag, truncate_str(&ad.prompt, 20), truncate_str(&ad.default_value, 20),
                fmt_vec3(&ad.insertion_point), ad.height)
        }
        EntityType::AttributeEntity(ae) => {
            format!("tag=\"{}\" value=\"{}\" at={} h={:.2}",
                ae.tag, truncate_str(&ae.value, 30), fmt_vec3(&ae.insertion_point), ae.height)
        }
        EntityType::Block(b) => {
            format!("name=\"{}\" base={} desc=\"{}\"",
                b.name, fmt_vec3(&b.base_point), truncate_str(&b.description, 20))
        }
        EntityType::RasterImage(_) => "raster image".to_string(),
        EntityType::Wipeout(_) => "wipeout".to_string(),
        EntityType::Underlay(_) => "underlay".to_string(),
        EntityType::Ole2Frame(_) => "OLE2 frame".to_string(),
        EntityType::Polyline(_) => "legacy polyline".to_string(),
        EntityType::PolygonMesh(_) => "polygon mesh".to_string(),
        EntityType::BlockEnd(_) | EntityType::Seqend(_) => return String::new(),
        EntityType::Unknown(u) => {
            format!("dxf_name=\"{}\"", u.dxf_name)
        }
    };

    format!("{} [h={}, layer=\"{}\"] {}", ty, handle, layer, data)
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn extract_entity_type(filename: &str) -> String {
    // filename format: entity_AC1014_CIRCLE.dwg
    let stem = filename.trim_end_matches(".dwg");
    // Find the part after the second underscore (version_ENTITYTYPE)
    let parts: Vec<&str> = stem.splitn(3, '_').collect();
    if parts.len() >= 3 {
        parts[2].to_uppercase()
    } else {
        stem.to_uppercase()
    }
}

fn get_entity_type_name(e: &acadrust::entities::EntityType) -> String {
    use acadrust::entities::EntityType;
    match e {
        EntityType::Point(_) => "POINT".to_string(),
        EntityType::Line(_) => "LINE".to_string(),
        EntityType::Circle(_) => "CIRCLE".to_string(),
        EntityType::Arc(_) => "ARC".to_string(),
        EntityType::Ellipse(_) => "ELLIPSE".to_string(),
        EntityType::Polyline(_) => "POLYLINE".to_string(),
        EntityType::Polyline2D(_) => "POLYLINE2D".to_string(),
        EntityType::Polyline3D(_) => "POLYLINE3D".to_string(),
        EntityType::LwPolyline(_) => "LWPOLYLINE".to_string(),
        EntityType::Text(_) => "TEXT".to_string(),
        EntityType::MText(_) => "MTEXT".to_string(),
        EntityType::Spline(_) => "SPLINE".to_string(),
        EntityType::Dimension(_) => "DIMENSION".to_string(),
        EntityType::Hatch(_) => "HATCH".to_string(),
        EntityType::Solid(_) => "SOLID".to_string(),
        EntityType::Face3D(_) => "FACE3D".to_string(),
        EntityType::Insert(_) => "INSERT".to_string(),
        EntityType::Block(_) => "BLOCK".to_string(),
        EntityType::BlockEnd(_) => "BLOCKEND".to_string(),
        EntityType::Ray(_) => "RAY".to_string(),
        EntityType::XLine(_) => "XLINE".to_string(),
        EntityType::Viewport(_) => "VIEWPORT".to_string(),
        EntityType::AttributeDefinition(_) => "ATTDEF".to_string(),
        EntityType::AttributeEntity(_) => "ATTRIB".to_string(),
        EntityType::Leader(_) => "LEADER".to_string(),
        EntityType::MultiLeader(_) => "MULTILEADER".to_string(),
        EntityType::MLine(_) => "MLINE".to_string(),
        EntityType::Mesh(_) => "MESH".to_string(),
        EntityType::RasterImage(_) => "IMAGE".to_string(),
        EntityType::Solid3D(_) => "SOLID3D".to_string(),
        EntityType::Region(_) => "REGION".to_string(),
        EntityType::Body(_) => "BODY".to_string(),
        EntityType::Table(_) => "TABLE".to_string(),
        EntityType::Tolerance(_) => "TOLERANCE".to_string(),
        EntityType::PolyfaceMesh(_) => "POLYFACEMESH".to_string(),
        EntityType::Wipeout(_) => "WIPEOUT".to_string(),
        EntityType::Shape(_) => "SHAPE".to_string(),
        EntityType::Underlay(_) => "UNDERLAY".to_string(),
        EntityType::Seqend(_) => "SEQEND".to_string(),
        EntityType::Ole2Frame(_) => "OLE2FRAME".to_string(),
        EntityType::PolygonMesh(_) => "POLYGONMESH".to_string(),
        EntityType::Unknown(u) => {
            if u.dxf_name.is_empty() {
                "UNKNOWN".to_string()
            } else {
                format!("UNKNOWN({})", u.dxf_name)
            }
        }
    }
}
