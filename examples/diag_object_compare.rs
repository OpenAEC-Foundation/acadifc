/// Compare raw object bytes between two DWG files (original vs roundtripped).
///
/// For each handle that exists in BOTH files' handle maps, extract the
/// raw record bytes from the AcDb:AcDbObjects section and compare.
/// Report the first N differences.

use acadrust::DwgReader;
use std::collections::HashMap;

/// Parse a single MS (Modular Short) from data, return (value, bytes_consumed).
fn read_ms(data: &[u8]) -> (usize, usize) {
    let mut value: usize = 0;
    let mut shift = 0;
    let mut i = 0;
    loop {
        if i + 1 >= data.len() { break; }
        let word = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;
        value |= ((word & 0x7FFF) as usize) << shift;
        shift += 15;
        if (word & 0x8000) == 0 { break; }
    }
    (value, i)
}

/// Extract (handle_map, objects_buf) from a DWG file.
fn load_file(path: &str) -> Option<(HashMap<u64, i64>, Vec<u8>)> {
    let mut reader = DwgReader::from_file(path).ok()?;
    let info = reader.read_file_header().ok()?;
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).ok()?;
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf).ok()?;
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).ok()?;
    Some((handle_map, objects_buf))
}

/// Extract the raw record bytes at a given offset in the objects buffer.
/// Returns (ms_len, size, raw_record_with_framing).
fn extract_record(buf: &[u8], offset: usize) -> Option<(usize, usize, &[u8])> {
    if offset >= buf.len() { return None; }
    let (size, ms_len) = read_ms(&buf[offset..]);
    let total = ms_len + size;
    if offset + total > buf.len() { return None; }
    Some((ms_len, size, &buf[offset..offset + total]))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cargo run --example diag_object_compare -- <original.dwg> <roundtrip.dwg> [max_diffs]");
        std::process::exit(2);
    }
    let orig_path = &args[1];
    let rt_path = &args[2];
    let max_diffs: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20);
    let filter_type: Option<u16> = args.get(4).and_then(|s| s.parse().ok());
    let filter_handle: Option<u64> = args.get(5).and_then(|s| u64::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"), 16).ok());

    eprintln!("Loading original: {}", orig_path);
    let (orig_map, orig_buf) = match load_file(orig_path) {
        Some(v) => v,
        None => { eprintln!("Failed to load original"); std::process::exit(1); }
    };

    eprintln!("Loading RT: {}", rt_path);
    let (rt_map, rt_buf) = match load_file(rt_path) {
        Some(v) => v,
        None => { eprintln!("Failed to load RT"); std::process::exit(1); }
    };

    eprintln!("Original: {} handles, {} bytes objects section", orig_map.len(), orig_buf.len());
    eprintln!("RT: {} handles, {} bytes objects section", rt_map.len(), rt_buf.len());

    // Handles present in both
    let mut common_handles: Vec<u64> = orig_map.keys()
        .filter(|h| rt_map.contains_key(h))
        .copied()
        .collect();
    common_handles.sort();

    let orig_only: Vec<u64> = orig_map.keys().filter(|h| !rt_map.contains_key(h)).copied().collect();
    let rt_only: Vec<u64> = rt_map.keys().filter(|h| !orig_map.contains_key(h)).copied().collect();

    eprintln!("Common handles: {}", common_handles.len());
    eprintln!("Original-only handles: {}", orig_only.len());
    eprintln!("RT-only handles: {}", rt_only.len());

    if !orig_only.is_empty() {
        let show: Vec<String> = orig_only.iter().take(20).map(|h| format!("{:#X}", h)).collect();
        eprintln!("  First original-only: {:?}", show);
    }
    if !rt_only.is_empty() {
        let show: Vec<String> = rt_only.iter().take(20).map(|h| format!("{:#X}", h)).collect();
        eprintln!("  First RT-only: {:?}", show);
    }

    // Compare objects — track by type code
    let mut diff_count = 0;
    let mut match_count = 0;
    let mut size_only_diff = 0;
    let mut ms_len_diff = 0;
    let mut diff_by_type: HashMap<u16, usize> = HashMap::new();
    let mut match_by_type: HashMap<u16, usize> = HashMap::new();
    let mut size_delta_by_type: HashMap<u16, Vec<i64>> = HashMap::new();
    let mut printed = 0;

    for &handle in &common_handles {
        let orig_offset = orig_map[&handle] as usize;
        let rt_offset = rt_map[&handle] as usize;

        let orig_rec = match extract_record(&orig_buf, orig_offset) {
            Some(r) => r,
            None => {
                diff_count += 1;
                continue;
            }
        };
        let rt_rec = match extract_record(&rt_buf, rt_offset) {
            Some(r) => r,
            None => {
                diff_count += 1;
                continue;
            }
        };

        let (orig_ms_len, orig_size, orig_bytes) = orig_rec;
        let (rt_ms_len, rt_size, rt_bytes) = rt_rec;

        // Compare the OBJECT DATA (after MS framing)
        let orig_data = &orig_bytes[orig_ms_len..];
        let rt_data = &rt_bytes[rt_ms_len..];

        // Read type code from first 2 bits + rest
        let type_code = read_type_code(orig_data);

        if let Some(ft) = filter_type {
            if type_code != ft {
                if orig_data == rt_data { match_count += 1; } else { diff_count += 1; }
                *match_by_type.entry(type_code).or_insert(0) += 1;
                continue;
            }
        }
        if let Some(fh) = filter_handle {
            if handle != fh {
                if orig_data == rt_data { match_count += 1; } else { diff_count += 1; }
                *match_by_type.entry(type_code).or_insert(0) += 1;
                continue;
            }
        }

        if orig_data == rt_data {
            match_count += 1;
            *match_by_type.entry(type_code).or_insert(0) += 1;
            continue;
        }

        *diff_by_type.entry(type_code).or_insert(0) += 1;

        if orig_ms_len != rt_ms_len {
            ms_len_diff += 1;
        }

        if orig_size != rt_size {
            size_only_diff += 1;
            let delta = rt_size as i64 - orig_size as i64;
            size_delta_by_type.entry(type_code).or_insert_with(Vec::new).push(delta);
            if printed < max_diffs {
                println!("HANDLE {:#X} TYPE={}: SIZE DIFFERS orig={} rt={} delta={}", handle, type_code, orig_size, rt_size, delta);
                // Show full bytes of both
                print!("  orig ALL [{}]: ", orig_data.len());
                for i in 0..orig_data.len() { print!("{:02X} ", orig_data[i]); }
                println!();
                print!("  rt   ALL [{}]: ", rt_data.len());
                for i in 0..rt_data.len() { print!("{:02X} ", rt_data[i]); }
                println!();
                // Show last 10 bytes
                let tail = 10;
                if orig_data.len() > tail {
                    let start = orig_data.len() - tail;
                    print!("  orig TAIL[{}..{}]: ", start, orig_data.len());
                    for i in start..orig_data.len() { print!("{:02X} ", orig_data[i]); }
                    println!();
                }
                if rt_data.len() > tail {
                    let start = rt_data.len() - tail;
                    print!("  rt   TAIL[{}..{}]: ", start, rt_data.len());
                    for i in start..rt_data.len() { print!("{:02X} ", rt_data[i]); }
                    println!();
                }
                // Decode RL for R2007 (BS type code then RL)
                decode_rl_r2007(orig_data, "orig");
                decode_rl_r2007(rt_data, "rt  ");
                // Show byte diff positions
                let min_len = orig_data.len().min(rt_data.len());
                let mut diff_positions = Vec::new();
                for i in 0..min_len {
                    if orig_data[i] != rt_data[i] { diff_positions.push(i); }
                }
                if orig_data.len() > min_len {
                    for i in min_len..orig_data.len() { diff_positions.push(i); }
                }
                println!("  byte_diff_positions: {:?}", diff_positions);
                printed += 1;
            }
            diff_count += 1;
            continue;
        }

        // Same size, different content
        if printed < max_diffs {
            let min_len = orig_data.len().min(rt_data.len());
            let mut first_diff = None;
            let mut diff_bytes_count = 0;
            for i in 0..min_len {
                if orig_data[i] != rt_data[i] {
                    if first_diff.is_none() { first_diff = Some(i); }
                    diff_bytes_count += 1;
                }
            }
            if let Some(d) = first_diff {
                println!(
                    "HANDLE {:#X} TYPE={}: size={} first_diff={} total_diff_bytes={}",
                    handle, type_code, orig_size, d, diff_bytes_count
                );
                // If a filter handle is set, dump full bytes for analysis
                if filter_handle.is_some() {
                    print!("  orig ALL [{}]: ", orig_data.len());
                    for b in orig_data { print!("{:02X} ", b); }
                    println!();
                    print!("  rt   ALL [{}]: ", rt_data.len());
                    for b in rt_data { print!("{:02X} ", b); }
                    println!();
                    decode_rl_r2007(orig_data, "orig");
                    decode_rl_r2007(rt_data, "rt  ");
                    let mut diff_positions = Vec::new();
                    for i in 0..min_len {
                        if orig_data[i] != rt_data[i] { diff_positions.push(i); }
                    }
                    println!("  byte_diff_positions: {:?}", diff_positions);
                } else {
                    let start = d.saturating_sub(4);
                    let end = (d + 5).min(orig_data.len());
                    print!("  orig[{}..{}]: ", start, end);
                    for i in start..end {
                        if i == d { print!("[{:02X}]", orig_data[i]); }
                        else { print!("{:02X} ", orig_data[i]); }
                    }
                    println!();
                    let end_rt = (d + 5).min(rt_data.len());
                    print!("  rt  [{}..{}]: ", start, end_rt);
                    for i in start..end_rt {
                        if i == d { print!("[{:02X}]", rt_data[i]); }
                        else { print!("{:02X} ", rt_data[i]); }
                    }
                    println!();
                }
            }
            printed += 1;
        }
        diff_count += 1;
    }

    println!("\n=== SUMMARY ===");
    println!("Common handles compared: {}", common_handles.len());
    println!("Matching: {}", match_count);
    println!("Different: {}", diff_count);
    println!("  Size-only diffs: {}", size_only_diff);
    println!("  MS-length diffs: {}", ms_len_diff);

    println!("\n=== DIFFS BY TYPE CODE ===");
    let mut type_list: Vec<_> = diff_by_type.iter().collect();
    type_list.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (tc, count) in &type_list {
        let matched = match_by_type.get(tc).copied().unwrap_or(0);
        println!("  type={}: {} different, {} matching", tc, count, matched);
    }

    println!("\n=== MATCH-ONLY TYPES ===");
    let mut match_list: Vec<_> = match_by_type.iter()
        .filter(|(tc, _)| !diff_by_type.contains_key(tc))
        .collect();
    match_list.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (tc, count) in &match_list {
        println!("  type={}: {} all matching", tc, count);
    }

    if !size_delta_by_type.is_empty() {
        println!("\n=== SIZE DELTA PATTERNS BY TYPE ===");
        let mut sdlist: Vec<_> = size_delta_by_type.iter().collect();
        sdlist.sort_by_key(|(_, deltas)| std::cmp::Reverse(deltas.len()));
        for (tc, deltas) in &sdlist {
            let min = deltas.iter().copied().min().unwrap_or(0);
            let max = deltas.iter().copied().max().unwrap_or(0);
            if min == max {
                println!("  type={}: {} objects, all delta={}", tc, deltas.len(), min);
            } else {
                println!("  type={}: {} objects, delta range={}..{}", tc, deltas.len(), min, max);
            }
        }
    }
}

