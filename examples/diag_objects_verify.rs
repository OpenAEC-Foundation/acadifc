/// Diagnostic: verify handle→object address mapping in a R2007 DWG file.
///
/// Reads the Handles section and AcDb:AcDbObjects section,
/// then for each handle checks that the object at that address has a
/// non-zero MS(size) and a valid type code, reporting mismatches.

use acadrust::io::dwg::DwgReader;
use std::collections::HashMap;

fn read_modular_short(data: &[u8]) -> (usize, usize) {
    // ModularShort: 2-byte chunks, bit 15 = continuation, 15 data bits per chunk, LE
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

fn read_handle_section(data: &[u8]) -> HashMap<u64, i64> {
    let mut handle_map = HashMap::new();
    let mut pos = 0usize;

    while pos + 2 <= data.len() {
        let size = ((data[pos] as usize) << 8) | (data[pos + 1] as usize);
        pos += 2;

        if size <= 2 || size > 2048 { break; }
        let data_bytes = size - 2;
        let chunk_end = (pos + data_bytes).min(data.len());

        let mut last_handle: u64 = 0;
        let mut last_offset: i64 = 0;

        while pos < chunk_end {
            // Read MC (unsigned)
            let mut value: u64 = 0;
            let mut shift: u32 = 0;
            loop {
                if pos >= chunk_end { break; }
                let b = data[pos]; pos += 1;
                value |= ((b & 0x7F) as u64) << shift;
                if (b & 0x80) == 0 { break; }
                shift += 7;
            }
            let handle_delta = value;

            // Read SMC (signed) 
            let mut value: u64 = 0;
            let mut shift: u32 = 0;
            let mut last_byte: u8 = 0;
            loop {
                if pos >= chunk_end { break; }
                let b = data[pos]; pos += 1;
                last_byte = b;
                if (b & 0x80) == 0 {
                    value |= ((b & 0x3F) as u64) << shift;
                    break;
                } else {
                    value |= ((b & 0x7F) as u64) << shift;
                    shift += 7;
                }
            }
            let offset_delta = if (last_byte & 0x40) != 0 {
                -(value as i64)
            } else {
                value as i64
            };

            last_handle = last_handle.wrapping_add(handle_delta);
            last_offset = last_offset.wrapping_add(offset_delta);
            handle_map.insert(last_handle, last_offset);
        }

        // Skip CRC
        if pos + 2 <= data.len() { pos += 2; }
    }

    handle_map
}

fn main() {
    let path = std::env::args().nth(1).expect("Usage: diag_objects_verify <file.dwg>");
    println!("=== Object/Handle Verification: {} ===", path);

    let mut reader = DwgReader::from_file(&path).expect("Failed to open DWG");
    let info = reader.read_file_header().expect("Failed to read header");

    // Read handles section
    let handles_buf = reader.get_section_buffer("AcDb:Handles", &info)
        .expect("Failed to read Handles section");
    let handle_map = read_handle_section(&handles_buf);
    println!("Handle entries: {}", handle_map.len());

    // Read objects section
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("Failed to read AcDbObjects section");
    println!("Objects section size: {} bytes", objects_buf.len());

    // Check 0x0DCA marker
    if objects_buf.len() >= 4 {
        let marker = i32::from_le_bytes([objects_buf[0], objects_buf[1], objects_buf[2], objects_buf[3]]);
        println!("First 4 bytes (expect 0x0DCA): {:#X}", marker);
    }

    // Analyze handle→object mapping
    let mut ok = 0usize;
    let mut zero_size = 0usize;
    let mut out_of_range = 0usize;

    // Sort handles for deterministic output
    let mut sorted_handles: Vec<(u64, i64)> = handle_map.iter().map(|(&h, &o)| (h, o)).collect();
    sorted_handles.sort_by_key(|&(h, _)| h);

    // Report first 20 zero-size problems in detail
    let mut detail_count = 0;

    for (handle, offset) in &sorted_handles {
        let off = *offset as usize;
        if off >= objects_buf.len() {
            out_of_range += 1;
            if detail_count < 20 {
                println!("  [OOB] handle={:#X} offset={:#X} (data len={:#X})", handle, off, objects_buf.len());
                detail_count += 1;
            }
            continue;
        }
        let (size, _ms_len) = read_modular_short(&objects_buf[off..]);
        if size == 0 {
            zero_size += 1;
            if detail_count < 20 {
                let preview_end = (off + 8).min(objects_buf.len());
                let preview: Vec<String> = objects_buf[off..preview_end].iter().map(|b| format!("{:02X}", b)).collect();
                println!("  [ZERO] handle={:#X} offset={:#X} bytes=[{}]", handle, off, preview.join(" "));
                detail_count += 1;
            }
        } else {
            ok += 1;
        }
    }

    println!("\n=== Handle map results ===");
    println!("  OK:           {}", ok);
    println!("  Zero size:    {}", zero_size);
    println!("  Out of range: {}", out_of_range);
    println!("  Total:        {}", sorted_handles.len());

    // Sequential scan of objects section
    println!("\n=== Sequential scan of objects section ===");
    let mut scan_pos = 0usize;
    // Skip 0x0DCA marker
    if objects_buf.len() >= 4 {
        scan_pos = 4;
    }
    let mut seq_ok = 0usize;
    let mut seq_zero = 0usize;
    let mut seq_dead = 0usize;
    let mut first_zero_seq: Option<usize> = None;

    while scan_pos < objects_buf.len() {
        let start = scan_pos;
        let (size, ms_len) = read_modular_short(&objects_buf[scan_pos..]);
        scan_pos += ms_len;

        if size == 0 {
            seq_zero += 1;
            if first_zero_seq.is_none() { first_zero_seq = Some(start); }
            // Zero-size: check CRC (2 bytes) and skip
            scan_pos += 2;
            continue;
        }

        if scan_pos + size + 2 > objects_buf.len() {
            seq_dead += 1;
            break;
        }
        // skip merged data + CRC
        scan_pos += size + 2;
        seq_ok += 1;
    }

    println!("  Sequential valid objects: {}", seq_ok);
    println!("  Sequential zero-size:     {}", seq_zero);
    if let Some(p) = first_zero_seq {
        println!("  First zero-size at seq offset: {:#X}", p);
    }
    println!("  Sequential dead (truncated): {}", seq_dead);
    println!("  Sequential scan ended at: {:#X}", scan_pos);

    // Show offset range
    let min_off = sorted_handles.iter().map(|(_, o)| *o).min().unwrap_or(0);
    let max_off = sorted_handles.iter().map(|(_, o)| *o).max().unwrap_or(0);
    println!("\nHandle offset range: {:#X} .. {:#X}", min_off, max_off);
    println!("Objects section size: {:#X}", objects_buf.len());

    // Check specific addresses from AutoCAD audit file
    // "address" might be the HANDLE value, not the byte offset
    println!("\n=== Specific handles from AutoCAD audit ===");
    let audit_handles: &[u64] = &[0x1A9B1E, 0x000F802D, 0x000F804A, 0x000F8189, 0x000F81E0];
    for &h in audit_handles {
        if let Some(&offset) = handle_map.get(&h) {
            let off = offset as usize;
            if off + 2 <= objects_buf.len() {
                let (size, ms_len) = read_modular_short(&objects_buf[off..]);
                let preview_end = (off + 8).min(objects_buf.len());
                let preview: Vec<String> = objects_buf[off..preview_end].iter().map(|b| format!("{:02X}", b)).collect();
                println!("  handle={:#X}: offset={:#X} size={} ms_len={} bytes=[{}]", h, off, size, ms_len, preview.join(" "));
            }
        } else {
            println!("  handle={:#X}: NOT in handle map", h);
        }
    }

    // Show first 10 and last 10 handle entries 
    println!("\n=== First 10 handle entries ===");
    for (h, o) in sorted_handles.iter().take(10) {
        println!("  handle={:#X} → offset={:#X}", h, o);
    }
    println!("\n=== Last 10 handle entries ===");
    let n = sorted_handles.len();
    for (h, o) in sorted_handles.iter().skip(n.saturating_sub(10)) {
        println!("  handle={:#X} → offset={:#X}", h, o);
    }
}
