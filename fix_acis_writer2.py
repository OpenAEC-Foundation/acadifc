with open('src/io/dwg/dwg_stream_writers/object_writer/entities.rs', 'rb') as f:
    c = f.read()

old_s = b'        // acis_empty_bit \xc3\xa2\xe2\x82\xac\xe2\x80\x9d second copy of the "empty" flag.\r\n        // Must match the acis_empty bit written above.\r\n        self.writer.write_bit(acds);\r\n\r\n        // R2007+: unknown BL field (COMMON_3DSOLID)\r\n        if self.version.r2007_plus() {\r\n            self.writer.write_bit_long(0);\r\n        }\r\n\r\n        // 3DSOLID R2007+: history_id handle'

new_s = b'        // acis_empty_bit: for SAB (binary) entities this bit is NOT written here.\r\n        if !is_sab {\r\n            self.writer.write_bit(acds);\r\n        }\r\n\r\n        // R2007+: unknown BL field (COMMON_3DSOLID)\r\n        if self.version.r2007_plus() {\r\n            self.writer.write_bit_long(0);\r\n        }\r\n\r\n        // 3DSOLID R2007+: history_id handle'

if old_s in c:
    c = c.replace(old_s, new_s, 1)
    print('solid3d OK')
else:
    print('MISS')
    idx = c.find(b'second copy of the')
    print(repr(c[idx-50:idx+200]))

with open('src/io/dwg/dwg_stream_writers/object_writer/entities.rs', 'wb') as f:
    f.write(c)
