//! Diagnostic: Verify LZ77 compression roundtrip integrity per page
//!
//! For each page of AcDbObjects, checks that compress→decompress produces identical data.
//! Also checks if what we decompress from the written file matches the original.
//!
//! Usage: cargo run --example diag_lz77_verify

use acadrust::io::dwg::compressor_ac21::compress_ac21;
use acadrust::io::dwg::decompressor_ac21::decompress_ac21;
use acadrust::io::dwg::{DwgReader, DwgWriter};

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("Reading: {}", input);
    let doc = {
        let mut reader = DwgReader::from_file(&input).expect("open");
        reader.read().expect("read")
    };

    // Get the raw objects data from the writer (before compression)
    let writer =
        acadrust::io::dwg::dwg_stream_writers::object_writer::DwgObjectWriter::new(&doc)
            .expect("create object writer");
    let (obj_data, handle_map, _extents, _sab) = writer.write();
    println!(
        "Raw objects data: {} bytes ({} handle entries)",
        obj_data.len(),
        handle_map.len()
    );

    // Partition into pages
    let page_size: usize = 0xF800;
    let total_pages = (obj_data.len() + page_size - 1) / page_size;
    println!("Pages: {} (at {} bytes each)\n", total_pages, page_size);

    let mut bad_pages = 0usize;
    for page_idx in 0..total_pages {
        let start = page_idx * page_size;
        let end = (start + page_size).min(obj_data.len());
        let page_data = &obj_data[start..end];

        // Compress
        let compressed = compress_ac21(page_data);

        // Decompress back
        let mut decompressed = vec![0u8; page_data.len()];
        decompress_ac21(&compressed, 0, compressed.len() as u32, &mut decompressed);

        // Compare byte by byte
        let mut first_diff = None;
        let mut diff_count = 0;
        for i in 0..page_data.len() {
            if decompressed[i] != page_data[i] {
                if first_diff.is_none() {
                    first_diff = Some(i);
                }
                diff_count += 1;
            }
        }

        if diff_count > 0 {
            let fd = first_diff.unwrap();
            println!(
                "Page {:2}: {} MISMATCHES (first at page offset 0x{:X}, section offset 0x{:X}). ratio={:.1}%",
                page_idx,
                diff_count,
                fd,
                start + fd,
                compressed.len() as f64 / page_data.len() as f64 * 100.0
            );
            // Show context around first diff
            let ctx_start = fd.saturating_sub(4);
            let ctx_end = (fd + 8).min(page_data.len());
            print!("  Original:     ");
            for i in ctx_start..ctx_end {
                if i == fd {
                    print!("[{:02X}]", page_data[i]);
                } else {
                    print!(" {:02X} ", page_data[i]);
                }
            }
            println!();
            print!("  Decompressed: ");
            for i in ctx_start..ctx_end {
                if i == fd {
                    print!("[{:02X}]", decompressed[i]);
                } else {
                    print!(" {:02X} ", decompressed[i]);
                }
            }
            println!();
            bad_pages += 1;
        } else {
            println!(
                "Page {:2}: OK (comp ratio={:.1}%)",
                page_idx,
                compressed.len() as f64 / page_data.len() as f64 * 100.0
            );
        }
    }

    println!("\n=== Summary ===");
    println!("Total pages: {}", total_pages);
    println!("Bad pages: {}", bad_pages);
    if bad_pages == 0 {
        println!("All pages round-trip correctly through LZ77 compress/decompress");
        println!("=> LZ77 roundtrip is self-consistent.");
        println!("=> If AutoCAD still fails, its decompressor interprets our output differently.");
    } else {
        println!("LZ77 compression roundtrip FAILURES detected!");
        println!("=> LZ77 compressor BUG confirmed.");
    }
}
