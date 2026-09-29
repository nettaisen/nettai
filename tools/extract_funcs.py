#!/usr/bin/env python3
"""Extract function entry points and code labels from the bn6f disassembly.

Writes data/functions.tsv (addr, mode, name) and data/labels.tsv (addr, name)
for the recompiler. Addresses come from bn6f.sym; function boundaries and
ARM/Thumb mode come from the function macros in the .s sources.
"""
import os, re, sys

BN6F = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else "~/Documents/Programming/bn6f")
OUT = os.path.join(os.path.dirname(__file__), "..", "data")

syms = {}
for line in open(os.path.join(BN6F, "bn6f.sym")):
    parts = line.split()
    if len(parts) != 4:
        continue
    addr, _, _, name = parts
    syms.setdefault(name, int(addr, 16))

files = ["asm/start.s", "asm/main.s"] + sorted(
    os.path.join("asm", f) for f in os.listdir(os.path.join(BN6F, "asm"))
    if f.endswith(".s") and f not in ("start.s", "main.s"))
label_re = re.compile(r"^([A-Za-z_][A-Za-z0-9_.]*):")
funcs = {}
for f in files:
    mode = "thumb"
    pending = None
    text = open(os.path.join(BN6F, f), encoding="utf-8", errors="replace").read()
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    for raw in text.split("\n"):
        line = raw.split("//")[0].split("@")[0].strip()
        if not line:
            continue
        tok = line.split()[0]
        if tok in (".arm", ".code") and (tok == ".arm" or "32" in line):
            mode = "arm"
        elif tok == ".thumb" or (tok == ".code" and "16" in line):
            mode = "thumb"
        if tok in ("arm_func_start", "arm_local_start"):
            mode = "arm"; pending = "arm"; continue
        if tok in ("thumb_func_start", "thumb_local_start"):
            mode = "thumb"; pending = "thumb"; continue
        m = label_re.match(line)
        if m and pending:
            name = m.group(1)
            if name in syms:
                funcs[syms[name]] = (pending, name)
            else:
                print("warning: no symbol for", name, file=sys.stderr)
            pending = None

with open(os.path.join(OUT, "functions.tsv"), "w") as out:
    for addr in sorted(funcs):
        mode, name = funcs[addr]
        out.write(f"{addr:08x}\t{mode}\t{name}\n")

# Code-space labels (ROM and IWRAM), for naming blocks and functions.
with open(os.path.join(OUT, "labels.tsv"), "w") as out:
    seen = set()
    for name, addr in sorted(syms.items(), key=lambda kv: (kv[1], kv[0])):
        if (0x08000000 <= addr < 0x0a000000 or 0x03000000 <= addr < 0x03008000) and (addr, name) not in seen:
            seen.add((addr, name))
            out.write(f"{addr:08x}\t{name}\n")
print(len(funcs), "functions")

# Code references from data words (".word label" / ".word label+1"): candidate
# targets for computed jumps (switch tables) and function pointers.
word_re = re.compile(r"\.word\s+([A-Za-z_][A-Za-z0-9_.]*)\s*(\+\s*1)?\s*$")
refs = set()
for f in files:
    text = open(os.path.join(BN6F, f), encoding="utf-8", errors="replace").read()
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    for raw in text.split("\n"):
        line = raw.split("//")[0].strip()
        if ":" in line and line.split(":")[0].replace(".", "").replace("_", "").isalnum():
            line = line.split(":", 1)[1].strip()
        m = word_re.search(line)
        if m and m.group(1) in syms and not re.match(
                r"(byte|word|dword|hword|off|unk|str|comp|dat|a[A-Z0-9]|pt|ptr|tbl|jt)_?", m.group(1)) and (
                m.group(2) or re.match(r"(loc|locret|def|sub|nullsub|\.)", m.group(1)) or m.group(1)[0].isupper()):
            addr = syms[m.group(1)]
            if 0x08000000 <= addr < 0x08200000 or 0x03000000 <= addr < 0x03008000:
                refs.add((addr, 1 if m.group(2) else 0))
with open(os.path.join(OUT, "code_refs.tsv"), "w") as out:
    for addr, thumb in sorted(refs):
        out.write(f"{addr:08x}\t{thumb}\n")
print(len(refs), "code refs")
