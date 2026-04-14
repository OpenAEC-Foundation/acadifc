import re

with open('target/stderr9.txt', 'r', encoding='utf-8', errors='replace') as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

# Find all DEBUG WRITE VERT lines
vert_lines = [(i+1, line.rstrip()) for i, line in enumerate(lines) if 'DEBUG WRITE VERT' in line]
print(f"\nAll DEBUG WRITE VERT lines ({len(vert_lines)}):")
for ln, content in vert_lines[:20]:
    print(f"  Line {ln}: {content}")

# Find all DEBUG WRITE PFACE lines for 1FB197
pface_lines = [(i+1, line.rstrip()) for i, line in enumerate(lines) if 'DEBUG WRITE PFACE' in line and '1fb197' in line.lower()]
print(f"\nDEBUG WRITE PFACE for 0x1FB197 ({len(pface_lines)}):")
for ln, content in pface_lines:
    print(f"  Line {ln}: {content}")
    
# Find all mentions of 1FB197 in debug context
mentions = [(i+1, line.rstrip()) for i, line in enumerate(lines) if '1fb197' in line.lower()]
print(f"\nALL mentions of 0x1FB197 ({len(mentions)}):")
for ln, content in mentions[:30]:
    print(f"  Line {ln}: {content}")
