import re

# Read stdout in UTF-16LE
with open('target/stdout9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    content = f.read()

lines = content.splitlines()
print(f"Total lines: {len(lines)}")

# Print everything up to line 100
for i, line in enumerate(lines[:100]):
    print(f"{i+1}: {line}")
