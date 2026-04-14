import re

# Read UTF-16LE file
with open('target/stderr9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

# What are the vertex handles for mesh[0]?
vert_handles = {'229116', '229117', '229118', '229119', '22911A', '22911B', '22911C', '22911D',
                '229116', '229117', '229118', '229119', '22911a', '22911b', '22911c', '22911d'}

# Search for any of these handles in any line
found = []
for i, line in enumerate(lines):
    for h in vert_handles:
        if h in line:
            found.append((i+1, line.rstrip()))
            break

print(f"\nAll mentions of vertex handles (0x229116-0x22911D): {len(found)}")
for ln, content in found[:30]:
    print(f"  Line {ln}: {content}")

# Also look for face handles (0x22911E - 0x229129) and seqend (0x22912A)
face_handles = set()
for n in range(0x22911E, 0x22912B):
    face_handles.add(format(n, 'X'))
    face_handles.add(format(n, 'x'))
    
found2 = []
for i, line in enumerate(lines):
    for h in face_handles:
        if h in line:
            found2.append((i+1, line.rstrip()))
            break

print(f"\nAll mentions of face handles (0x22911E-0x22912A): {len(found2)}")
for ln, content in found2[:30]:
    print(f"  Line {ln}: {content}")
    
# Also check what's at line 105485 (between write pface and first vert)
print(f"\nLines 105480-105510 context:")
for i in range(105479, min(105510, len(lines))):
    print(f"  {i+1}: {lines[i].rstrip()}")
