//! Diagnostic: Categorize record differences by object type.
//!
//! For each shared handle, decode the type code and categorize
//! the byte-level differences by object type.
//!
//! Usage: cargo run --example diag_type_diff

use std::collections::HashMap;
use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;
use acadrust::io::dwg::dwg_stream_readers::bit_reader::DwgBitReader;
use acadrust::io::dwg::dwg_version::DwgVersion;
use acadrust::types::DxfVersion;

fn read_handle_map(path: &str) -> HashMap<u64, i64> {
    let mut reader = DwgReader::from_file(path).expect("open");
    let info = reader.read_file_header().expect("header");
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
        .expect("parse handles")
}

fn read_objects_buffer(path: &str) -> Vec<u8> {
    let mut reader = DwgReader::from_file(path).expect("open");
    let info = reader.read_file_header().expect("header");
    reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects")
}

fn read_ms(data: &[u8], offset: usize) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = offset;
    loop {
        if pos + 1 >= data.len() { return (0, pos - offset); }
        let lo = data[pos] as usize;
        let hi = data[pos + 1] as usize;
        pos += 2;
        let word = lo | (hi << 8);
        let val = word & 0x7FFF;
        result |= val << shift;
        if (word & 0x8000) == 0 { break; }
        shift += 15;
    }
    (result, pos - offset)
}

fn read_record(data: &[u8], offset: usize) -> Option<(Vec<u8>, usize)> {
    if offset >= data.len() { return None; }
    let (size, ms_bytes) = read_ms(data, offset);
    if size == 0 || offset + ms_bytes + size + 2 > data.len() { return None; }
    let merged_start = offset + ms_bytes;
    let merged_data = data[merged_start..merged_start + size].to_vec();
    Some((merged_data, ms_bytes + size + 2))
}

fn read_type_code(merged: &[u8]) -> i16 {
    let mut reader = DwgBitReader::new(merged.to_vec(), DwgVersion::AC24, DxfVersion::AC1021);
    reader.read_object_type()
}

fn type_name(code: i16) -> &'static str {
    match code {
        1 => "TEXT", 2 => "ATTRIB", 3 => "ATTDEF", 4 => "BLOCK",
        5 => "ENDBLK", 6 => "SEQEND", 7 => "INSERT", 8 => "MINSERT",
        10 => "VERTEX_2D", 11 => "VERTEX_3D", 12 => "VERTEX_MESH",
        13 => "VERTEX_PFACE", 14 => "VERTEX_PFACE_FACE",
        15 => "POLYLINE_2D", 16 => "POLYLINE_3D", 17 => "ARC",
        18 => "CIRCLE", 19 => "LINE", 20 => "DIM_ORDINATE",
        21 => "DIM_LINEAR", 22 => "DIM_ALIGNED", 23 => "DIM_ANG_3PT",
        24 => "DIM_ANG_2LN", 25 => "DIM_RADIUS", 26 => "DIM_DIAMETER",
        27 => "POINT", 28 => "3DFACE", 29 => "POLYLINE_PFACE",
        30 => "POLYLINE_MESH", 31 => "SOLID", 32 => "TRACE",
        33 => "SHAPE", 34 => "VIEWPORT", 35 => "ELLIPSE",
        36 => "SPLINE", 37 => "REGION", 38 => "3DSOLID", 39 => "BODY",
        40 => "RAY", 41 => "XLINE", 42 => "DICTIONARY",
        43 => "OLEFRAME", 44 => "MTEXT", 45 => "LEADER",
        46 => "TOLERANCE", 47 => "MLINE",
        48 => "BLOCK_CONTROL", 49 => "BLOCK_HEADER",
        50 => "LAYER_CONTROL", 51 => "LAYER",
        52 => "STYLE_CONTROL", 53 => "STYLE",
        56 => "LTYPE_CONTROL", 57 => "LTYPE",
        60 => "VIEW_CONTROL", 61 => "VIEW",
        62 => "UCS_CONTROL", 63 => "UCS",
        64 => "VPORT_CONTROL", 65 => "VPORT",
        66 => "APPID_CONTROL", 67 => "APPID",
        68 => "DIMSTYLE_CONTROL", 69 => "DIMSTYLE",
        70 => "VPENT_HDR_CONTROL", 71 => "VPENT_HDR",
        72 => "GROUP", 73 => "MLINESTYLE",
        74 => "OLE2FRAME", 77 => "LWPOLYLINE", 78 => "HATCH",
        79 => "XRECORD", 80 => "PLACEHOLDER", 82 => "LAYOUT",
        _ => "CLASS/UNKNOWN",
    }
}

