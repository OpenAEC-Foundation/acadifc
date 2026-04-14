/// Diagnostic: Verify all CRC-64 values in an AC21 (R2007) DWG file.
///
/// This reads the raw file bytes and recomputes every CRC-64 from scratch,
/// comparing against the stored values.  Mismatches indicate a writer bug.
///
/// Checks performed:
///   1. Header CRC-64 (metadata offset 0x108)
///   2. Compressed data CRC (file header page)
///   3. Checking sequence CRC (file header page)
///   4. Page map system page CRCs (compressed + uncompressed)
///   5. Section map system page CRCs (compressed + uncompressed)
///   6. Per-data-page CRCs (from section map records)

use std::fs;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};

use acadrust::io::dwg::crc::{
    dwg_ac21_header_crc64,
    dwg_ac21_mirrored_crc64,
    dwg_ac21_normal_crc64,
    dwg_ac21_normal_crc64_seed1,
    dwg_ac21_page_checksum,
    dwg_ac21_check_data_normal_crc64,
    dwg_ac21_check_data_mirrored_crc64,
};
use acadrust::io::dwg::decompressor_ac21::decompress_ac21;
use acadrust::io::dwg::dwg21_metadata::Dwg21CompressedMetadata;
use acadrust::io::dwg::reed_solomon::reed_solomon_decode;

const METADATA_BLOCK_SIZE: usize = 0x80;
const FILE_HEADER_PAGE_SIZE: usize = 0x400;
const RESERVED_HEADER_SIZE: usize = METADATA_BLOCK_SIZE + FILE_HEADER_PAGE_SIZE;
const RS_SYSTEM_K: usize = 239;
const RS_DATA_K: usize = 251;
const RS_N: usize = 255;
const FILE_HEADER_RS_FACTOR: usize = 3;

