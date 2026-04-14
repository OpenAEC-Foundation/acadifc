import re

# Read stdout in UTF-16LE
with open('target/stdout9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    content = f.read()

# Find polyface mesh section
lines = content.splitlines()
in_section = False
for i, line in enumerate(lines):
    if 'PolyfaceMesh' in line or 'polyface' in line.lower() or 'PFACE' in line:
        in_section = True
    if in_section:
        print(f"{i+1}: {line}")
        if i > 0 and (i - 100) > 0 and not any('poly' in lines[j].lower() for j in range(i-3, min(i+3, len(lines)))):
            pass  # Keep printing

# Print first 10 lines  
print("\n=== First 10 lines ===")
for i, line in enumerate(lines[:10]):
    print(f"{i+1}: {line}")