struct TypeStats {
    identical: usize,
    orig_larger: usize,
    rt_larger: usize,
    same_size_diff: usize,
    total_orig_bytes: usize,
    total_rt_bytes: usize,
}

impl Default for TypeStats {
    fn default() -> Self {
        TypeStats { identical: 0, orig_larger: 0, rt_larger: 0,
            same_size_diff: 0, total_orig_bytes: 0, total_rt_bytes: 0 }
    }
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    let orig_handles = read_handle_map(&input);
    let orig_objects = read_objects_buffer(&input);

    let mut reader = DwgReader::from_file(&input).expect("open");
    let doc = reader.read().expect("read");
    let rt_path = "target/diag_type_diff.dwg";
    DwgWriter::write_to_file(rt_path, &doc).expect("write");

    let rt_handles = read_handle_map(rt_path);
    let rt_objects = read_objects_buffer(rt_path);

    let mut shared: Vec<u64> = orig_handles.keys()
        .filter(|h| rt_handles.contains_key(h))
        .copied()
        .collect();
    shared.sort();

    let mut stats: HashMap<i16, TypeStats> = HashMap::new();

    for &handle in &shared {
        let orig_off = orig_handles[&handle] as usize;
        let rt_off = rt_handles[&handle] as usize;

        if let (Some((orig_data, _)), Some((rt_data, _))) =
            (read_record(&orig_objects, orig_off), read_record(&rt_objects, rt_off))
        {
            let tc = read_type_code(&orig_data);
            let s = stats.entry(tc).or_default();
            s.total_orig_bytes += orig_data.len();
            s.total_rt_bytes += rt_data.len();

            if orig_data == rt_data {
                s.identical += 1;
            } else if orig_data.len() > rt_data.len() {
                s.orig_larger += 1;
            } else if rt_data.len() > orig_data.len() {
                s.rt_larger += 1;
            } else {
                s.same_size_diff += 1;
            }
        }
    }

    println!("{:<25} {:>5} {:>5} {:>5} {:>5} {:>8} {:>8} {:>8}",
        "TypeCode", "Same", "O>RT", "RT>O", "SzDf", "OrigB", "RT_B", "LostB");
    println!("{}", "-".repeat(100));

    let mut entries: Vec<(i16, &TypeStats)> = stats.iter().map(|(k,v)| (*k, v)).collect();
    entries.sort_by_key(|(_, s)| std::cmp::Reverse(s.orig_larger + s.rt_larger + s.same_size_diff));

    for (tc, s) in &entries {
        let name = type_name(*tc);
        let total = s.identical + s.orig_larger + s.rt_larger + s.same_size_diff;
        let lost = s.total_orig_bytes as i64 - s.total_rt_bytes as i64;
        if total > 0 {
            println!("{:<3} {:<21} {:>5} {:>5} {:>5} {:>5} {:>8} {:>8} {:>8}",
                tc, name, s.identical, s.orig_larger, s.rt_larger, s.same_size_diff,
                s.total_orig_bytes, s.total_rt_bytes, lost);
        }
    }

    let total_identical: usize = stats.values().map(|s| s.identical).sum();
    let total_orig_larger: usize = stats.values().map(|s| s.orig_larger).sum();
    let total_rt_larger: usize = stats.values().map(|s| s.rt_larger).sum();
    let total_same_diff: usize = stats.values().map(|s| s.same_size_diff).sum();
    let total_orig: usize = stats.values().map(|s| s.total_orig_bytes).sum();
    let total_rt: usize = stats.values().map(|s| s.total_rt_bytes).sum();
    println!("{}", "-".repeat(100));
    println!("{:<25} {:>5} {:>5} {:>5} {:>5} {:>8} {:>8} {:>8}",
        "TOTAL", total_identical, total_orig_larger, total_rt_larger, total_same_diff,
        total_orig, total_rt, total_orig as i64 - total_rt as i64);
}
