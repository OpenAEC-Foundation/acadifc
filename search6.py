import re

# Read UTF-16LE file
with open('target/stderr9.txt', 'r', encoding='utf-16-le', errors='replace') as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

# Find all DEBUG lines
debug_lines = [(i, l.rstrip()) for i, l in enumerate(lines) if 'DEBUG' in l]
print(f"Debug lines total: {len(debug_lines)}")

# Find all WRITE PFACE/VERT for 1FB197
target_lines = [(i, l.rstrip()) for i, l in enumerate(lines) 
                if ('1FB197' in l or '1fb197' in l.lower()) and 'DEBUG' in l]
print(f"\n=== Lines mentioning 0x1FB197 ({len(target_lines)}) ===")
for ln, content in target_lines[:40]:
    print(f"  Line {ln+1}: {content}")

# Find WRITE VERT lines
vert_handles_writes = []
for i, line in enumerate(lines):
    if 'DEBUG WRITE VERT' in line:
        m = re.search(r'DEBUG WRITE VERT (0x[0-9A-Fa-f]+): owner=(0x[0-9A-Fa-f]+)', line)
        if m:
            vert_handles_writes.append((i+1, m.group(1), m.group(2)))

print(f"\n=== WRITE VERT lines (total {len(vert_handles_writes)}) ===")
for ln, vh, owner in vert_handles_writes[:20]:
    print(f"  Line {ln}: vert={vh} owner={owner}")

# Find WRITE VERT for mesh[0] (0x1FB197)
vert_for_mesh0 = [(ln, vh, owner) for ln, vh, owner in vert_handles_writes if '1FB197' in owner or '1fb197' in owner.lower()]
print(f"\n=== WRITE VERT for mesh[0] (0x1FB197): {len(vert_for_mesh0)} entries ===")
for ln, vh, owner in vert_for_mesh0:
    print(f"  Line {ln}: vert={vh} owner={owner}")
