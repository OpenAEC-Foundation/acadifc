//! Quick diagnostic: compare first bytes of AcDbObjects section between original and RT
//!
//! Usage: cargo run --example diag_first_bytes

use acadrust::io::dwg::DwgReader;

fn main() {
    // Original
    let mut reader = DwgReader::from_file("tests/roundtrip/samplekitchen.dwg").expect("open");
    let info = reader.read_file_header().expect("header");
    let orig_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects");
    
    println!("Original AcDbObjects first 32 bytes:");
    print!("  ");
    for i in 0..32.min(orig_buf.len()) {
        print!("{:02X} ", orig_buf[i]);
    }
    println!();

    // Roundtripped
    let mut reader2 = DwgReader::from_file("target/diag_crc.dwg").expect("open rt");
    let info2 = reader2.read_file_header().expect("header");
    let rt_buf = reader2.get_section_buffer("AcDb:AcDbObjects", &info2).expect("objects");
    
    println!("Roundtripped AcDbObjects first 32 bytes:");
    print!("  ");
    for i in 0..32.min(rt_buf.len()) {
        print!("{:02X} ", rt_buf[i]);
    }
    println!();
    
    // Show the marker interpretation
    let orig_marker = u32::from_le_bytes([orig_buf[0], orig_buf[1], orig_buf[2], orig_buf[3]]);
    let rt_marker = u32::from_le_bytes([rt_buf[0], rt_buf[1], rt_buf[2], rt_buf[3]]);
    println!("\nOriginal marker:     0x{:08X}", orig_marker);
    println!("Roundtripped marker: 0x{:08X}", rt_marker);
    
    // Show original handles section info
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    let hmap = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf).expect("parse");
    let mut offsets: Vec<i64> = hmap.values().copied().collect();
    offsets.sort();
    println!("\nOriginal: first handle offset = {}, total entries = {}", offsets[0], offsets.len());
    
    let handle_buf2 = reader2.get_section_buffer("AcDb:Handles", &info2).expect("handles");
    let hmap2 = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf2).expect("parse");
    let mut offsets2: Vec<i64> = hmap2.values().copied().collect();
    offsets2.sort();
    println!("RT: first handle offset = {}, total entries = {}", offsets2[0], offsets2.len());
    
    println!("\nOriginal buf size: {}", orig_buf.len());
    println!("RT buf size: {}", rt_buf.len());
}
