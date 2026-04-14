import re

with open('target/stderr8.txt', 'r', encoding='utf-8', errors='replace') as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

# Find the vertex handles allocated for mesh[0] (0x1FB197)
for i, line in enumerate(lines):
    if 'DEBUG WRITE PFACE' in line or 'DEBUG WRITE VERT' in line:
        if '1fb197' in line.lower() or '1fb1ad' in line.lower():
            print(f"Line {i+1}: {line.rstrip()}")

print("\n--- Searching for write vert handles ---")
vert_handles = set()
for i, line in enumerate(lines):
    if 'DEBUG WRITE VERT' in line and '1fb197' in line.lower():
        m = re.search(r'0x([0-9a-fA-F]+):', line)
        if m:
            vert_handles.add(m.group(1).upper())

print(f"Vertex handles for mesh 0x1FB197: {sorted(vert_handles)}")

print("\n--- Searching for these handles in later passes ---")
for i, line in enumerate(lines):
    for h in vert_handles:
        if h.lower() in line.lower():
            print(f"Line {i+1}: {line.rstrip()}")
            break