/// Read the BS type code from the beginning of object data.
/// BS = bit-short: first 2 bits determine the encoding.
fn read_type_code(data: &[u8]) -> u16 {
    if data.is_empty() { return 0; }
    let b0 = data[0];
    let flag = b0 >> 6; // top 2 bits
    match flag {
        0b00 => {
            // Full 16-bit value in next 2 bytes (bits 2..17)
            if data.len() < 3 { return 0; }
            let bits = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);
            ((bits >> 6) & 0xFFFF) as u16
        }
        0b01 => {
            // 8-bit value in next byte (bits 2..9)
            if data.len() < 2 { return 0; }
            let bits = ((data[0] as u16) << 8) | (data[1] as u16);
            ((bits >> 6) & 0xFF) as u16
        }
        0b10 => 0, // value is 0
        0b11 => {
            // value is 256 (but wait, in DWG this means something else)
            256
        }
        _ => 0,
    }
}

/// Decode the RL (total_size_bits) field for R2007+ 3-stream format.
/// After the BS type code (10 bits for codes < 256), an RL (32-bit LE)
/// is written at bit_shift=2.
fn decode_rl_r2007(data: &[u8], label: &str) {
    if data.len() < 6 { return; }
    // BS type code uses 10 bits (for type < 256 with bb=01), leaving bit_shift=2 in byte 1.
    // RL is written as 4 LE bytes starting at bit_shift=2.
    // Byte 1 = 0xC0 | (v0 >> 2), Byte 2 = (v0<<6) | (v1>>2), etc.
    let b1 = data[1]; let b2 = data[2]; let b3 = data[3]; let b4 = data[4]; let b5 = data[5];
    // Extract RL bytes
    let v0 = ((b1 & 0x3F) << 2) | (b2 >> 6);
    let v1 = ((b2 & 0x3F) << 2) | (b3 >> 6);
    let v2 = ((b3 & 0x3F) << 2) | (b4 >> 6);
    let v3 = ((b4 & 0x3F) << 2) | (b5 >> 6);
    let rl = (v0 as u32) | ((v1 as u32) << 8) | ((v2 as u32) << 16) | ((v3 as u32) << 24);
    let total_bits = data.len() * 8;
    let handle_bits = total_bits as i64 - rl as i64;
    println!("  {} RL={} (total_size_bits), blob_total_bits={}, handle_region_bits={}",
        label, rl, total_bits, handle_bits);
}
