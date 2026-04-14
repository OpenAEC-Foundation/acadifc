# Fix DXF PolyfaceMesh vertex detection: check bit 64 (POLYGON_MESH) BEFORE bit 128 (POLYFACE_MESH)
# Geometry vertices have flag=192 (64|128), face records have flag=128
# Current code checks bit 128 first, so 192 → face record (WRONG!)
# Fix: check bit 64 first → geometry vertex wins when both bits set
with open('src/io/dxf/reader/section_reader.rs', 'rb') as f:
    data = f.read()

old = (
    b'                    // PolyfaceVertexFlags::POLYFACE_MESH = 128 => face record\r\n'
    b'                    // PolyfaceVertexFlags::POLYGON_MESH  = 64  => geometry vertex\r\n'
    b'                    if (vflags & 128) != 0 {\r\n'
    b'                        // Face record\r\n'
    b'                        let mut face = PolyfaceFace {\r\n'
    b'                            common: EntityCommon::default(),\r\n'
    b'                            flags: PolyfaceVertexFlags::NONE,\r\n'
    b'                            index1: vi1,\r\n'
    b'                            index2: vi2,\r\n'
    b'                            index3: vi3,\r\n'
    b'                            index4: vi4,\r\n'
    b'                            color: None,\r\n'
    b'                        };\r\n'
    b'                        face.flags = PolyfaceVertexFlags::from_bits_truncate(vflags);\r\n'
    b'                        pface_faces.push(face);\r\n'
    b'                    } else if (vflags & 64) != 0 {\r\n'
    b'                        // Geometry vertex\r\n'
    b'                        pface_vertices.push(PolyfaceVertex {\r\n'
)
print("old found:", old in data)

new = (
    b'                    // Geometry vertex detection: bit 6 (64 = POLYGON_MESH) trumps bit 7\r\n'
    b'                    // (128 = POLYFACE_MESH).  Internally vertices are stored with\r\n'
    b'                    // flags=128, then ORed with 64 by the writer => written flag = 192.\r\n'
    b'                    // Face records are written with flag=128 only.\r\n'
    b'                    // Therefore: check bit 64 FIRST.\r\n'
    b'                    if (vflags & 64) != 0 {\r\n'
    b'                        // Geometry vertex\r\n'
    b'                        pface_vertices.push(PolyfaceVertex {\r\n'
)
data2 = data.replace(old, new, 1)
print("replaced:", len(data2) != len(data))

# Now fix the else-if that follows (it was the closing of the old else-if)
# After the geometry vertex block, the old code had no more conditions (it fell to polyline vertex)
# Let's check what comes after the geometry vertex push
old2 = (
    b'                    } else {\r\n'
    b'                        polyline_vertices.push(Vertex3D::new(\r\n'
    b'                            crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                        ));\r\n'
    b'                    }\r\n'
)
# Find what's immediately after the geometry vertex block in the section we care about
# We need to add the face record check as the else-if after the geometry vertex block
old3 = (
    b'                        pface_vertices.push(PolyfaceVertex {\r\n'
    b'                            common: EntityCommon::default(),\r\n'
    b'                            location: crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                            flags: PolyfaceVertexFlags::from_bits_truncate(vflags),\r\n'
    b'                            bulge: 0.0,\r\n'
    b'                            start_width: 0.0,\r\n'
    b'                            end_width: 0.0,\r\n'
    b'                            curve_tangent: 0.0,\r\n'
    b'                            id: 0,\r\n'
    b'                        });\r\n'
    b'                    } else {\r\n'
    b'                        polyline_vertices.push(Vertex3D::new(\r\n'
    b'                            crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                        ));\r\n'
    b'                    }\r\n'
)
new3 = (
    b'                        pface_vertices.push(PolyfaceVertex {\r\n'
    b'                            common: EntityCommon::default(),\r\n'
    b'                            location: crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                            flags: PolyfaceVertexFlags::from_bits_truncate(vflags),\r\n'
    b'                            bulge: 0.0,\r\n'
    b'                            start_width: 0.0,\r\n'
    b'                            end_width: 0.0,\r\n'
    b'                            curve_tangent: 0.0,\r\n'
    b'                            id: 0,\r\n'
    b'                        });\r\n'
    b'                    } else if (vflags & 128) != 0 {\r\n'
    b'                        // Face record (only bit 128 set, no bit 64)\r\n'
    b'                        let mut face = PolyfaceFace {\r\n'
    b'                            common: EntityCommon::default(),\r\n'
    b'                            flags: PolyfaceVertexFlags::NONE,\r\n'
    b'                            index1: vi1,\r\n'
    b'                            index2: vi2,\r\n'
    b'                            index3: vi3,\r\n'
    b'                            index4: vi4,\r\n'
    b'                            color: None,\r\n'
    b'                        };\r\n'
    b'                        face.flags = PolyfaceVertexFlags::from_bits_truncate(vflags);\r\n'
    b'                        pface_faces.push(face);\r\n'
    b'                    } else {\r\n'
    b'                        polyline_vertices.push(Vertex3D::new(\r\n'
    b'                            crate::types::Vector3::new(vx, vy, vz),\r\n'
    b'                        ));\r\n'
    b'                    }\r\n'
)
print("old3 found:", old3 in data2)
data3 = data2.replace(old3, new3, 1)
print("replaced3:", len(data3) != len(data2))

with open('src/io/dxf/reader/section_reader.rs', 'wb') as f:
    f.write(data3)
print("done")
