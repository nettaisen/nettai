#!/usr/bin/env python3
"""Name addresses using the bn6f symbol table: sym.py ADDR... -> nearest symbol at or below."""
import bisect, os, sys
SYM = os.path.expanduser("~/Documents/Programming/bn6f/bn6f.sym")
entries = []
for line in open(SYM):
    p = line.split()
    if len(p) == 4 and not p[3].startswith("."):
        entries.append((int(p[0], 16), p[3]))
entries.sort()
keys = [e[0] for e in entries]
for arg in sys.argv[1:]:
    a = int(arg, 16)
    i = bisect.bisect_right(keys, a) - 1
    # prefer non-local names among equal addresses
    addr, name = entries[i]
    print(f"{a:#010x} = {name}+{a - addr:#x}")
