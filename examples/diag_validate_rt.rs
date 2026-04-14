/// Validate CRC integrity of a roundtripped DWG file.
/// Reads the original, writes a roundtrip, and validates the output.

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
    
    // Save to disk for external testing
    let rt_path = path.replace(".dwg", "_rt.dwg");
    std::fs::write(&rt_path, &rt_bytes).expect("Failed to write RT DWG");
    println!("Wrote roundtripped file to: {} ({} bytes)", rt_path, rt_bytes.len());
    
    // Validate the roundtripped file using our CRC validation
    println!("\n=== Validating roundtripped file ===");
    let mut rt_reader = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let info = rt_reader.read_file_header().expect("Failed to read RT header");
    
    println!("Version: {:?}", info.version);
    println!("Sections: {}", info.section_descriptors.len());
    
    // Try reading each section
    for sd in &info.section_descriptors {
        match rt_reader.get_section_buffer(&sd.name, &info) {
            Ok(data) => println!("  {} OK ({} bytes)", sd.name, data.len()),
            Err(e) => println!("  {} FAILED: {}", sd.name, e),
        }
    }
    
    // Now try full document re-read
    println!("\n=== Re-reading roundtripped file ===");
    let mut rt_reader2 = DwgReader::from_stream(Cursor::new(&rt_bytes));
    match rt_reader2.read() {
        Ok(doc2) => {
            println!("Re-read OK!");
            println!("  Entities: {}", doc2.entities().count());
            println!("  Objects: {}", doc2.objects.len());
            
            // Check for any notifications
            let errors: Vec<_> = doc2.notifications.iter()
                .filter(|n| matches!(n.notification_type, acadrust::notification::NotificationType::Error))
                .collect();
            let warnings: Vec<_> = doc2.notifications.iter()
                .filter(|n| matches!(n.notification_type, acadrust::notification::NotificationType::Warning))
                .collect();
            println!("  Errors: {}", errors.len());
            println!("  Warnings: {}", warnings.len());
            for e in errors.iter().take(10) {
                println!("    ERROR: {}", e.message);
            }
            for w in warnings.iter().take(30) {
                println!("    WARN: {}", w.message);
            }
        }
        Err(e) => {
            println!("Re-read FAILED: {}", e);
        }
    }
    
    // Now compare original to roundtripped in detail
    println!("\n=== Header comparison ===");
    println!("  Original HANDSEED: {:#X}", doc.header.handle_seed);
    
    // Re-read for doc2
    let mut rt_reader3 = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let doc2 = rt_reader3.read().expect("re-read");
    println!("  RT HANDSEED:       {:#X}", doc2.header.handle_seed);
    println!("  Original model_space_block: {:#X}", doc.header.model_space_block_handle.value());
    println!("  RT model_space_block:       {:#X}", doc2.header.model_space_block_handle.value());
    println!("  Original named_dict: {:#X}", doc.header.named_objects_dict_handle.value());
    println!("  RT named_dict:       {:#X}", doc2.header.named_objects_dict_handle.value());
}
