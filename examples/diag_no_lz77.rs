//! Diagnostic: Write samplekitchen with and without LZ77 compression
//! to help isolate whether errors come from compression or object data.
//!
//! Usage: cargo run --example diag_no_lz77

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
    println!("  Version: {:?}, entities: {}", doc.version, doc.entities().count());

    // Write compressed (normal)
    let out_compressed = "target/diag_compressed.dwg";
    DwgWriter::write_to_file(out_compressed, &doc).expect("write compressed");
    let size_c = std::fs::metadata(out_compressed).map(|m| m.len()).unwrap_or(0);
    println!("Compressed:   {} ({} bytes)", out_compressed, size_c);

    // Write without LZ77
    let out_nolz77 = "target/diag_no_lz77.dwg";
    DwgWriter::write_to_file_no_lz77(out_nolz77, &doc).expect("write no-lz77");
    let size_nc = std::fs::metadata(out_nolz77).map(|m| m.len()).unwrap_or(0);
    println!("No LZ77:      {} ({} bytes)", out_nolz77, size_nc);

    // Verify both can be read back by our reader
    println!("\nRead-back test:");
    {
        let mut r = DwgReader::from_file(out_compressed).expect("open compressed");
        let d = r.read().expect("read compressed");
        println!("  Compressed:  {} entities, {} objects", d.entities().count(), d.objects.len());
    }
    {
        let mut r = DwgReader::from_file(out_nolz77).expect("open no-lz77");
        let d = r.read().expect("read no-lz77");
        println!("  No LZ77:     {} entities, {} objects", d.entities().count(), d.objects.len());
    }

    println!("\nDone. Open both files in AutoCAD to compare RECOVER results.");
    println!("If no-lz77 version has fewer errors → LZ77 compressor bug.");
    println!("If both have same errors → object data formatting bug.");
}
