/// Decode and validate RL (size_in_bits) fields from roundtripped DWG object records.
/// Checks that RL correctly points to the handle stream boundary.
///
/// Usage: cargo run --example diag_record_compare -- path/to/file.dwg

use acadrust::io::dwg::{DwgReader, DwgWriter};
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });

    let bytes = std::fs::read(&path).expect("Failed to read file");
    
    // Read original
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let doc = reader.read().expect("Failed to read DWG");
    
    // Write roundtrip
    let rt_bytes = DwgWriter::write_to_vec(&doc).expect("Failed to write DWG");
    
    // Parse RT handle map + objects section
    let mut rt_reader = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let rt_info = rt_reader.read_file_header().expect("rt header");
    let rt_handles_buf = rt_reader.get_section_buffer("AcDb:Handles", &rt_info).unwrap();
    let rt_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&rt_handles_buf).unwrap();
    let rt_objects = rt_reader.get_section_buffer("AcDb:AcDbObjects", &rt_info).unwrap();
    
    println!("RT objects section: {} bytes", rt_objects.len());
    println!("RT handle entries:  {}", rt_handle_map.len());
    
    // Validate each record: decode BS(type_code) + RL(size_in_bits)
    let mut sorted_handles: Vec<(u64, i64)> = rt_handle_map.iter().map(|(&h, &o)| (h, o)).collect();
    sorted_handles.sort_by_key(|&(_, o)| o);
    
    let mut valid_records = 0u32;
    let mut bad_rl = 0u32;
    let mut parse_errors = 0u32;
    let mut type_counts: std::collections::HashMap<i16, u32> = std::collections::HashMap::new();
    
    for &(handle, offset) in &sorted_handles {
        let offset = offset as usize;
        
        // Read modular short (record size in bytes)
        let (ms_size, data_start) = match read_ms(&rt_objects, offset) {
            Some(v) => v,
            None => { parse_errors += 1; continue; }
        };
        
        if data_start + ms_size > rt_objects.len() {
            parse_errors += 1;
            continue;
        }
        
        let record_data = &rt_objects[data_start..data_start + ms_size];
        let total_bits = ms_size * 8;
        
        // Decode BS(type_code) from the record
        let (type_code, bits_after_type) = match read_bs(record_data, 0) {
            Some(v) => v,
            None => { parse_errors += 1; continue; }
        };
        
        *type_counts.entry(type_code).or_insert(0) += 1;
        
        // Decode RL(size_in_bits) — 32-bit raw long
        let rl = match read_rl(record_data, bits_after_type) {
            Some(v) => v,
            None => { parse_errors += 1; continue; }
        };
        let bits_after_rl = bits_after_type + 32;
        
        // Validate RL: should be <= total_bits (RL = position AFTER which handles start)
        if rl < 0 || rl as usize > total_bits {
            bad_rl += 1;
            if bad_rl <= 10 {
                println!("BAD RL: handle={:#X} type={} rl={} total_bits={} ms_size={}",
                    handle, type_code, rl, total_bits, ms_size);
            }
        } else {
            // Check flag bit at RL-1
            let flag_pos = (rl - 1) as usize;
            if flag_pos < total_bits {
                let flag_bit = read_bit(record_data, flag_pos);
                if flag_bit && rl as usize > bits_after_rl {
                    // Text present — read text_size from flag_pos - 128 bits (16 bytes)
                    let text_size_pos = flag_pos as i64 - 16;
                    if text_size_pos >= bits_after_rl as i64 {
                        // Verify text_size is reasonable
                        if let Some(text_size) = read_raw_ushort(record_data, text_size_pos as usize) {
                            let actual_text_size = text_size & 0x7FFF;
                            if actual_text_size > 0 && (actual_text_size as usize) < total_bits {
                                // Text stream present - verify start position
                                let text_start = (text_size_pos as usize) - actual_text_size as usize;
                                if text_start >= bits_after_rl && text_start < flag_pos {
                                    valid_records += 1;
                                } else {
                                    bad_rl += 1;
                                    if bad_rl <= 10 {
                                        println!("BAD TEXT START: handle={:#X} type={} text_start={} bits_after_rl={}",
                                            handle, type_code, text_start, bits_after_rl);
                                    }
                                }
                            } else {
                                valid_records += 1; // zero-size text is fine
                            }
                        } else {
                            valid_records += 1; // can't read but maybe OK
                        }
                    } else {
                        valid_records += 1; // small record, text info packed tight
                    }
                } else {
                    valid_records += 1; // no text stream
                }
            } else {
                bad_rl += 1;
            }
        }
    }
    
    println!("\n--- Record Validation ---");
    println!("  Valid records:  {}", valid_records);
    println!("  Bad RL:         {}", bad_rl);
    println!("  Parse errors:   {}", parse_errors);
    
    println!("\n--- Type distribution (top 20) ---");
    let mut types: Vec<(i16, u32)> = type_counts.into_iter().collect();
    types.sort_by(|a, b| b.1.cmp(&a.1));
    for (tc, count) in types.iter().take(20) {
        let name = type_name(*tc);
        println!("  type {:3} ({:20}): {}", tc, name, count);
    }
}

