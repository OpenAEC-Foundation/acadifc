# Fix DXF dimension reader: apply text_middle_point, definition_point, and line_spacing_factor
with open('src/io/dxf/reader/section_reader.rs', 'rb') as f:
    data = f.read()

# 1. Find the read_dimension function and add line_spacing_factor parsing
# Look for the rotation reader entry and add line_spacing_factor nearby
old_rotation = (
    b'                50 => {\r\n'
    b'                    if let Some(rot) = pair.as_double() {\r\n'
    b'                        rotation = rot;\r\n'
    b'                    }\r\n'
    b'                }'
)
new_rotation = (
    b'                50 => {\r\n'
    b'                    if let Some(rot) = pair.as_double() {\r\n'
    b'                        rotation = rot;\r\n'
    b'                    }\r\n'
    b'                }\r\n'
    b'                44 => {\r\n'
    b'                    if let Some(lsf) = pair.as_double() {\r\n'
    b'                        line_spacing_factor = lsf;\r\n'
    b'                    }\r\n'
    b'                }'
)
print('rotation block found:', old_rotation in data)
data = data.replace(old_rotation, new_rotation, 1)

# 2. Add line_spacing_factor variable after 'leader_length'
old_leader_len_decl = (
    b'        let mut leader_length = 0.0;\r\n'
    b'        let mut common = EntityCommon::new();'
)
new_leader_len_decl = (
    b'        let mut leader_length = 0.0;\r\n'
    b'        let mut line_spacing_factor = 1.0f64;\r\n'
    b'        let mut common = EntityCommon::new();'
)
print('leader_length decl found:', old_leader_len_decl in data)
data = data.replace(old_leader_len_decl, new_leader_len_decl, 1)

# 3. In the common block at the end, also apply text_middle_point, definition_point, line_spacing_factor
old_common = (
    b'            dc.common.transparency = common.transparency;\r\n'
    b'        }\r\n'
    b'\r\n'
    b'        Ok(Some(dimension))\r\n'
    b'    }\r\n'
    b'\r\n'
    b'    /// Read a HATCH entity'
)
new_common = (
    b'            dc.common.transparency = common.transparency;\r\n'
    b'            if let Some(pt) = text_middle_point.get_point() {\r\n'
    b'                dc.text_middle_point = pt;\r\n'
    b'            }\r\n'
    b'            if let Some(pt) = definition_point.get_point() {\r\n'
    b'                dc.definition_point = pt;\r\n'
    b'            }\r\n'
    b'            if line_spacing_factor != 1.0 {\r\n'
    b'                dc.line_spacing_factor = line_spacing_factor;\r\n'
    b'            }\r\n'
    b'        }\r\n'
    b'\r\n'
    b'        Ok(Some(dimension))\r\n'
    b'    }\r\n'
    b'\r\n'
    b'    /// Read a HATCH entity'
)
print('common block found:', old_common in data)
data = data.replace(old_common, new_common, 1)

with open('src/io/dxf/reader/section_reader.rs', 'wb') as f:
    f.write(data)
print('done')