fn verify_file(path: &str) {
    println!("=== {} ===\n", path);

    let file_data = match fs::read(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            return;
        }
    };

    if file_data.len() < RESERVED_HEADER_SIZE {
        eprintln!("File too small for AC21 format");
        return;
    }

    // Verify magic
    let version = std::str::from_utf8(&file_data[0..6]).unwrap_or("???");
    println!("Version string: {}", version);
    if version != "AC1021" {
        eprintln!("Not an AC1021 file, skipping CRC verification");
        return;
    }
    println!("File size: {} bytes\n", file_data.len());

    let mut pass = 0u32;
    let mut fail = 0u32;

    // ═══════════════════════════════════════════════════════════════
    // 1. RS-decode the file header page (0x80..0x480)
    // ═══════════════════════════════════════════════════════════════
    println!("--- File Header Page RS Decode ---");
    let fh_page = &file_data[METADATA_BLOCK_SIZE..METADATA_BLOCK_SIZE + FILE_HEADER_PAGE_SIZE];

    // RS decode: factor=3, k=239 → 717 decoded bytes
    let decoded_size = FILE_HEADER_RS_FACTOR * RS_SYSTEM_K; // 717
    let mut decoded = vec![0u8; decoded_size];
    reed_solomon_decode(fh_page, &mut decoded, FILE_HEADER_RS_FACTOR, RS_SYSTEM_K);

    // Extract fields from decoded data
    let mut cursor = Cursor::new(&decoded);
    let stored_check_seq_crc = cursor.read_u64::<LittleEndian>().unwrap();
    let check_seq_val1 = cursor.read_u64::<LittleEndian>().unwrap();
    let stored_compr_crc = cursor.read_u64::<LittleEndian>().unwrap();
    let compr_len = cursor.read_i32::<LittleEndian>().unwrap();
    let _length2 = cursor.read_i32::<LittleEndian>().unwrap();

    println!("Stored checking_seq_crc:   {:#018X}", stored_check_seq_crc);
    println!("check_seq_val1 (key):      {:#018X}", check_seq_val1);
    println!("Stored compr_crc:          {:#018X}", stored_compr_crc);
    println!("compr_len:                 {}", compr_len);
    println!("length2:                   {}", _length2);

    // ═══════════════════════════════════════════════════════════════
    // 2. Verify checking sequence CRC
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Checking Sequence CRC ---");
    // The checking sequence is 16 bytes: [val1, encode_value(val1, val1)]
    let check_seq_val2 = encode_value(check_seq_val1, check_seq_val1);
    let mut check_seq_bytes = [0u8; 16];
    check_seq_bytes[0..8].copy_from_slice(&check_seq_val1.to_le_bytes());
    check_seq_bytes[8..16].copy_from_slice(&check_seq_val2.to_le_bytes());
    let recomputed_check_seq_crc = dwg_ac21_normal_crc64_seed1(0, 16, &check_seq_bytes);

    if recomputed_check_seq_crc == stored_check_seq_crc {
        println!("  PASS  checking_seq_crc: {:#018X}", recomputed_check_seq_crc);
        pass += 1;
    } else {
        println!("  FAIL  checking_seq_crc: stored={:#018X} recomputed={:#018X}",
                 stored_check_seq_crc, recomputed_check_seq_crc);
        fail += 1;
    }

    // ═══════════════════════════════════════════════════════════════
    // 3. Extract and decompress metadata
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Metadata Decompression ---");
    let mut metadata_buffer = vec![0u8; 0x110];

    // Compressed data starts at decoded offset 0x20 (32)
    let compr_data_start = 32usize;
    let compr_data_len = if compr_len < 0 {
        (-compr_len) as usize
    } else {
        compr_len as usize
    };
    let compr_data_end = (compr_data_start + compr_data_len).min(decoded.len());
    let compressed_metadata = &decoded[compr_data_start..compr_data_end];

    if compr_len < 0 {
        // Negative = raw (uncompressed)
        let copy_len = compr_data_len.min(0x110).min(compressed_metadata.len());
        metadata_buffer[..copy_len].copy_from_slice(&compressed_metadata[..copy_len]);
        println!("Metadata stored uncompressed ({} bytes)", compr_data_len);
    } else {
        // Positive = LZ77 compressed
        decompress_ac21(&decoded, compr_data_start as u32, compr_len as u32, &mut metadata_buffer);
        println!("Metadata LZ77 compressed ({} bytes → 0x110)", compr_len);
    }

    // ═══════════════════════════════════════════════════════════════
    // 4. Verify compressed data CRC (over compressed metadata)
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Compressed Data CRC ---");
    let recomputed_compr_crc = dwg_ac21_normal_crc64(0, compressed_metadata.len() as u32, compressed_metadata);

    if recomputed_compr_crc == stored_compr_crc {
        println!("  PASS  compr_crc: {:#018X}", recomputed_compr_crc);
        pass += 1;
    } else {
        println!("  FAIL  compr_crc: stored={:#018X} recomputed={:#018X}",
                 stored_compr_crc, recomputed_compr_crc);
        fail += 1;
    }

    // ═══════════════════════════════════════════════════════════════
    // 5. Parse metadata and verify header CRC-64
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Header CRC-64 ---");
    let metadata = match Dwg21CompressedMetadata::from_bytes(&metadata_buffer) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to parse metadata: {:?}", e);
            return;
        }
    };

    println!("  file_size:          {}", metadata.file_size);
    println!("  crc_seed:           {:#018X}", metadata.crc_seed);
    println!("  crc_seed_encoded:   {:#018X}", metadata.crc_seed_encoded);
    println!("  random_seed:        {:#018X}", metadata.random_seed);
    println!("  stream_version:     {:#018X}", metadata.stream_version);
    println!("  pages_map_crc_seed: {:#018X}", metadata.pages_map_crc_seed);
    println!("  sect_map_crc_seed:  {:#018X}", metadata.sections_map_crc_seed);

    // === RNG hypothesis test ===
    // Hypothesis A: spec §5.2.1.1.1 says RandomSeed IS the CRC encoder's seed
    // Order per spec: SectionsMapCrcSeed(§3) → PagesMapCrcSeed(§4) → CheckData(§5) → CrcSeedEncoded(§6)
    {
        let mut rng = DiagCrcRandomEncoder::new(metadata.random_seed);
        let test_sect = rng.encode_crc_seed(metadata.crc_seed);
        let test_pages = rng.encode_crc_seed(metadata.crc_seed);
        let test_r1 = rng.get_next_u64();
        let test_r2 = rng.get_next_u64();
        let test_enc_check = rng.encode_crc_seed(metadata.crc_seed);
        // NormalCrc and MirroredCrc computed but skip for now
        // §5.2.1.1.6: CrcSeedEncoded AFTER check data
        let test_crc_seed_enc = rng.encode_crc_seed(metadata.crc_seed);
        println!("\n  RNG Hypothesis A (spec): seed=random_seed ({:#018X})", metadata.random_seed);
        println!("    sections_map_crc_seed: {:#018X} {}",
                 test_sect, if test_sect == metadata.sections_map_crc_seed { "MATCH" } else { "MISMATCH" });
        println!("    pages_map_crc_seed:    {:#018X} {}",
                 test_pages, if test_pages == metadata.pages_map_crc_seed { "MATCH" } else { "MISMATCH" });
        println!("    check_random1:         {:#018X}", test_r1);
        println!("    check_random2:         {:#018X}", test_r2);
        println!("    check_encoded_seed:    {:#018X}", test_enc_check);
        println!("    crc_seed_encoded:      {:#018X} {}",
                 test_crc_seed_enc, if test_crc_seed_enc == metadata.crc_seed_encoded { "MATCH" } else { "MISMATCH" });
    }
    println!();

    println!("  header_crc64:       {:#018X}", metadata.header_crc64);
    println!("  pages_amount:       {}", metadata.pages_amount);
    println!("  sections_amount:    {}", metadata.sections_amount);

    let recomputed_header_crc = dwg_ac21_header_crc64(&metadata_buffer);
    if recomputed_header_crc == metadata.header_crc64 {
        println!("  PASS  header_crc64: {:#018X}", recomputed_header_crc);
        pass += 1;
    } else {
        println!("  FAIL  header_crc64: stored={:#018X} recomputed={:#018X}",
                 metadata.header_crc64, recomputed_header_crc);
        fail += 1;
    }

    // ═══════════════════════════════════════════════════════════════
    // 6. Read page map and verify its CRCs
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Page Map CRCs ---");
    let crc_seed = metadata.crc_seed;

    // Page map is at file offset = RESERVED_HEADER_SIZE + pages_map_offset
    let pm_file_offset = RESERVED_HEADER_SIZE as u64 + metadata.pages_map_offset;
    println!("  Page map at file offset: {:#X}", pm_file_offset);
    println!("  Stored pages_map_crc_compressed:   {:#018X}", metadata.pages_map_crc_compressed);
    println!("  Stored pages_map_crc_uncompressed: {:#018X}", metadata.pages_map_crc_uncompressed);

    if let Some((pm_compressed, pm_uncompressed)) = read_and_verify_system_page(
        &file_data,
        pm_file_offset as usize,
        metadata.pages_map_size_compressed,
        metadata.pages_map_size_uncompressed,
        metadata.pages_map_correction_factor,
        crc_seed,
    ) {
        let recomputed_pm_crc_comp = dwg_ac21_mirrored_crc64(
            crc_seed, pm_compressed.len() as u32, &pm_compressed
        );
        let recomputed_pm_crc_uncomp = dwg_ac21_mirrored_crc64(
            crc_seed, pm_uncompressed.len() as u32, &pm_uncompressed
        );

        if recomputed_pm_crc_comp == metadata.pages_map_crc_compressed {
            println!("  PASS  pages_map_crc_compressed: {:#018X}", recomputed_pm_crc_comp);
            pass += 1;
        } else {
            println!("  FAIL  pages_map_crc_compressed: stored={:#018X} recomputed={:#018X}",
                     metadata.pages_map_crc_compressed, recomputed_pm_crc_comp);
            fail += 1;
        }

        if recomputed_pm_crc_uncomp == metadata.pages_map_crc_uncompressed {
            println!("  PASS  pages_map_crc_uncompressed: {:#018X}", recomputed_pm_crc_uncomp);
            pass += 1;
        } else {
            println!("  FAIL  pages_map_crc_uncompressed: stored={:#018X} recomputed={:#018X}",
                     metadata.pages_map_crc_uncompressed, recomputed_pm_crc_uncomp);
            fail += 1;
        }

        // Build page map: list of (size, id) entries → compute offsets
        let page_table = build_page_table(&pm_uncompressed);
        println!("  Page map entries: {}", page_table.len());

        // ═══════════════════════════════════════════════════════════════
        // 7. Read section map and verify its CRCs
        // ═══════════════════════════════════════════════════════════════
        println!("\n--- Section Map CRCs ---");
        let sm_page_id = metadata.sections_map_id as i64;
        if let Some(&(sm_offset, _sm_size)) = page_table.iter().find(|&&(_, id)| id == sm_page_id).map(|x| {
            // Find the offset for this page ID in the page_table_with_offsets
            x
        }) {
            // page_table_with_offsets has (offset, id) pairs
            // Actually, we need to get the file offset from the page table
            // Let me rebuild with offsets
            let _ = sm_offset;
        }
        // Let me use the page offset table properly
        let page_offset_map = build_page_offset_map(&pm_uncompressed);
        println!("  sections_map page id: {}", sm_page_id);

        if let Some(&sm_file_offset) = page_offset_map.get(&sm_page_id) {
            println!("  Section map at file offset: {:#X}", sm_file_offset);
            println!("  Stored sections_map_crc_compressed:   {:#018X}", metadata.sections_map_crc_compressed);
            println!("  Stored sections_map_crc_uncompressed: {:#018X}", metadata.sections_map_crc_uncompressed);

            if let Some((sm_compressed, sm_uncompressed)) = read_and_verify_system_page(
                &file_data,
                sm_file_offset as usize,
                metadata.sections_map_size_compressed,
                metadata.sections_map_size_uncompressed,
                metadata.sections_map_correction_factor,
                crc_seed,
            ) {
                let recomputed_sm_crc_comp = dwg_ac21_mirrored_crc64(
                    crc_seed, sm_compressed.len() as u32, &sm_compressed
                );
                let recomputed_sm_crc_uncomp = dwg_ac21_mirrored_crc64(
                    crc_seed, sm_uncompressed.len() as u32, &sm_uncompressed
                );

                if recomputed_sm_crc_comp == metadata.sections_map_crc_compressed {
                    println!("  PASS  sections_map_crc_compressed: {:#018X}", recomputed_sm_crc_comp);
                    pass += 1;
                } else {
                    println!("  FAIL  sections_map_crc_compressed: stored={:#018X} recomputed={:#018X}",
                             metadata.sections_map_crc_compressed, recomputed_sm_crc_comp);
                    fail += 1;
                }

                if recomputed_sm_crc_uncomp == metadata.sections_map_crc_uncompressed {
                    println!("  PASS  sections_map_crc_uncompressed: {:#018X}", recomputed_sm_crc_uncomp);
                    pass += 1;
                } else {
                    println!("  FAIL  sections_map_crc_uncompressed: stored={:#018X} recomputed={:#018X}",
                             metadata.sections_map_crc_uncompressed, recomputed_sm_crc_uncomp);
                    fail += 1;
                }

                // ═══════════════════════════════════════════════════════════════
                // 8. Parse section map and verify per-page CRCs
                // ═══════════════════════════════════════════════════════════════
                println!("\n--- Data Page CRCs (from section map) ---");
                verify_section_page_crcs(
                    &file_data,
                    &sm_uncompressed,
                    &page_offset_map,
                    crc_seed,
                    &mut pass,
                    &mut fail,
                );
            } else {
                println!("  SKIP  Failed to decode section map system page");
            }
        } else {
            println!("  SKIP  Could not find section map page id {} in page table", sm_page_id);
        }
    } else {
        println!("  SKIP  Failed to decode page map system page");
    }

    // ═══════════════════════════════════════════════════════════════
    // 9. Verify check data at end of file header page
    // ═══════════════════════════════════════════════════════════════
    println!("\n--- Check Data (file header page tail) ---");
    let check_data_offset = 0x3D8;
    let check_data = &fh_page[check_data_offset..FILE_HEADER_PAGE_SIZE];
    let mut cd_cursor = Cursor::new(check_data);
    let stored_normal_crc = cd_cursor.read_u64::<LittleEndian>().unwrap();
    let stored_mirrored_crc = cd_cursor.read_u64::<LittleEndian>().unwrap();
    let cd_random1 = cd_cursor.read_u64::<LittleEndian>().unwrap();
    let cd_random2 = cd_cursor.read_u64::<LittleEndian>().unwrap();
    let cd_encoded_crc_seed = cd_cursor.read_u64::<LittleEndian>().unwrap();

    println!("  Normal CRC:         {:#018X}", stored_normal_crc);
    println!("  Mirrored CRC:       {:#018X}", stored_mirrored_crc);
    println!("  Random1:            {:#018X}", cd_random1);
    println!("  Random2:            {:#018X}", cd_random2);
    println!("  Encoded CRC Seed:   {:#018X}", cd_encoded_crc_seed);

    // Recompute Normal CRC
    let normal_crc_recomputed = {
        let mut buf = [0u64; 8];
        buf[0] = encode_value(cd_random1, cd_random2);
        buf[1] = encode_value(buf[0], buf[0]);
        buf[2] = encode_value(cd_random2, buf[1]);
        buf[3] = encode_value(buf[2], buf[2]);
        buf[4] = encode_value(cd_random1, buf[3]);
        buf[5] = encode_value(buf[4], buf[4]);
        buf[6] = encode_value(buf[5], buf[5]);
        buf[7] = encode_value(buf[6], buf[6]);

        let mut bytes = [0u8; 64];
        for (i, &val) in buf.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
        }
        dwg_ac21_check_data_normal_crc64(cd_random2, &bytes)
    };

    if normal_crc_recomputed == stored_normal_crc {
        println!("  PASS  Normal CRC: {:#018X}", normal_crc_recomputed);
        pass += 1;
    } else {
        println!("  FAIL  Normal CRC: stored={:#018X} recomputed={:#018X}",
                 stored_normal_crc, normal_crc_recomputed);
        fail += 1;
    }

    // Recompute Mirrored CRC
    let mirrored_crc_recomputed = {
        let mut buf = [0u64; 8];
        buf[0] = encode_value(cd_random1, cd_random2);
        buf[1] = encode_value(stored_normal_crc, buf[0]);
        buf[2] = encode_value(cd_random2, buf[1]);
        buf[3] = encode_value(stored_normal_crc, buf[2]);
        buf[4] = encode_value(cd_random1, buf[3]);
        buf[5] = encode_value(stored_normal_crc, buf[4]);
        buf[6] = encode_value(cd_random2, buf[5]);
        buf[7] = encode_value(buf[6], buf[6]);

        let mut bytes = [0u8; 64];
        for (i, &val) in buf.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
        }
        dwg_ac21_check_data_mirrored_crc64(cd_random1, &bytes)
    };

    if mirrored_crc_recomputed == stored_mirrored_crc {
        println!("  PASS  Mirrored CRC: {:#018X}", mirrored_crc_recomputed);
        pass += 1;
    } else {
        println!("  FAIL  Mirrored CRC: stored={:#018X} recomputed={:#018X}",
                 stored_mirrored_crc, mirrored_crc_recomputed);
        fail += 1;
    }

    // ═══════════════════════════════════════════════════════════════
    // Summary
    // ═══════════════════════════════════════════════════════════════
    println!("\n========================================");
    println!("PASS: {}, FAIL: {}", pass, fail);
    if fail > 0 {
        println!("CRC VERIFICATION FAILED");
    } else {
        println!("ALL CRCs VALID");
    }
    println!();
}