fn type_name(tc: i16) -> &'static str {
    match tc {
        1 => "TEXT", 2 => "ATTRIB", 3 => "ATTDEF", 4 => "BLOCK",
        5 => "ENDBLK", 6 => "SEQEND", 7 => "INSERT", 13 => "VERTEX_PFACE",
        14 => "VERTEX_PFACE_FACE", 17 => "ARC", 18 => "CIRCLE",
        19 => "LINE", 27 => "POINT", 28 => "3DFACE", 29 => "POLYLINE_PFACE",
        31 => "SOLID", 34 => "VIEWPORT", 35 => "ELLIPSE",
        38 => "3DSOLID", 42 => "DICTIONARY", 44 => "MTEXT",
        45 => "LEADER", 46 => "TOLERANCE", 48 => "BLOCK_CONTROL",
        49 => "BLOCK_HEADER", 50 => "LAYER_CONTROL", 51 => "LAYER",
        52 => "STYLE_CONTROL", 53 => "STYLE", 56 => "LTYPE_CONTROL",
        57 => "LTYPE", 60 => "VIEW_CONTROL", 61 => "VIEW",
        62 => "UCS_CONTROL", 63 => "UCS", 64 => "VPORT_CONTROL",
        65 => "VPORT", 66 => "APPID_CONTROL", 67 => "APPID",
        68 => "DIMSTYLE_CONTROL", 69 => "DIMSTYLE", 70 => "VPENT_CONTROL",
        71 => "VPENT_HDR", 72 => "GROUP", 73 => "MLINESTYLE",
        77 => "LWPOLYLINE", 78 => "HATCH", 79 => "XRECORD",
        80 => "PLACEHOLDER", 82 => "LAYOUT",
        _ => "UNKNOWN/CLASS"
    }
}

fn read_ms(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    let mut pos = offset;
    let mut size: usize = 0;
    let mut shift = 0;
    loop {
        if pos + 1 >= data.len() { return None; }
        let word = data[pos] as usize | ((data[pos+1] as usize) << 8);
        pos += 2;
        size |= (word & 0x7FFF) << shift;
        shift += 15;
        if (word & 0x8000) == 0 { break; }
    }
    Some((size, pos))
}

fn read_bit(data: &[u8], bit_pos: usize) -> bool {
    let byte_idx = bit_pos / 8;
    let bit_idx = 7 - (bit_pos % 8); // MSB first
    if byte_idx >= data.len() { return false; }
    (data[byte_idx] >> bit_idx) & 1 != 0
}

fn read_bs(data: &[u8], bit_pos: usize) -> Option<(i16, usize)> {
    let b0 = read_bit(data, bit_pos);
    let b1 = read_bit(data, bit_pos + 1);
    match (b0, b1) {
        (false, false) => {
            // 00 → 16-bit value (LE)
            let lo = read_byte(data, bit_pos + 2)?;
            let hi = read_byte(data, bit_pos + 10)?;
            Some(((lo as i16 | ((hi as i16) << 8)), bit_pos + 18))
        }
        (false, true) => {
            // 01 → 8-bit value
            let v = read_byte(data, bit_pos + 2)?;
            Some((v as i16, bit_pos + 10))
        }
        (true, false) => {
            // 10 → value = 0
            Some((0, bit_pos + 2))
        }
        (true, true) => {
            // 11 → value = 256 (for R2004+, not standard BS)
            Some((256, bit_pos + 2))
        }
    }
}

fn read_byte(data: &[u8], bit_pos: usize) -> Option<u8> {
    let mut val = 0u8;
    for i in 0..8 {
        if read_bit(data, bit_pos + i) {
            val |= 1 << (7 - i);
        }
    }
    Some(val)
}

fn read_rl(data: &[u8], bit_pos: usize) -> Option<i32> {
    // RL = Raw Long = 4 bytes LE
    let b0 = read_byte(data, bit_pos)? as i32;
    let b1 = read_byte(data, bit_pos + 8)? as i32;
    let b2 = read_byte(data, bit_pos + 16)? as i32;
    let b3 = read_byte(data, bit_pos + 24)? as i32;
    Some(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
}

fn read_raw_ushort(data: &[u8], bit_pos: usize) -> Option<u16> {
    let lo = read_byte(data, bit_pos)? as u16;
    let hi = read_byte(data, bit_pos + 8)? as u16;
    Some(lo | (hi << 8))
}
