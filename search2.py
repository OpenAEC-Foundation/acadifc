with open('target/debug_stderr6.txt', 'r', encoding='utf-8', errors='replace') as f:
    for line in f:
        if 'DEBUG WRITE PFACE' in line and ('1FB197' in line or '1FB1AD' in line):
            print(line.rstrip())
