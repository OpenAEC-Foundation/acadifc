import re, sys

# Read stdout in UTF-16LE
with open('target/stdout9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    content = f.read()

lines = content.splitlines()

# Write to a temp file to avoid encoding issues
with open('target/stdout9_utf8.txt', 'w', encoding='utf-8') as out:
    out.write(f"Total lines: {len(lines)}\n")
    for i, line in enumerate(lines[:150]):
        out.write(f"{i+1}: {line}\n")
        
print("Done, check target/stdout9_utf8.txt")
