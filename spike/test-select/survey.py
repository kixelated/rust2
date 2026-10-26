"""Baseline selection over the last N first-parent commits, with HEAD's package graph.

Usage: survey.py MOQ N OUT_JSON
"""
import json, os, sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ts

moq, n, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
pkgs = ts.metadata(moq)
commits = ts.git(moq, 'log', '--first-parent', f'-{n}', '--format=%H %P', 'main').split('\n')
rows = []
for line in commits:
    if not line:
        continue
    h, parent = line.split()[:2]
    paths = ts.git(moq, 'diff', '--name-only', '--no-renames', parent, h).split()
    base = ts.baseline(paths, pkgs)
    kind = 'none' if base is None else 'ALL' if base == 'ALL' else 'some'
    rows.append({'commit': h, 'parent': parent, 'paths': paths, 'baseline': kind,
                 'packages': sorted(base) if kind == 'some' else []})
json.dump(rows, open(out, 'w'), indent=0)
print(Counter(r['baseline'] for r in rows))
print(Counter(len(r['packages']) for r in rows if r['baseline'] == 'some'))
