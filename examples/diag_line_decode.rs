//! Diagnostic: Decode LINE entity fields bit-by-bit from both original and roundtripped files
//!
//! Usage: cargo run --example diag_line_decode -- <input.dwg>

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;

/// Read ModularShort
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

struct BitDecoder<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitDecoder<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, bit_pos: 0 } }
    fn pos(&self) -> usize { self.bit_pos }

    fn read_bit(&mut self) -> bool {
        let byte_idx = self.bit_pos / 8;
        let bit_idx = 7 - (self.bit_pos % 8);
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
            if self.read_bit() { val |= 1 << i; }
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

    fn read_raw_double(&mut self) -> f64 {
        let mut bytes = [0u8; 8];
        for b in &mut bytes { *b = self.read_byte(); }
        f64::from_le_bytes(bytes)
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

    fn read_bit_double(&mut self) -> (f64, u8) {
        let bb = self.read_2bits();
        match bb {
            0b00 => (self.read_raw_double(), bb),
            0b01 => (1.0, bb),
            0b10 => (0.0, bb),
            _ => (0.0, bb), // shouldn't happen
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

    fn read_3bit_double(&mut self) -> (Vector3, u8, u8, u8) {
        let (x, bbx) = self.read_bit_double();
        let (y, bby) = self.read_bit_double();
        let (z, bbz) = self.read_bit_double();
        (Vector3 { x, y, z }, bbx, bby, bbz)
    }
}

#[derive(Debug)]
struct Vector3 { x: f64, y: f64, z: f64 }

fn decode_line(label: &str, data: &[u8]) {
    println!("\n  === {} ===", label);
    println!("  data_len: {} bytes = {} bits", data.len(), data.len() * 8);

    let mut d = BitDecoder::new(data);

    // Type code (BS)
    let p = d.pos();
    let type_code = d.read_bit_short();
    println!("  [{:4}] type_code BS = {} (bits: {})", p, type_code, d.pos() - p);

    // RL placeholder
    let p = d.pos();
    let rl = d.read_raw_long();
    println!("  [{:4}] RL total_size_bits = {} (bits: 32)", p, rl);

    // Handle
    let p = d.pos();
    let (code, bc, handle) = d.read_handle();
    println!("  [{:4}] handle: code={} bc={} value=0x{:X} (bits: {})", p, code, bc, handle, d.pos() - p);

    // EED
    let p = d.pos();
    let eed_size = d.read_bit_short();
    println!("  [{:4}] EED size BS = {} (bits: {})", p, eed_size, d.pos() - p);
    if eed_size > 0 {
        println!("  ** EED present — skipping **");
        return;
    }

    // Graphic presence flag
    let p = d.pos();
    let has_graphic = d.read_bit();
    println!("  [{:4}] has_graphic B = {} (bits: 1)", p, has_graphic);

    // Entity mode (2 bits)
    let p = d.pos();
    let ent_mode = d.read_2bits();
    println!("  [{:4}] entity_mode BB = {} (bits: 2)", p, ent_mode);

    // If entmode==0, owner handle follows (in handle stream — but merged)
    if ent_mode == 0 {
        let p = d.pos();
        let (c, bc, h) = d.read_handle();
        println!("  [{:4}] owner handle: code={} bc={} val=0x{:X} (bits: {})", p, c, bc, h, d.pos() - p);
    }

    // Reactor count (BL)
    let p = d.pos();
    let reactor_count = d.read_bit_long();
    println!("  [{:4}] reactor_count BL = {} (bits: {})", p, reactor_count, d.pos() - p);

    // R2004+: no-xdic flag
    let p = d.pos();
    let no_xdic = d.read_bit();
    println!("  [{:4}] no_xdic B = {} (bits: 1)", p, no_xdic);

    // R2013+: binary data flag → skip for now, assume not present

    // ENC color: BS(flags_and_color)
    let p = d.pos();
    let enc_bs = d.read_bit_short();
    let enc_bits = d.pos() - p;
    let enc_flags = (enc_bs as u16) & 0xFF00;
    let enc_index = enc_bs & 0x0FFF;
    println!("  [{:4}] ENC color BS = {} (0x{:04X}) flags=0x{:04X} index={} (bits: {})",
        p, enc_bs, enc_bs as u16, enc_flags, enc_index, enc_bits);

    if (enc_flags & 0x8000) != 0 {
        let p2 = d.pos();
        let rgb = d.read_bit_long() as u32;
        println!("  [{:4}] ENC true-color BL = 0x{:08X} (bits: {})", p2, rgb, d.pos() - p2);
    }
    if (enc_flags & 0x2000) != 0 {
        let p2 = d.pos();
        let tr = d.read_bit_long() as u32;
        println!("  [{:4}] ENC transparency BL = 0x{:08X} (bits: {})", p2, tr, d.pos() - p2);
    }

    // Linetype scale (BD)
    let p = d.pos();
    let (lt_scale, lt_bb) = d.read_bit_double();
    println!("  [{:4}] linetype_scale BD = {} (bb={}) (bits: {})", p, lt_scale, lt_bb, d.pos() - p);

    // Layer handle
    let p = d.pos();
    let (c, bc, h) = d.read_handle();
    println!("  [{:4}] layer handle: code={} bc={} val=0x{:X} (bits: {})", p, c, bc, h, d.pos() - p);

    // Linetype flags (2 bits)
    let p = d.pos();
    let lt_flags = d.read_2bits();
    println!("  [{:4}] linetype_flags BB = {} (bits: 2)", p, lt_flags);

    if lt_flags == 0b11 {
        let p2 = d.pos();
        let (c, bc, h) = d.read_handle();
        println!("  [{:4}] linetype handle: code={} bc={} val=0x{:X} (bits: {})", p2, c, bc, h, d.pos() - p2);
    }

    // R2007+: material flags + shadow flags
    let p = d.pos();
    let mat_flags = d.read_2bits();
    println!("  [{:4}] material_flags BB = {} (bits: 2)", p, mat_flags);

    if mat_flags == 0b11 {
        let p2 = d.pos();
        let (c, bc, h) = d.read_handle();
        println!("  [{:4}] material handle: code={} bc={} val=0x{:X} (bits: {})", p2, c, bc, h, d.pos() - p2);
    }

    let p = d.pos();
    let shadow = d.read_byte();
    println!("  [{:4}] shadow_flags RC = {} (bits: 8)", p, shadow);

    // Plotstyle flags
    let p = d.pos();
    let ps_flags = d.read_2bits();
    println!("  [{:4}] plotstyle_flags BB = {} (bits: 2)", p, ps_flags);

    if ps_flags == 0b11 {
        let p2 = d.pos();
        let (c, bc, h) = d.read_handle();
        println!("  [{:4}] plotstyle handle: code={} bc={} val=0x{:X} (bits: {})", p2, c, bc, h, d.pos() - p2);
    }

    // Invisibility
    let p = d.pos();
    let invis = d.read_bit_short();
    println!("  [{:4}] invisibility BS = {} (bits: {})", p, invis, d.pos() - p);

    // Lineweight
    let p = d.pos();
    let lw = d.read_byte();
    println!("  [{:4}] lineweight RC = {} (bits: 8)", p, lw);

    println!("  [{:4}] === END common entity data ===", d.pos());

    // LINE-specific data:
    // R2000+: z_are_zero flag (B)
    let p = d.pos();
    let z_are_zero = d.read_bit();
    println!("  [{:4}] z_are_zero B = {} (bits: 1)", p, z_are_zero);

    // start.x (RD), end.x (DD default=start.x), start.y (RD), end.y (DD default=start.y)
    let p = d.pos();
    let sx = d.read_raw_double();
    println!("  [{:4}] start.x RD = {} (bits: 64)", p, sx);

    // end.x (BD with default = start.x)
    let p = d.pos();
    let bb_ex = d.read_2bits();
    let ex = match bb_ex {
        0 => sx,
        1 => { let mut bytes = [0u8; 8]; let sx_bytes = sx.to_le_bytes(); bytes[4..8].copy_from_slice(&sx_bytes[4..8]); for i in 0..4 { bytes[i] = d.read_byte(); } f64::from_le_bytes(bytes) },
        2 => { let mut bytes = [0u8; 8]; let sx_bytes = sx.to_le_bytes(); bytes[6..8].copy_from_slice(&sx_bytes[6..8]); bytes[4] = d.read_byte(); bytes[5] = d.read_byte(); for i in 0..4 { bytes[i] = d.read_byte(); } f64::from_le_bytes(bytes) },
        _ => d.read_raw_double(),
    };
    println!("  [{:4}] end.x DD(start.x) bb={} val={} (bits: {})", p, bb_ex, ex, d.pos() - p);

    let p = d.pos();
    let sy = d.read_raw_double();
    println!("  [{:4}] start.y RD = {} (bits: 64)", p, sy);

    let p = d.pos();
    let bb_ey = d.read_2bits();
    let ey = match bb_ey {
        0 => sy,
        1 => { let mut bytes = [0u8; 8]; let sy_bytes = sy.to_le_bytes(); bytes[4..8].copy_from_slice(&sy_bytes[4..8]); for i in 0..4 { bytes[i] = d.read_byte(); } f64::from_le_bytes(bytes) },
        2 => { let mut bytes = [0u8; 8]; let sy_bytes = sy.to_le_bytes(); bytes[6..8].copy_from_slice(&sy_bytes[6..8]); bytes[4] = d.read_byte(); bytes[5] = d.read_byte(); for i in 0..4 { bytes[i] = d.read_byte(); } f64::from_le_bytes(bytes) },
        _ => d.read_raw_double(),
    };
    println!("  [{:4}] end.y DD(start.y) bb={} val={} (bits: {})", p, bb_ey, ey, d.pos() - p);

    if !z_are_zero {
        let p = d.pos();
        let sz = d.read_raw_double();
        println!("  [{:4}] start.z RD = {} (bits: 64)", p, sz);

        let p = d.pos();
        let bb_ez = d.read_2bits();
        println!("  [{:4}] end.z DD(start.z) bb={} (bits: {})", p, bb_ez, d.pos() - p);
    }

    // Thickness (BT: B + BD)
    let p = d.pos();
    let thick_zero = d.read_bit();
    if thick_zero {
        println!("  [{:4}] thickness BT = 0.0 (bits: 1)", p);
    } else {
        let (th, bb) = d.read_bit_double();
        println!("  [{:4}] thickness BT = {} bb={} (bits: {})", p, th, bb, d.pos() - p);
    }

    // Extrusion (BE: B + 3BD)
    let p = d.pos();
    let ext_default = d.read_bit();
    if ext_default {
        println!("  [{:4}] extrusion BE = (0,0,1) default (bits: 1)", p);
    } else {
        let (ext, bx, by, bz) = d.read_3bit_double();
        println!("  [{:4}] extrusion BE = ({},{},{}) bb=({},{},{}) (bits: {})", p, ext.x, ext.y, ext.z, bx, by, bz, d.pos() - p);
    }

    println!("  [{:4}] === END LINE data ===", d.pos());
    println!("  total_bits_consumed: {}", d.pos());
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    let mut reader = DwgReader::from_file(&input).expect("open original");
    let info = reader.read_file_header().expect("read header");
    let orig_objects = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("orig objects");
    let orig_handles_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("orig handles");
    drop(reader);

    let orig_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&orig_handles_buf)
        .expect("parse orig handles");

    let doc = {
        let mut r = DwgReader::from_file(&input).expect("open");
        r.read().expect("read")
    };
    let rt_path = "target/diag_line_decode_rt.dwg";
    DwgWriter::write_to_file(rt_path, &doc).expect("write RT");

    let mut reader2 = DwgReader::from_file(rt_path).expect("open RT");
    let info2 = reader2.read_file_header().expect("read RT header");
    let rt_objects = reader2.get_section_buffer("AcDb:AcDbObjects", &info2).expect("RT objects");
    let rt_handles_buf = reader2.get_section_buffer("AcDb:Handles", &info2).expect("RT handles");
    drop(reader2);

    let rt_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&rt_handles_buf)
        .expect("parse RT handles");

    // Pick a handle from the previous comparison output that was different.
    // type=19 handles from the diff output start at 0x22380A.
    let test_handles: Vec<u64> = vec![0x22380A, 0x22380B, 0x22380C];

    for &handle in &test_handles {
        println!("\n\n========== HANDLE 0x{:X} ==========", handle);

        if let Some(&orig_off) = orig_handle_map.get(&handle) {
            let off = orig_off as usize;
            if let Some((size, ms_len)) = read_ms(&orig_objects, off) {
                let data = &orig_objects[off + ms_len..off + ms_len + size];
                decode_line("ORIGINAL", data);
            }
        }

        if let Some(&rt_off) = rt_handle_map.get(&handle) {
            let off = rt_off as usize;
            if let Some((size, ms_len)) = read_ms(&rt_objects, off) {
                let data = &rt_objects[off + ms_len..off + ms_len + size];
                decode_line("ROUNDTRIP", data);
            }
        }
    }
}
