/// Diagnostic: decode DXF cipher-encoded ACIS SAT text
use acadrust::entities::AcisData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: diag_acis_decode <file.dxf> <handle_hex>");
        std::process::exit(1);
    }
    let path = &args[1];
    let target_handle = &args[2];
    
    let content = std::fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    
    // Find 3DSOLID with given handle
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "3DSOLID" 
            && i + 2 < lines.len() 
            && lines[i + 2].trim().eq_ignore_ascii_case(target_handle) 
        {
            println!("Found 3DSOLID {} at line {}", target_handle, i);
            // Find the ACIS data (group code 1 entries after AcDbModelerGeometry)
            let mut j = i + 4;
            let mut found_modeler = false;
            let mut sat_lines = Vec::new();
            while j < lines.len() {
                let code = lines[j].trim();
                if code == "100" && j + 1 < lines.len() {
                    let subclass = lines[j + 1].trim();
                    if subclass == "AcDbModelerGeometry" {
                        found_modeler = true;
                        j += 2;
                        continue;
                    }
                    if subclass.starts_with("AcDb") && found_modeler {
                        break; // Hit next subclass
                    }
                }
                if found_modeler && (code == "1" || code == "3") {
                    if j + 1 < lines.len() {
                        sat_lines.push(lines[j + 1].to_string());
                    }
                    j += 2;
                    continue;
                }
                if code == "70" && !found_modeler {
                    j += 2; // skip version
                    continue;
                }
                j += 1;
                if j > i + 500 { break; } // safety
            }
            
            println!("\n--- CIPHER TEXT ({} lines) ---", sat_lines.len());
            for (idx, line) in sat_lines.iter().enumerate().take(30) {
                println!("  [{}] {}", idx, line);
            }
            
            // Decode
            let full_text = sat_lines.join("\n");
            let decoded = AcisData::decode_sat(&full_text);
            println!("\n--- DECODED SAT TEXT (first 2000 chars) ---");
            let preview: String = decoded.chars().take(2000).collect();
            println!("{}", preview);
            
            return;
        }
        i += 1;
    }
    
    println!("Handle {} not found in {}", target_handle, path);
}
