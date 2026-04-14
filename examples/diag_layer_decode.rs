//! Diagnostic: Decode layer objects bit-by-bit from both original and roundtripped files
//!
//! Reads the actual merged bytes for a specific LAYER handle from both files
//! and decodes each field manually, comparing bit positions.
//!
//! Usage: cargo run --example diag_layer_decode -- <input.dwg>

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;

/// Read ModularShort (2 or 4 bytes) — returns (value, bytes_consumed)
fn read_ms(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset + 1 >= data.len() { return None; }
    let word = u16::from_le_bytes([data[offset], data[offset + 1]]);
    if (word & 0x8000) == 0 {
        Some((word as usize, 2))
    } else {
        if offset + 3 >= data.len() { return None; }
        let word2 = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let val = (word as usize & 0x7FFF) | ((word2 as usize) << 15);
        Some((val, 4))
    }
}

/// Bit-level decoder for DWG objects
struct BitDecoder<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    fn pos(&self) -> usize { self.bit_pos }

    fn read_bit(&mut self) -> bool {
        let byte_idx = self.bit_pos / 8;
        let bit_idx = 7 - (self.bit_pos % 8); // MSB first within byte
        if byte_idx >= self.data.len() { return false; }
        let val = (self.data[byte_idx] >> bit_idx) & 1 != 0;
        self.bit_pos += 1;
        val
    }

    fn read_2bits(&mut self) -> u8 {
        let b1 = self.read_bit() as u8;
        let b0 = self.read_bit() as u8;
        (b1 << 1) | b0
    }

    fn read_byte(&mut self) -> u8 {
        let mut val = 0u8;
        for i in (0..8).rev() {
            if self.read_bit() {
                val |= 1 << i;
            }
        }
        val
    }

    fn read_raw_short(&mut self) -> i16 {
        let lo = self.read_byte();
        let hi = self.read_byte();
        i16::from_le_bytes([lo, hi])
    }

    fn read_raw_long(&mut self) -> i32 {
        let b0 = self.read_byte();
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        let b3 = self.read_byte();
        i32::from_le_bytes([b0, b1, b2, b3])
    }

    fn read_bit_short(&mut self) -> i16 {
        let bb = self.read_2bits();
        match bb {
            0b00 => self.read_raw_short(),
            0b01 => self.read_byte() as i16,
            0b10 => 0,
            0b11 => 256,
            _ => 0,
        }
    }

    fn read_bit_long(&mut self) -> i32 {
        let bb = self.read_2bits();
        match bb {
            0b00 => self.read_raw_long(),
            0b01 => self.read_byte() as i32,
            0b10 => 0,
            _ => 0,
        }
    }

    fn read_handle(&mut self) -> (u8, u8, u64) {
        let header = self.read_byte();
        let code = (header >> 4) & 0x0F;
        let byte_count = header & 0x0F;
        let mut handle = 0u64;
        for _ in 0..byte_count {
            handle = (handle << 8) | self.read_byte() as u64;
        }
        (code, byte_count, handle)
    }

    fn read_variable_text_r2007(&mut self) -> (i16, String) {
        let char_count = self.read_bit_short();
        if char_count <= 0 {
            return (char_count, String::new());
        }
        let mut utf16 = Vec::new();
        for _ in 0..char_count {
            let lo = self.read_byte();
            let hi = self.read_byte();
            utf16.push(u16::from_le_bytes([lo, hi]));
        }
        (char_count, String::from_utf16_lossy(&utf16))
    }
}