fn encode_value(value: u64, control: u64) -> u64 {
    let shift = (control & 0x1F) as u32;
    if shift == 0 {
        value
    } else {
        (value << shift) | (value >> (64 - shift))
    }
}

/// Read a system page (page map or section map), RS-decode, and optionally decompress.
/// Returns (compressed_data, uncompressed_data) or None on error.
fn read_and_verify_system_page(
    file_data: &[u8],
    file_offset: usize,
    size_compressed: u64,
    size_uncompressed: u64,
    correction_factor: u64,
    _crc_seed: u64,
) -> Option<(Vec<u8>, Vec<u8>)> {
    // System page RS parameters
    let aligned_comp = ((size_compressed as usize + 7) & !7) * correction_factor as usize;
    let block_count = (aligned_comp + RS_SYSTEM_K - 1) / RS_SYSTEM_K;
    let page_size = (block_count * RS_N + 7) & !7;

    if file_offset + page_size > file_data.len() {
        eprintln!("  System page extends beyond file (offset={:#X}, page_size={})", file_offset, page_size);
        return None;
    }

    let raw_page = &file_data[file_offset..file_offset + page_size];

    // RS decode
    let decoded_size = block_count * RS_SYSTEM_K;
    let mut decoded = vec![0u8; decoded_size];
    reed_solomon_decode(raw_page, &mut decoded, block_count, RS_SYSTEM_K);

    // The compressed data is the first size_compressed bytes of the decoded data
    let comp_len = size_compressed as usize;
    let compressed = decoded[..comp_len.min(decoded.len())].to_vec();

    // Decompress
    let mut uncompressed = vec![0u8; size_uncompressed as usize];
    if size_compressed < size_uncompressed {
        decompress_ac21(&decoded, 0, size_compressed as u32, &mut uncompressed);
    } else {
        let copy_len = (size_uncompressed as usize).min(decoded.len());
        uncompressed[..copy_len].copy_from_slice(&decoded[..copy_len]);
    }

    Some((compressed, uncompressed))
}

