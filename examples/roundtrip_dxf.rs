/// Roundtrip DXF files: read → write → read-back and compare entity/layer counts.
use acadrust::{DxfReader, DxfWriter};

fn main() -> acadrust::Result<()> {
    let files = [
        "examples/DXF/A1K_50$1-ALT_PANEL.dxf",
        "examples/DXF/A2K_90_90$1-KOSE_DOLAP_SOL_ARKA_PANEL.dxf",
        "examples/DXF/A2K_90_90$1-KOSE_DOLAP_SOL_PANEL.dxf",
        "examples/DXF/A2K_90_90$1-KOSE_DOLAP_UST_PANEL.dxf",
    ];

    let mut all_pass = true;

    for input in &files {
        let short = input.rsplit('/').next().unwrap();
        println!("== {short} ==");

        // ── Read original ──
        let doc = match DxfReader::from_file(input).and_then(|mut r| r.read()) {
            Ok(d) => d,
            Err(e) => {
                println!("  READ FAILED: {e}\n");
                all_pass = false;
                continue;
            }
        };
        let ent1 = doc.entities().count();
        let lay1 = doc.layers.len();
        let ver = doc.version;
        println!("  Read:    {ent1} entities, {lay1} layers, version={ver:?}");

        // Show block breakdown
        for br in doc.block_records.iter() {
            if !br.entities.is_empty() {
                println!("    Block '{}': {} entities", br.name, br.entities.len());
            }
        }

        // ── Write DXF ──
        let output = format!("target/{}_roundtrip.dxf", short.trim_end_matches(".dxf"));
        let writer = DxfWriter::new(doc.clone());
        match writer.write_to_file(&output) {
            Ok(_) => {
                let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                println!("  Written: {output} ({size} bytes)");
            }
            Err(e) => {
                println!("  WRITE FAILED: {e}\n");
                all_pass = false;
                continue;
            }
        }

        // ── Read back ──
        match DxfReader::from_file(&output).and_then(|mut r| r.read()) {
            Ok(doc2) => {
                let ent2 = doc2.entities().count();
                let lay2 = doc2.layers.len();
                println!("  Readback: {ent2} entities, {lay2} layers");
                if ent1 == ent2 && lay1 == lay2 {
                    println!("  PASS");
                } else {
                    println!("  DIFF: entities {ent1}->{ent2}, layers {lay1}->{lay2}");
                    all_pass = false;
                }
            }
            Err(e) => {
                println!("  READ-BACK FAILED: {e}");
                all_pass = false;
            }
        }
        println!();
    }

    if all_pass {
        println!("All files PASS");
    } else {
        println!("Some files FAILED");
        std::process::exit(1);
    }

    Ok(())
}