fn decode_layer(label: &str, data: &[u8]) {
    println!("\n  === {} ===", label);
    println!("  data_len: {} bytes = {} bits", data.len(), data.len() * 8);

    // For R2007 three-stream merge, the byte layout is:
    // [type_code | RL_placeholder | main_data | text_data | flag | handles]
    // We decode the main stream (which includes RL) from bit 0

    let mut d = BitDecoder::new(data);

    // 1. Type code (BS)
    let p = d.pos();
    let type_code = d.read_bit_short();
    println!("  [{:4}] type_code BS = {} (bits: {})", p, type_code, d.pos() - p);

    // 2. RL placeholder (32 bits raw long) — total_size_bits
    let p = d.pos();
    let rl = d.read_raw_long();
    println!("  [{:4}] RL total_size_bits = {} (bits: 32)", p, rl);

    // 3. Handle (Undefined type) — entity's own handle in main stream
    let p = d.pos();
    let (code, bc, handle) = d.read_handle();
    println!("  [{:4}] handle: code={} byte_count={} value=0x{:X} (bits: {})", p, code, bc, handle, d.pos() - p);

    // 4. EED - BS(size) — 0 means no EED
    let p = d.pos();
    let eed_size = d.read_bit_short();
    println!("  [{:4}] EED size BS = {} (bits: {})", p, eed_size, d.pos() - p);

    if eed_size > 0 {
        println!("  ** EED present — skipping rest of decode **");
        return;
    }

    // After EED, the next fields depend on the object type.
    // For non-entity objects in R2007 three-stream merge:
    // MAIN: reactor count (BL), no-xdic flag (B)
    // HANDLE: owner, reactor handles, xdic handle

    // But the ORDER in the merged stream is:
    // The merged writer writes: main data, then handle data is APPENDED.
    // The ORDER of writes in write_common_non_entity_data is:
    //   HANDLE: owner
    //   MAIN: reactor count (BL)
    //   HANDLE: reactor handles
    //   MAIN: no-xdic flag (B) for R2004+
    //   HANDLE: xdic handle (conditional)
    //   MAIN: R2013+ binary-data flag
    //
    // The merged writer puts ALL main writes in the main sub-stream and ALL
    // handle writes in the handle sub-stream. After merge, main comes first
    // and handles are appended at the end.
    // So in the merged output after EED, the MAIN stream continues with:

    // Reactor count (BL)
    let p = d.pos();
    let reactor_count = d.read_bit_long();
    println!("  [{:4}] reactor_count BL = {} (bits: {})", p, reactor_count, d.pos() - p);

    // No-xdic flag (B) for R2004+
    let p = d.pos();
    let no_xdic = d.read_bit();
    println!("  [{:4}] no_xdic B = {} (bits: 1)", p, no_xdic);

    // Layer-specific MAIN fields:

    // Name (variable text) — in R2007, this goes to TEXT stream, NOT main
    // So the next main field is xref_dependant

    // xref_dependant bits for R2007+: BS(combined)
    let p = d.pos();
    let xref_combined = d.read_bit_short();
    println!("  [{:4}] xref_combined BS = {} (bits: {})", p, xref_combined, d.pos() - p);

    // R2000+: combined values BS (lineweight + flags)
    let p = d.pos();
    let values = d.read_bit_short();
    let lw = (values >> 5) & 0x1F;
    let frozen = (values & 1) != 0;
    let off = (values & 2) != 0;
    let locked = (values & 8) != 0;
    let plottable = (values & 16) != 0;
    println!("  [{:4}] values BS = {} (lw={} frozen={} off={} locked={} plot={}) (bits: {})",
        p, values, lw, frozen, off, locked, plottable, d.pos() - p);

    // CMC color for R2004+: BS(color_index) + BL(rgb) + RC(id)
    let p = d.pos();
    let color_index = d.read_bit_short();
    let ci_bits = d.pos() - p;
    println!("  [{:4}] CMC color_index BS = {} (bits: {})", p, color_index, ci_bits);

    let p2 = d.pos();
    let rgb = d.read_bit_long() as u32;
    println!("  [{:4}] CMC rgb BL = 0x{:08X} (bits: {})", p2, rgb, d.pos() - p2);

    let p3 = d.pos();
    let id = d.read_byte();
    println!("  [{:4}] CMC id RC = {} (bits: 8)", p3, id);

    println!("  [{:4}] main_end_pos (after CMC)", d.pos());
    println!("  Total main stream bits (approx): {}", d.pos());

    // RL should equal: main_bits + text_bits + 1 + flag_words
    // So text_bits = RL - main_bits - 1 (- flag_words)
    let main_bits = d.pos() as i32;
    let text_start_estimate = main_bits;
    println!("  RL={}, main_bits={}, estimated text starts at bit {}", rl, main_bits, text_start_estimate);

    // Try to decode text stream starting at the estimated position
    // Text stream contains: BS(char_count) + UTF-16LE chars
    let p = d.pos();
    let (char_count, name) = d.read_variable_text_r2007();
    println!("  [{:4}] text: char_count={} name='{}' (bits: {})", p, char_count, name, d.pos() - p);

    // If CMC had color name/book in text stream, it would be after the layer name
    // but our writer doesn't write them

    println!("  final bit pos: {}", d.pos());
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("=== Layer field decode comparison ===");
    println!("Input: {}", input);

    // Read original
    let mut reader = DwgReader::from_file(&input).expect("open original");
    let info = reader.read_file_header().expect("read header");
    let orig_objects = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("orig objects");
    let orig_handles_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("orig handles");
    drop(reader);

    let orig_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&orig_handles_buf)
        .expect("parse orig handles");

    // Roundtrip
    let doc = {
        let mut r = DwgReader::from_file(&input).expect("open");
        r.read().expect("read")
    };
    let rt_path = "target/diag_layer_decode_rt.dwg";
    DwgWriter::write_to_file(rt_path, &doc).expect("write RT");

    let mut reader2 = DwgReader::from_file(rt_path).expect("open RT");
    let info2 = reader2.read_file_header().expect("read RT header");
    let rt_objects = reader2.get_section_buffer("AcDb:AcDbObjects", &info2).expect("RT objects");
    let rt_handles_buf = reader2.get_section_buffer("AcDb:Handles", &info2).expect("RT handles");
    drop(reader2);

    let rt_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&rt_handles_buf)
        .expect("parse RT handles");

    // Find first few LAYER handles (type 51)
    // Check handle 0x10 specifically (from comparison output)
    let test_handles: Vec<u64> = vec![0x10, 0x41, 0x42]; // known layer handles

    for &handle in &test_handles {
        println!("\n\n========== HANDLE 0x{:X} ==========", handle);

        if let Some(&orig_off) = orig_handle_map.get(&handle) {
            let off = orig_off as usize;
            if let Some((size, ms_len)) = read_ms(&orig_objects, off) {
                let data = &orig_objects[off + ms_len..off + ms_len + size];
                decode_layer("ORIGINAL", data);
            }
        }

        if let Some(&rt_off) = rt_handle_map.get(&handle) {
            let off = rt_off as usize;
            if let Some((size, ms_len)) = read_ms(&rt_objects, off) {
                let data = &rt_objects[off + ms_len..off + ms_len + size];
                decode_layer("ROUNDTRIP", data);
            }
        }
    }
}