/// Build a page offset map from uncompressed page map data.
/// Returns HashMap<page_id, file_offset>.
fn build_page_offset_map(pm_data: &[u8]) -> std::collections::HashMap<i64, u64> {
    let mut map = std::collections::HashMap::new();
    let mut cursor = Cursor::new(pm_data);
    let mut offset = RESERVED_HEADER_SIZE as u64; // Pages start after 0x480

    while (cursor.position() as usize) + 16 <= pm_data.len() {
        let size = cursor.read_i64::<LittleEndian>().unwrap_or(0);
        let id = cursor.read_i64::<LittleEndian>().unwrap_or(0);

        if size == 0 && id == 0 {
            break; // Null terminator
        }

        if id > 0 {
            map.insert(id, offset);
        }

        offset += size as u64;
    }

    map
}

/// Build a simple page table from uncompressed page map data.
/// Returns Vec<(file_offset, page_id)>.
fn build_page_table(pm_data: &[u8]) -> Vec<(u64, i64)> {
    let mut table = Vec::new();
    let mut cursor = Cursor::new(pm_data);
    let mut offset = RESERVED_HEADER_SIZE as u64;

    while (cursor.position() as usize) + 16 <= pm_data.len() {
        let size = cursor.read_i64::<LittleEndian>().unwrap_or(0);
        let id = cursor.read_i64::<LittleEndian>().unwrap_or(0);

        if size == 0 && id == 0 {
            break;
        }

        if id > 0 {
            table.push((offset, id));
        }

        offset += size as u64;
    }

    table
}

