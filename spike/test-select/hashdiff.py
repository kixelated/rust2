"""Nodes whose MIR hash or callees differ between two graph dirs: hashdiff.py A B [pattern]."""
import glob, json, os, re, sys


def load(d):
    nodes, edges = {}, {}
    for f in glob.glob(os.path.join(d, '*.json')):
        g = json.load(open(f))
        syms = [n[0] for n in g['nodes']]
        for n in g['nodes']:
            if len(n) > 2 or n[0] not in nodes:
                nodes[n[0]] = (n[1], tuple(n[2:]) if len(n) > 2 else nodes.get(n[0], (None, None))[1])
        for a, bs in g['edges']:
            edges.setdefault(syms[a], set()).update(syms[b] for b in bs)
    return nodes, edges


a, ea = load(sys.argv[1])
b, eb = load(sys.argv[2])
pat = re.compile(sys.argv[3] if len(sys.argv) > 3 else '')
strip = lambda s: re.sub(r'\[[0-9a-f]+\]', '', s)
for sym, (dfn, h) in sorted(b.items(), key=lambda x: x[1][0]):
    if not pat.search(strip(dfn)):
        continue
    if sym not in a:
        print('new    ', strip(dfn))
    elif h != a[sym][1]:
        print('body   ' if h[1] != a[sym][1][1] else 'line!  ', strip(dfn))
    elif eb.get(sym, set()) != ea.get(sym, set()):
        print('callees', strip(dfn))
