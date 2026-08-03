import os
import re

root = "bindings/typescript/src"
missing = []
pat = re.compile(r'from\s+"(\.[^"]+)"')
for dirpath, _, files in os.walk(root):
    for f in files:
        if not f.endswith((".ts", ".tsx")):
            continue
        p = os.path.join(dirpath, f)
        with open(p) as fh:
            content = fh.read()
        for m in pat.finditer(content):
            spec = m.group(1)
            base = os.path.normpath(os.path.join(dirpath, spec))
            cands = [base + ".ts", base + ".tsx", base + "/index.ts", base]
            if not any(os.path.exists(c) for c in cands):
                missing.append((p, spec))
print("unresolved relative imports:", len(missing))
for p, s in missing:
    print(" ", p, "->", s)
