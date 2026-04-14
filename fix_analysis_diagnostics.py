# Enhance deep_polyface_mesh to show handles of failing meshes
with open('examples/roundtrip_analysis.rs', 'rb') as f:
    data = f.read()

old_print = (
    b'        if ov != rv || of_ != rf {\r\n'
    b'            println!(\r\n'
    b'                "    [{}] vertices: {} \xe2\x86\x92 {}  faces: {} \xe2\x86\x92 {}  layer={:?}",\r\n'
    b'                i, ov, rv, of_, rf, o.common.layer\r\n'
    b'            );\r\n'
    b'            lost_vertices += ov.saturating_sub(rv);\r\n'
    b'            lost_faces += of_.saturating_sub(rf);\r\n'
    b'        }\r\n'
)
new_print = (
    b'        if ov != rv || of_ != rf {\r\n'
    b'            println!(\r\n'
    b'                "    [{}] vertices: {} \xe2\x86\x92 {}  faces: {} \xe2\x86\x92 {}  layer={:?}  orig_handle={:#X}  rt_handle={:#X}",\r\n'
    b'                i, ov, rv, of_, rf, o.common.layer, o.common.handle.value(), r.common.handle.value()\r\n'
    b'            );\r\n'
    b'            lost_vertices += ov.saturating_sub(rv);\r\n'
    b'            lost_faces += of_.saturating_sub(rf);\r\n'
    b'        }\r\n'
)
print("old found:", old_print in data)
data = data.replace(old_print, new_print, 1)

with open('examples/roundtrip_analysis.rs', 'wb') as f:
    f.write(data)
print('done')
