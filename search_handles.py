import sys

handles_of_interest = {
    "229116", "229117", "229118", "229119", "22911a", "22911b", "22911c", "22911d",
    "22911e", "22911f", "229120", "229121", "229122", "229123", "229124", "229125",
    "229126", "229127", "229128", "229129", "22912a",  # faces + seqend
    "229130", "229131",  # mesh[1] vertices might start here
}

with open("target/debug_stderr6.txt", "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")

found = []
for i, line in enumerate(lines):
    line_low = line.lower()
    for h in handles_of_interest:
        if h in line_low:
            found.append((i+1, line.rstrip()))
            break

print(f"Found {len(found)} mentions:")
for ln, content in found[:50]:
    print(f"  Line {ln}: {content}")
