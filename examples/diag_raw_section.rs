use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::dwg_stream_readers::object_reader::DwgObjectReader;
use acadrust::io::dwg::dwg_stream_readers::object_reader::common::*;
use std::collections::HashMap;
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());
    let mut reader = DwgReader::from_file(&path).expect("open");
    let info = reader.read_file_header().expect("header");
    println!("Original AcDbObjects: {} bytes", info.section_descriptors.iter().find(|s| s.name == "AcDb:AcDbObjects").map(|s| s.compressed_size).unwrap_or(0));
    let handles_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handles_buf).expect("handles");
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects");
    println!("Decompressed: {} bytes, handles: {}", objects_buf.len(), handle_map.len());
}
