/// Byte-level comparison of a specific object record between original and roundtrip DWG.
/// Usage: cargo run --example diag_byte_compare -- orig.dwg handle_hex

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;
use std::io::Cursor;

fn read_ms(buf: &[u8], offset: usize) -> Option<(usize, usize)> {
    let mut pos = offset;
    let mut size: usize = 0;
    let mut shift = 0;
    loop {
        if pos >= buf.len() { return None; }
        let b = buf[pos] as usize;
        pos += 1;
        size |= (b & 0x7F) << shift;
        if b & 0x80 == 0 { break; }
        shift += 7;
    }
    Some((size, pos))
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });
    let handle_str = std::env::args().nth(2).unwrap_or_else(|| "0xA761".to_string());
    let target_handle = u64::from_str_radix(handle_str.trim_start_matches("0x"), 16).unwrap();

    let bytes = std::fs::read(&path).expect("read file");
    
    // Read original
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let orig_info = reader.read_file_header().expect("orig header");
    let orig_handles_buf = reader.get_section_buffer("AcDb:Handles", &orig_info).unwrap();
    let orig_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&orig_handles_buf).unwrap();
    let orig_objects = reader.get_section_buffer("AcDb:AcDbObjects", &orig_info).unwrap();

    // Write roundtrip
    let mut reader2 = DwgReader::from_stream(Cursor::new(&bytes));
    let doc = reader2.read().expect("Parse DWG");
    let rt_bytes = DwgWriter::write_to_vec(&doc).expect("write RT");
    
    let mut rt_reader = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let rt_info = rt_reader.read_file_header().expect("rt header");
    let rt_handles_buf = rt_reader.get_section_buffer("AcDb:Handles", &rt_info).unwrap();
    let rt_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&rt_handles_buf).unwrap();
    let rt_objects = rt_reader.get_section_buffer("AcDb:AcDbObjects", &rt_info).unwrap();

    // Extract record for target handle
    let orig_offset = orig_handle_map.get(&target_handle).expect("handle not in orig");
    let rt_offset = rt_handle_map.get(&target_handle).expect("handle not in RT");

    let (orig_ms_size, orig_data_start) = read_ms(&orig_objects, *orig_offset as usize).unwrap();
    let (rt_ms_size, rt_data_start) = read_ms(&rt_objects, *rt_offset as usize).unwrap();

    let orig_record = &orig_objects[orig_data_start..orig_data_start + orig_ms_size];
    let rt_record = &rt_objects[rt_data_start..rt_data_start + rt_ms_size];

    println!("Handle: 0x{:X}", target_handle);
    println!("Original: offset={}, ms_size={} bytes", orig_offset, orig_ms_size);
    println!("RT:       offset={}, ms_size={} bytes", rt_offset, rt_ms_size);
    println!();

    // Dump both records in hex, highlighting differences
    let max_len = orig_ms_size.max(rt_ms_size);
    println!("OFF  ORIG   RT     DIFF");
    println!("---  ----   --     ----");
    for i in 0..max_len {
        let orig_byte = if i < orig_record.len() { Some(orig_record[i]) } else { None };
        let rt_byte = if i < rt_record.len() { Some(rt_record[i]) } else { None };
        
        let diff = match (orig_byte, rt_byte) {
            (Some(a), Some(b)) if a != b => format!("<< DIFF (orig {:08b} vs rt {:08b})", a, b),
            (Some(_), None) => "<< ORIG ONLY".to_string(),
            (None, Some(_)) => "<< RT ONLY".to_string(),
            _ => String::new(),
        };
        
        let ob = orig_byte.map(|b| format!("0x{:02X}", b)).unwrap_or_else(|| "----".to_string());
        let rb = rt_byte.map(|b| format!("0x{:02X}", b)).unwrap_or_else(|| "----".to_string());
        
        if !diff.is_empty() || i < 10 || i >= max_len.saturating_sub(5) {
            println!("{:3}  {}  {}  {}", i, ob, rb, diff);
        }
    }

    // Also dump as bit strings for the first bytes where they mismatch
    println!("\n=== BIT-LEVEL COMPARISON (first 80 bits) ===");
    let orig_bits: Vec<u8> = orig_record.iter()
        .flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1))
        .collect();
    let rt_bits: Vec<u8> = rt_record.iter()
        .flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1))
        .collect();
    
    // Find first bit difference
    let mut first_diff_bit = None;
    for i in 0..orig_bits.len().min(rt_bits.len()) {
        if orig_bits[i] != rt_bits[i] {
            first_diff_bit = Some(i);
            break;
        }
    }
    
    if let Some(fdb) = first_diff_bit {
        println!("First bit difference at bit {}", fdb);
        let start = if fdb > 16 { fdb - 16 } else { 0 };
        let end = (fdb + 48).min(orig_bits.len()).min(rt_bits.len());
        
        print!("ORIG bits[{}..{}]: ", start, end);
        for i in start..end {
            if i == fdb { print!("["); }
            print!("{}", if i < orig_bits.len() { orig_bits[i].to_string() } else { "-".to_string() });
            if i == fdb { print!("]"); }
        }
        println!();
        
        print!("RT   bits[{}..{}]: ", start, end);
        for i in start..end {
            if i == fdb { print!("["); }
            print!("{}", if i < rt_bits.len() { rt_bits[i].to_string() } else { "-".to_string() });
            if i == fdb { print!("]"); }
        }
        println!();
        
        // Check if RT is just orig with N bits removed
        // Try shifting RT by 1-16 bits and see if it aligns
        println!("\n=== SHIFT ANALYSIS ===");
        for shift in 1..=16 {
            let mut match_count = 0;
            let check_len = orig_bits.len().min(rt_bits.len() + shift).saturating_sub(shift);
            for i in fdb..fdb + check_len.min(200) {
                if i + shift < orig_bits.len() && i < rt_bits.len() && orig_bits[i + shift] == rt_bits[i] {
                    match_count += 1;
                }
            }
            let total = check_len.min(200);
            if total > 0 && match_count * 100 / total > 90 {
                println!("  SHIFT +{}: orig_bits[fdb+{}..] matches rt_bits[fdb..] ({}% of {} bits)", 
                    shift, shift, match_count * 100 / total, total);
            }
        }
        // Try: RT has extra bits inserted
        for shift in 1..=16 {
            let mut match_count = 0;
            let check_len = rt_bits.len().min(orig_bits.len() + shift).saturating_sub(shift);
            for i in fdb..fdb + check_len.min(200) {
                if i < orig_bits.len() && i + shift < rt_bits.len() && orig_bits[i] == rt_bits[i + shift] {
                    match_count += 1;
                }
            }
            let total = check_len.min(200);
            if total > 0 && match_count * 100 / total > 90 {
                println!("  SHIFT -{}: rt_bits[fdb+{}..] matches orig_bits[fdb..] ({}% of {} bits)", 
                    shift, shift, match_count * 100 / total, total);
            }
        }
    } else if orig_bits.len() != rt_bits.len() {
        println!("No bit difference in common region, but sizes differ: orig {} bits, rt {} bits", orig_bits.len(), rt_bits.len());
    } else {
        println!("Records are identical!");
    }
}