/// Parse section map and verify per-page CRCs.
fn verify_section_page_crcs(
    file_data: &[u8],
    sm_data: &[u8],
    page_offset_map: &std::collections::HashMap<i64, u64>,
    crc_seed: u64,
    pass: &mut u32,
    fail: &mut u32,
) {
    let mut cursor = Cursor::new(sm_data);

    while (cursor.position() as usize) + 0x40 <= sm_data.len() {
        // Section header: 0x40 bytes
        let data_size = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let max_size = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let _encryption = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let _hashcode = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let name_length = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let _unknown = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let encoding = cursor.read_u64::<LittleEndian>().unwrap_or(0);
        let num_pages = cursor.read_u64::<LittleEndian>().unwrap_or(0);

        if data_size == 0 && max_size == 0 && num_pages == 0 {
            break;
        }

        // Read section name (UTF-16LE)
        let name_bytes = name_length as usize;
        if cursor.position() as usize + name_bytes > sm_data.len() {
            break;
        }
        let mut name_buf = vec![0u8; name_bytes];
        std::io::Read::read_exact(&mut cursor, &mut name_buf).ok();
        let name: String = name_buf
            .chunks(2)
            .filter_map(|c| {
                if c.len() == 2 {
                    let ch = u16::from_le_bytes([c[0], c[1]]);
                    char::from_u32(ch as u32)
                } else {
                    None
                }
            })
            .collect();

        println!("\n  Section: {} (encoding={}, pages={}, data_size={})", name, encoding, num_pages, data_size);

        // Read per-page records (56 bytes each = 7 × u64)
        for page_idx in 0..num_pages {
            if cursor.position() as usize + 56 > sm_data.len() {
                break;
            }
            let page_data_offset = cursor.read_u64::<LittleEndian>().unwrap_or(0);
            let page_size = cursor.read_u64::<LittleEndian>().unwrap_or(0);
            let page_id = cursor.read_i64::<LittleEndian>().unwrap_or(0);
            let uncomp_size = cursor.read_u64::<LittleEndian>().unwrap_or(0);
            let comp_size = cursor.read_u64::<LittleEndian>().unwrap_or(0);
            let stored_checksum = cursor.read_u64::<LittleEndian>().unwrap_or(0);
            let stored_crc = cursor.read_u64::<LittleEndian>().unwrap_or(0);

            // Find this page in the page offset map
            if let Some(&page_file_offset) = page_offset_map.get(&page_id) {
                // Read the raw page data from the file
                let page_end = (page_file_offset as usize) + (page_size as usize);
                if page_end > file_data.len() {
                    println!("    Page[{}] id={}: extends beyond file", page_idx, page_id);
                    continue;
                }

                let raw_page = &file_data[page_file_offset as usize..page_end];

                if encoding == 4 {
                    // encoding=4: RS(255,251) encoded compressed data
                    // RS-decode to get compressed data
                    let aligned_comp = ((comp_size as usize + 7) & !7) as usize;
                    let factor = (aligned_comp + RS_DATA_K - 1) / RS_DATA_K;
                    let expected_rs_size = factor * RS_N;

                    if expected_rs_size > raw_page.len() {
                        println!("    Page[{}] id={}: RS size {} > page_size {}",
                                 page_idx, page_id, expected_rs_size, raw_page.len());
                        continue;
                    }

                    let decoded_size = factor * RS_DATA_K;
                    let mut decoded_page = vec![0u8; decoded_size];
                    reed_solomon_decode(raw_page, &mut decoded_page, factor, RS_DATA_K);

                    // The compressed data is the first comp_size bytes
                    let comp_data = &decoded_page[..comp_size as usize];

                    // Verify CRC on compressed data (before RS encoding)
                    let recomputed_crc = dwg_ac21_mirrored_crc64(0, comp_data.len() as u32, comp_data);

                    if recomputed_crc == stored_crc {
                        *pass += 1;
                    } else {
                        println!("    FAIL  Page[{}] id={} crc: stored={:#018X} recomputed={:#018X} (comp_size={}, uncomp_size={})",
                                 page_idx, page_id, stored_crc, recomputed_crc, comp_size, uncomp_size);
                        *fail += 1;
                    }

                    // Also verify checksum on uncompressed data
                    if comp_size < uncomp_size {
                        let mut uncomp_data = vec![0u8; uncomp_size as usize];
                        decompress_ac21(&decoded_page, 0, comp_size as u32, &mut uncomp_data);
                        let recomputed_checksum = dwg_ac21_page_checksum(0, &uncomp_data) as u64;
                        if recomputed_checksum != stored_checksum {
                            println!("    FAIL  Page[{}] id={} checksum: stored={:#018X} recomputed={:#018X}",
                                     page_idx, page_id, stored_checksum, recomputed_checksum);
                            *fail += 1;
                        } else {
                            *pass += 1;
                        }
                    } else {
                        // Not compressed (comp_size == uncomp_size)
                        let recomputed_checksum = dwg_ac21_page_checksum(0, comp_data) as u64;
                        if recomputed_checksum != stored_checksum {
                            println!("    FAIL  Page[{}] id={} checksum: stored={:#018X} recomputed={:#018X}",
                                     page_idx, page_id, stored_checksum, recomputed_checksum);
                            *fail += 1;
                        } else {
                            *pass += 1;
                        }
                    }
                } else {
                    // encoding=1: raw data (no RS, no compression)
                    let data_len = uncomp_size as usize;
                    if data_len > raw_page.len() {
                        println!("    Page[{}] id={}: data_len {} > page_size {}",
                                 page_idx, page_id, data_len, raw_page.len());
                        continue;
                    }

                    let page_data = &raw_page[..data_len];
                    let recomputed_crc = dwg_ac21_mirrored_crc64(0, page_data.len() as u32, page_data);

                    if recomputed_crc == stored_crc {
                        *pass += 1;
                    } else {
                        println!("    FAIL  Page[{}] id={} crc: stored={:#018X} recomputed={:#018X} (encoding=1)",
                                 page_idx, page_id, stored_crc, recomputed_crc);
                        *fail += 1;
                    }

                    let recomputed_checksum = dwg_ac21_page_checksum(0, page_data) as u64;
                    if recomputed_checksum != stored_checksum {
                        println!("    FAIL  Page[{}] id={} checksum: stored={:#018X} recomputed={:#018X}",
                                 page_idx, page_id, stored_checksum, recomputed_checksum);
                        *fail += 1;
                    } else {
                        *pass += 1;
                    }
                }
            } else {
                println!("    Page[{}] id={}: not found in page map (offset={}, size={})",
                         page_idx, page_id, page_data_offset, page_size);
            }
        }
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: cargo run --example diag_crc64_verify -- <file1.dwg> [file2.dwg ...]");
        std::process::exit(2);
    }

    for path in &paths {
        verify_file(path);
    }
}

