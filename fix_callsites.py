import re

for f in ['src/io/dwg/dwg_stream_writers/object_writer/entities.rs', 'src/io/dwg/dwg_stream_writers/object_writer/mod.rs']:
    with open(f, 'r', encoding='utf-8') as fh:
        content = fh.read()
    
    # Pattern 1: sub-entities using ExtendedData::default()
    content_new = re.sub(
        r'(            1\.0,\n)(            &crate::xdata::ExtendedData::default\(\),)',
        r'\g<1>            "ByLayer",\n\2',
        content
    )
    
    # Pattern 2: block begin/end using &common.extended_data
    content_new = re.sub(
        r'(            common\.linetype_scale,\n)(            &common\.extended_data,)',
        r'\g<1>            &common.linetype,\n\2',
        content_new
    )
    
    if content_new != content:
        with open(f, 'w', encoding='utf-8') as fh:
            fh.write(content_new)
        print(f'{f}: updated')
    else:
        print(f'{f}: no changes')
