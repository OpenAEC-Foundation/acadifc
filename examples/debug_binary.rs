/// Direct binary comparison: encode two LwPolylines (one that would pass, one fail)
/// and compare byte-by-byte.
use acadrust::document::CadDocument;
use acadrust::entities::{EntityType, lwpolyline::{LwPolyline, LwVertex}};
use acadrust::io::dwg::dwg_stream_writers::object_writer::DwgObjectWriter;
use acadrust::types::{DxfVersion, Handle, Vector2, Vector3};

fn main() {
    // Create a document with AC1024 (R2010) version
    let mut doc = CadDocument::new();
    doc.version = DxfVersion::AC1024;

    // Create a "passing" LwPolyline (open, 3 verts, Rgb color)
    let mut lw_pass = LwPolyline::new();
    lw_pass.is_closed = false;
    lw_pass.vertices = vec![
        LwVertex { location: Vector2::new(0.0, 0.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
        LwVertex { location: Vector2::new(10.0, 0.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
        LwVertex { location: Vector2::new(10.0, 10.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
    ];
    lw_pass.common.handle = Handle::new(0x100);
    let _ = doc.add_entity(EntityType::LwPolyline(lw_pass));

    // Create a "failing" LwPolyline (closed, 4 verts, ByLayer color - like the failing ones in General.dwg)
    let mut lw_fail = LwPolyline::new();
    lw_fail.is_closed = true;
    lw_fail.vertices = vec![
        LwVertex { location: Vector2::new(14123.0, 3437.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
        LwVertex { location: Vector2::new(14156.0, 3437.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
        LwVertex { location: Vector2::new(14156.0, 3404.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
        LwVertex { location: Vector2::new(14123.0, 3404.0), bulge: 0.0, start_width: 0.0, end_width: 0.0 },
    ];
    lw_fail.common.handle = Handle::new(0x200);
    let _ = doc.add_entity(EntityType::LwPolyline(lw_fail));

    // Write using object writer
    let obj_writer = DwgObjectWriter::new(&doc).expect("create writer");
    let (obj_data, handle_map, _extents, _sab) = obj_writer.write();

    println!("Objects section: {} bytes", obj_data.len());
    println!("Handle map: {:?}", handle_map);

    // Dump bytes for each entity
    for &(handle, offset) in &handle_map {
        // Read MS
        let data = &obj_data[offset as usize..];
        let (size, ms_len) = read_modular_short(data);
        let (handle_bits, mc_len) = read_modular_char(&data[ms_len..]);
        let header_len = ms_len + mc_len;
        let merged = &data[header_len..header_len + size];

        // CRC
        let crc_bytes = &obj_data[offset as usize + header_len + size..offset as usize + header_len + size + 2];
        let stored_crc = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
        let record = &obj_data[offset as usize..offset as usize + header_len + size];
        let computed_crc = acadrust::io::dwg::crc::crc16(acadrust::io::dwg::crc::CRC16_SEED, record);

        println!("\n=== Handle {:#X} ===", handle);
        println!("Offset: {}, Size: {} bytes, Handle bits: {}", offset, size, handle_bits);
        println!("CRC: stored={:#06X}, computed={:#06X}, match={}", stored_crc, computed_crc, stored_crc == computed_crc);
        println!("Total record: {} bytes (MS:{} + MC:{} + data:{} + CRC:2)", header_len + size + 2, ms_len, mc_len, size);

        // Hex dump of merged data
        println!("Merged data ({} bytes):", size);
        for (i, chunk) in merged.chunks(16).enumerate() {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
            let ascii: String = chunk.iter().map(|&b| if b >= 0x20 && b < 0x7F { b as char } else { '.' }).collect();
            println!("  {:04X}: {}  {}", i * 16, hex.join(" "), ascii);
        }
    }
}

fn read_modular_short(data: &[u8]) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos + 1 >= data.len() { return (0, pos); }
        let word = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        result |= ((word & 0x7FFF) as usize) << shift;
        shift += 15;
        if word & 0x8000 == 0 { break; }
    }
    (result, pos)
}

fn read_modular_char(data: &[u8]) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos >= data.len() { return (0, pos); }
        let b = data[pos];
        pos += 1;
        result |= ((b & 0x7F) as usize) << shift;
        shift += 7;
        if b & 0x80 == 0 { break; }
    }
    (result, pos)
}