/// CRC random encoder matching ODA spec §5.11 exactly.
/// Uses LCG init for entries 0-1, MT-style init for 2+, no tempering,
/// InitPadding phase consuming first 128 entries, and 10-bit spread encoding.
struct DiagCrcRandomEncoder {
    table: Vec<u32>,
    index: usize,
    #[allow(dead_code)]
    padding: Vec<u32>,
}

impl DiagCrcRandomEncoder {
    fn new(seed: u64) -> Self {
        let mut table = vec![0u32; 0x270];
        // LCG init for entries 0 and 1 (spec §5.11, MSLCG constants)
        table[0] = (seed as u32).wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        table[1] = ((seed >> 32) as u32).wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        // MT-style init for entries 2..624
        let mut value = table[1];
        for i in 2..0x270usize {
            value = ((value >> 30) ^ value).wrapping_mul(0x6c078965).wrapping_add(i as u32);
            table[i] = value;
        }
        let mut encoder = Self { table, index: 0, padding: vec![0u32; 0x80] };
        encoder.init_padding();
        encoder
    }

    fn init_padding(&mut self) {
        for i in 0..0x80 {
            self.update_index();
            self.padding[i] = self.table[self.index];
            self.index += 1;
        }
    }

    fn update_index(&mut self) {
        if self.index >= 0x270 {
            // MT twist regeneration
            for i in 0..0x270 {
                let y = (self.table[i] & 0x80000000)
                    | (self.table[(i + 1) % 0x270] & 0x7FFFFFFF);
                self.table[i] = self.table[(i + 0x18D) % 0x270] ^ (y >> 1);
                if y & 1 != 0 {
                    self.table[i] ^= 0x9908B0DF;
                }
            }
            self.index = 0;
        }
    }

