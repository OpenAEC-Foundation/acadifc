with open('src/io/dwg/dwg_stream_writers/object_writer/entities.rs', 'rb') as f:
    data = f.read()

idx = data.find(b'DWG entity streams always use SAT text (version 1).')
end_marker = b'self.writer.write_bit_long(0); // terminating empty block'
end_idx = data.find(end_marker, idx)
end_idx += len(end_marker)

old = data[idx:end_idx]
print('old length:', len(old))
print('old found:', old in data)

new = (
    b'Choose encoding: preserve SAB binary (version 2) when the\r\n'
    b'            // source data is already in binary form; otherwise write SAT\r\n'
    b'            // text (version 1).  Both are valid in DWG entity streams.\r\n'
    b'            if acis.is_binary && !acis.sab_data.is_empty() {\r\n'
    b'                // SAB binary path (R2007+): write version=2 + raw bytes.\r\n'
    b'                self.writer.write_bit_short(2_i16);\r\n'
    b'                self.writer.write_bit_long(acis.sab_data.len() as i32);\r\n'
    b'                self.writer.write_bytes(&acis.sab_data);\r\n'
    b'            } else {\r\n'
    b'                // SAT text path (version 1).\r\n'
    b'                self.writer.write_bit_short(1_i16);\r\n'
    b'\r\n'
    b'                // Obtain SAT text \xe2\x80\x94 convert from SAB if needed.\r\n'
    b'                let sat_text = if !acis.sat_data.is_empty() {\r\n'
    b'                    // Already have SAT text\r\n'
    b'                    acis.sat_data.clone()\r\n'
    b'                } else if !acis.sab_data.is_empty() {\r\n'
    b'                    // Convert SAB binary \xe2\x86\x92 SAT text via SabReader + SatDocument\r\n'
    b'                    match crate::entities::acis::SabReader::read(&acis.sab_data) {\r\n'
    b'                        Ok(sat_doc) => sat_doc.to_sat_string(),\r\n'
    b'                        Err(_) => String::new(),\r\n'
    b'                    }\r\n'
    b'                } else {\r\n'
    b'                    String::new()\r\n'
    b'                };\r\n'
    b'\r\n'
    b'                // SAT text \xe2\x80\x94 all DWG versions use the same encoding:\r\n'
    b'                // BL-sized blocks of encrypted bytes (cipher: 159 - byte)\r\n'
    b'                // terminated by BL(0).  Per LibreDWG dwg.spec.\r\n'
    b'                let stripped = AcisData::strip_sat_terminator(&sat_text);\r\n'
    b'                let mut full = stripped.clone();\r\n'
    b'                full.push_str("End-of-ACIS-data\\n");\r\n'
    b'                let plain = full.as_bytes();\r\n'
    b'\r\n'
    b'                // Encrypt with selective 159-substitution cipher\r\n'
    b'                // (per LibreDWG dwg.spec: bytes <= 32 pass through, bytes > 32: 159 - byte)\r\n'
    b'                let mut encrypted = Vec::with_capacity(plain.len());\r\n'
    b'                for &b in plain.iter() {\r\n'
    b'                    if b <= 32 {\r\n'
    b'                        encrypted.push(b);\r\n'
    b'                    } else {\r\n'
    b'                        encrypted.push(159u8.wrapping_sub(b));\r\n'
    b'                    }\r\n'
    b'                }\r\n'
    b'\r\n'
    b'                // Write as a single block + terminating BL(0)\r\n'
    b'                self.writer.write_bit_long(encrypted.len() as i32);\r\n'
    b'                self.writer.write_bytes(&encrypted);\r\n'
    b'                self.writer.write_bit_long(0); // terminating empty block\r\n'
    b'            }'
)

result = data.replace(old, new, 1)
print('replaced:', result != data)

with open('src/io/dwg/dwg_stream_writers/object_writer/entities.rs', 'wb') as f:
    f.write(result)
print('done')
