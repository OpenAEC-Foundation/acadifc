import re

# Read stdout in UTF-16LE
with open('target/stdout9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    content = f.read()

lines = content.splitlines()

# Write relevant sections to UTF-8 file
with open('target/stdout9_utf8.txt', 'w', encoding='utf-8') as out:
    out.write(f"Total lines: {len(lines)}\n")
    # Find PolyfaceMesh analysis section
    for i, line in enumerate(lines):
        line_clean = line.encode('ascii', 'replace').decode('ascii')
        if 'polyface' in line.lower() or 'PFACE' in line or 'vertex' in line.lower() or 'vert' in line.lower() or 'mesh' in line.lower():
            out.write(f"{i+1}: {line_clean}\n")
    
    out.write("\n\n=== All lines with numbers or hashes (potential handles) ===\n")
    for i, line in enumerate(lines):
        line_clean = line.encode('ascii', 'replace').decode('ascii')
        if 'FB197' in line or 'FB1AD' in line or 'fail' in line.lower() or 'loss' in line.lower() or 'mismatch' in line.lower() or 'missing' in line.lower():
            out.write(f"{i+1}: {line_clean}\n")
            
print("Done")
