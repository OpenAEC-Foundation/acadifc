# Fix DXF dimension writer: add line_spacing_factor write
with open('src/io/dxf/writer/section_writer.rs', 'rb') as f:
    data = f.read()

# Add line_spacing_factor (group code 44) in write_dimension_base before Ok(())
old_dim_base_end = (
    b'        self.writer.write_string(3, &base.style_name)?;\r\n'
    b'        if !base.text.is_empty() {\r\n'
    b'            self.writer.write_string(1, &base.text)?;\r\n'
    b'        }\r\n'
    b'        Ok(())\r\n'
    b'    }\r\n'
    b'\r\n'
    b'    fn write_dimension_aligned'
)
new_dim_base_end = (
    b'        self.writer.write_string(3, &base.style_name)?;\r\n'
    b'        if !base.text.is_empty() {\r\n'
    b'            self.writer.write_string(1, &base.text)?;\r\n'
    b'        }\r\n'
    b'        if (base.line_spacing_factor - 1.0).abs() > 1e-10 {\r\n'
    b'            self.writer.write_double(44, base.line_spacing_factor)?;\r\n'
    b'        }\r\n'
    b'        Ok(())\r\n'
    b'    }\r\n'
    b'\r\n'
    b'    fn write_dimension_aligned'
)
print('dim base end found:', old_dim_base_end in data)
data = data.replace(old_dim_base_end, new_dim_base_end, 1)

with open('src/io/dxf/writer/section_writer.rs', 'wb') as f:
    f.write(data)
print('done')