    fn get_next_u64(&mut self) -> u64 {
        self.index += 2;
        self.update_index();
        let lo = self.table[self.index] as u64;
        let hi = self.table[self.index + 1] as u64;
        lo | (hi << 32)
    }

    fn encode(&mut self, value: u32) -> u64 {
        let random = self.get_next_u64();
        let mut lo = (random as u32) & 0xdf7df7df;
        let mut hi = ((random >> 32) as u32) & 0xf7df7df7;
        if value & 0x200 != 0 { lo |= 0x20; }
        if value & 0x100 != 0 { lo |= 0x800; }
        if value & 0x80 != 0 { lo |= 0x20000; }
        if value & 0x40 != 0 { lo |= 0x800000; }
        if value & 0x20 != 0 { lo |= 0x20000000; }
        if value & 0x10 != 0 { hi |= 0x08; }
        if value & 0x8 != 0 { hi |= 0x200; }
        if value & 0x4 != 0 { hi |= 0x8000; }
        if value & 0x2 != 0 { hi |= 0x200000; }
        if value & 0x1 != 0 { hi |= 0x8000000; }
        (lo as u64) | ((hi as u64) << 32)
    }

    fn encode_crc_seed(&mut self, seed: u64) -> u64 {
        // Spec says encode takes UInt32. CrcSeed is u64 but only 10 LSBs matter.
        self.encode(seed as u32)
    }
}
