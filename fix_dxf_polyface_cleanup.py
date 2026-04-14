# Clean up warnings in the new read_polyline_entity function
with open('src/io/dxf/reader/section_reader.rs', 'rb') as f:
    data = f.read()

# Remove the spurious 'let mut v = PolyfaceVertex' that was duplicated
old_dup = (
    b'                    } else if (vflags & 64) != 0 {\r\n'
    b'                        // Geometry vertex\r\n'
    b'                        let mut v = PolyfaceVertex {\r\n'
    b'                            common: EntityCommon::default(),\r\n'
    b'                            location: crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                            flags: PolyfaceVertexFlags::from_bits_truncate(vflags),\r\n'
    b'                            bulge: 0.0,\r\n'
    b'                            start_width: 0.0,\r\n'
    b'                            end_width: 0.0,\r\n'
    b'                            curve_tangent: 0.0,\r\n'
    b'                            id: 0,\r\n'
    b'                        };\r\n'
    b'                        let _ = v;\r\n'
    b'                        pface_vertices.push(PolyfaceVertex {'
)
new_dup = (
    b'                    } else if (vflags & 64) != 0 {\r\n'
    b'                        // Geometry vertex\r\n'
    b'                        pface_vertices.push(PolyfaceVertex {'
)
print("dup found:", old_dup in data)
data = data.replace(old_dup, new_dup, 1)

# Rename vertex_count and face_count to _vertex_count and _face_count to suppress warnings
data = data.replace(
    b'        let mut vertex_count: i16 = 0;\r\n'
    b'        let mut face_count: i16 = 0;\r\n',
    b'        let mut _vertex_count: i16 = 0;\r\n'
    b'        let mut _face_count: i16 = 0;\r\n',
    1
)
data = data.replace(
    b'                            vertex_count = vc;\r\n',
    b'                            _vertex_count = vc;\r\n',
    1
)
data = data.replace(
    b'                            face_count = fc;\r\n',
    b'                            _face_count = fc;\r\n',
    1
)

with open('src/io/dxf/reader/section_reader.rs', 'wb') as f:
    f.write(data)
print('done')
