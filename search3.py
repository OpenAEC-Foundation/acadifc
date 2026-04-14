with open('target/stderr7.txt', 'r', encoding='utf-8', errors='replace') as f:
    for line in f:
        if 'DEBUG WRITE PFACE' in line and '1FB197' in line:
            print(repr(line.rstrip()))
