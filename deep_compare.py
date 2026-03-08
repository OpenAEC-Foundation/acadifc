#!/usr/bin/env python3
"""Deep comparison of 3DSOLID entity structure between our output and reference."""

def parse_dxf_pairs(filepath):
    """Parse DXF into (gc, value) pairs."""
    with open(filepath, 'r', newline='') as f:
        content = f.read()
    lines = content.split('\r\n')
    pairs = []
    i = 0
    while i + 1 < len(lines):
        gc = lines[i].strip()
        val = lines[i+1]
        try:
            gc_int = int(gc)
        except ValueError:
            i += 1
            continue
        pairs.append((gc_int, val, i))
        i += 2
    return pairs

def extract_3dsolid_entity(pairs):
    """Extract all group codes for the first 3DSOLID entity."""
    result = []
    in_entity = False
    for gc, val, line_idx in pairs:
        if gc == 0 and val.strip() == '3DSOLID':
            in_entity = True
            result.append((gc, val, line_idx))
        elif in_entity:
            if gc == 0:  # next entity
                break
            result.append((gc, val, line_idx))
    return result

# Parse both files
our_pairs = parse_dxf_pairs('solid3d_box.dxf')
ref_pairs = parse_dxf_pairs('examples/sat v7 samples/UNIFIXT.dxf')

our_entity = extract_3dsolid_entity(our_pairs)
ref_entity = extract_3dsolid_entity(ref_pairs)

print("=== OUR 3DSOLID (non-SAT group codes) ===")
for gc, val, li in our_entity:
    if gc not in (1, 3):
        print(f"  gc={gc:>3}  val={repr(val.strip())}")

print(f"\n  Total gc=1 lines: {sum(1 for gc,_,_ in our_entity if gc == 1)}")
print(f"  Total gc=3 lines: {sum(1 for gc,_,_ in our_entity if gc == 3)}")

print("\n=== REFERENCE 3DSOLID (non-SAT group codes) ===")
for gc, val, li in ref_entity:
    if gc not in (1, 3):
        print(f"  gc={gc:>3}  val={repr(val.strip())}")

print(f"\n  Total gc=1 lines: {sum(1 for gc,_,_ in ref_entity if gc == 1)}")
print(f"  Total gc=3 lines: {sum(1 for gc,_,_ in ref_entity if gc == 3)}")

# Show first 3 and last 3 SAT lines for both
our_sat = [(gc, val) for gc, val, _ in our_entity if gc in (1, 3)]
ref_sat = [(gc, val) for gc, val, _ in ref_entity if gc in (1, 3)]

print("\n=== OUR first 5 SAT lines ===")
for gc, val in our_sat[:5]:
    print(f"  gc={gc}: {repr(val[:100])}")
print("  ...")
for gc, val in our_sat[-3:]:
    print(f"  gc={gc}: {repr(val[:100])}")

print("\n=== REFERENCE first 5 SAT lines ===")
for gc, val in ref_sat[:5]:
    print(f"  gc={gc}: {repr(val[:100])}")
print("  ...")
for gc, val in ref_sat[-3:]:
    print(f"  gc={gc}: {repr(val[:100])}")

# Decode and compare the actual SAT text
def decode_sat(encoded_text):
    result = []
    for c in encoded_text:
        b = ord(c)
        if 0x21 <= b <= 0x7E:
            result.append(chr(159 - b))
        else:
            result.append(c)
    return ''.join(result)

our_decoded_lines = [decode_sat(val) for gc, val in our_sat]
ref_decoded_lines = [decode_sat(val) for gc, val in ref_sat]

print("\n=== OUR decoded SAT first 5 lines ===")
for line in our_decoded_lines[:5]:
    print(f"  {repr(line[:120])}")
print("  ...")
for line in our_decoded_lines[-3:]:
    print(f"  {repr(line[:120])}")

print("\n=== REFERENCE decoded SAT first 5 lines ===")
for line in ref_decoded_lines[:5]:
    print(f"  {repr(line[:120])}")
print("  ...")
for line in ref_decoded_lines[-3:]:
    print(f"  {repr(line[:120])}")

# Check exact gc formatting in raw file
print("\n=== RAW BYTES around 3DSOLID start in OUR file ===")
with open('solid3d_box.dxf', 'rb') as f:
    data = f.read()
idx = data.find(b'3DSOLID')
if idx >= 0:
    # Show 200 bytes before and 400 after
    start = max(0, idx - 50)
    end = min(len(data), idx + 400)
    chunk = data[start:end]
    print(f"  Offset range: {start}-{end}")
    for i in range(0, len(chunk), 80):
        line = chunk[i:i+80]
        hex_str = ' '.join(f'{b:02X}' for b in line[:40])
        asc_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in line[:40])
        print(f"  {start+i:6d}: {hex_str}")
        print(f"          {asc_str}")

print("\n=== RAW BYTES around 3DSOLID start in REFERENCE file ===")
with open('examples/sat v7 samples/UNIFIXT.dxf', 'rb') as f:
    ref_data = f.read()
ref_idx = ref_data.find(b'3DSOLID')
if ref_idx >= 0:
    start = max(0, ref_idx - 50)
    end = min(len(ref_data), ref_idx + 400)
    chunk = ref_data[start:end]
    print(f"  Offset range: {start}-{end}")
    for i in range(0, len(chunk), 80):
        line = chunk[i:i+80]
        hex_str = ' '.join(f'{b:02X}' for b in line[:40])
        asc_str = ''.join(chr(b) if 32 <= b < 127 else '.' for b in line[:40])
        print(f"  {start+i:6d}: {hex_str}")
        print(f"          {asc_str}")

# Check: does reference have gc=1 BEFORE gc=70 or after?
print("\n=== GROUP CODE ORDER in 3DSOLID ===")
print("OUR order (non-SAT):")
in_sat = False
sat_start = None
sat_end = None
for i, (gc, val, li) in enumerate(our_entity):
    if gc in (1, 3):
        if not in_sat:
            sat_start = i
            in_sat = True
        sat_end = i
    else:
        if in_sat:
            print(f"  [SAT data: indices {sat_start}-{sat_end}]")
            in_sat = False
        print(f"  [{i}] gc={gc} val={repr(val.strip()[:60])}")
if in_sat:
    print(f"  [SAT data: indices {sat_start}-{sat_end}]")

print("\nREF order (non-SAT):")
in_sat = False
sat_start = None
sat_end = None
for i, (gc, val, li) in enumerate(ref_entity):
    if gc in (1, 3):
        if not in_sat:
            sat_start = i
            in_sat = True
        sat_end = i
    else:
        if in_sat:
            print(f"  [SAT data: indices {sat_start}-{sat_end}]")
            in_sat = False
        print(f"  [{i}] gc={gc} val={repr(val.strip()[:60])}")
if in_sat:
    print(f"  [SAT data: indices {sat_start}-{sat_end}]")
