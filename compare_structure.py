#!/usr/bin/env python3
"""Compare OBJECTS section between our DXF and ringo.dxf"""

def parse_objects(filepath):
    with open(filepath, 'r', newline='') as f:
        content = f.read()
    lines = content.split('\r\n')
    in_objects = False
    objects = []
    current = None
    i = 0
    while i + 1 < len(lines):
        gc = lines[i].strip()
        val = lines[i+1].strip()
        if gc == '2' and val == 'OBJECTS':
            in_objects = True
            i += 2
            continue
        if gc == '0' and val == 'ENDSEC' and in_objects:
            if current:
                objects.append(current)
            break
        if in_objects:
            if gc == '0':
                if current:
                    objects.append(current)
                current = {'type': val, 'pairs': []}
            elif current:
                current['pairs'].append((gc, val))
        i += 2
    return objects

our_obj = parse_objects('solid3d_box.dxf')
ref_obj = parse_objects('examples/sat v7 samples/ringo.dxf')

def print_objects(label, objects):
    print(f'{label}: {len(objects)} objects')
    for o in objects:
        handle = next((v for g,v in o['pairs'] if g=='5'), '?')
        tp = o['type']
        print(f'  {tp} handle={handle} ({len(o["pairs"])} pairs)')
        for gc, val in o['pairs']:
            if gc in ('3', '350', '360'):
                print(f'    gc={gc}: {val}')

print_objects('OUR OBJECTS', our_obj)
print()
print_objects('REF OBJECTS', ref_obj)

# Also compare ENTITIES section structure (non-SAT)
print("\n=== ENTITIES COMPARISON ===")
def get_entities(filepath):
    with open(filepath, 'r', newline='') as f:
        content = f.read()
    lines = content.split('\r\n')
    in_entities = False
    entities = []
    i = 0
    while i + 1 < len(lines):
        gc = lines[i].strip()
        val = lines[i+1].strip()
        if gc == '2' and val == 'ENTITIES':
            in_entities = True
        elif gc == '0' and val == 'ENDSEC' and in_entities:
            break
        elif in_entities and gc == '0':
            entities.append(val)
        i += 2
    return entities

our_ent = get_entities('solid3d_box.dxf')
ref_ent = get_entities('examples/sat v7 samples/ringo.dxf')
print(f'Our entities: {our_ent}')
print(f'Ref entities: {ref_ent}')

# Compare TABLES structure
print("\n=== TABLES COMPARISON ===")
def get_tables(filepath):
    with open(filepath, 'r', newline='') as f:
        content = f.read()
    lines = content.split('\r\n')
    in_tables = False
    tables = []
    i = 0
    while i + 1 < len(lines):
        gc = lines[i].strip()
        val = lines[i+1].strip()
        if gc == '2' and val == 'TABLES':
            in_tables = True
        elif gc == '0' and val == 'ENDSEC' and in_tables:
            break
        elif in_tables and gc == '0' and val == 'TABLE':
            # Next pair is gc=2 with table name
            if i + 3 < len(lines):
                tname = lines[i+3].strip()
                tables.append(tname)
        i += 2
    return tables

our_tables = get_tables('solid3d_box.dxf')
ref_tables = get_tables('examples/sat v7 samples/ringo.dxf')
print(f'Our tables: {our_tables}')
print(f'Ref tables: {ref_tables}')
