"""Symbols that several units of one build codegen with different MIR hashes: consistency.py GRAPH_DIR."""
import glob, json, sys
from collections import defaultdict

hashes, defs = defaultdict(set), {}
for f in glob.glob(sys.argv[1] + '/*.json'):
    d = json.load(open(f))
    for n in d['nodes']:
        if len(n) > 2:
            hashes[n[0]].add(tuple(n[2:]))
            defs[n[0]] = n[1]
shared = lambda s: not s.startswith('_R') or '7___rustc' in s
multi = [s for s, v in hashes.items() if len(v) > 1]
print(len(hashes), 'codegened;', len(multi), 'differ;', sum(1 for s in multi if not shared(s)), 'of them crate-mangled')
for s in [s for s in multi if not shared(s)][:10]:
    print(' ', defs[s])
