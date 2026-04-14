# Add owner_handle diagnostic to failing PolyfaceMesh entries
with open('examples/roundtrip_analysis.rs', 'rb') as f:
    data = f.read()

old = (
    b'            println!(\r\n'
    b'                "    [{}] vertices: {} \xe2\x86\x92 {}  faces: {} \xe2\x86\x92 {}  layer={:?}  orig_handle={:#X}  rt_handle={:#X}",\r\n'
    b'                i, ov, rv, of_, rf, o.common.layer, o.common.handle.value(), r.common.handle.value()\r\n'
    b'            );\r\n'
)
new = (
    b'            println!(\r\n'
    b'                "    [{}] v:{}\xe2\x86\x92{}  f:{}\xe2\x86\x92{}  layer={:?}  handle={:#X}\xe2\x86\x92{:#X}  owner={:#X}\xe2\x86\x92{:#X}",\r\n'
    b'                i, ov, rv, of_, rf, o.common.layer,\r\n'
    b'                o.common.handle.value(), r.common.handle.value(),\r\n'
    b'                o.common.owner_handle.value(), r.common.owner_handle.value()\r\n'
    b'            );\r\n'
)
print("old found:", old in data)
data2 = data.replace(old, new, 1)
print("replaced:", len(data2) != len(data))

with open('examples/roundtrip_analysis.rs', 'wb') as f:
    f.write(data2)
print("done")
