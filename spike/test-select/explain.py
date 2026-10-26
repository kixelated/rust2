"""Why the MIR diff selects what it does for one replayed commit: explain.py BUILT_MOQ REPLAY_DIR COMMIT [N]."""
import json, os, re, shutil, subprocess, sys
from collections import Counter

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

built, rdir, commit = sys.argv[1:4]
show = int(sys.argv[4]) if len(sys.argv) > 4 else 40
r = json.load(open(os.path.join(rdir, commit + '.json')))
work = os.path.join(here, 'explain-work')


def load(c):
    d = os.path.join(work, c)
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    subprocess.run(['tar', '--zstd', '-xf', os.path.join(rdir, 'snap', c + '.tar.zst'), '-C', d], check=True)
    b = ts.Build(built, os.path.join(rdir, c + '.build.json'), d)
    shutil.rmtree(d)
    return b


base, head = load(r['base']), load(commit)
why = Counter()
rows = []
for sym in head.node_def:
    h = head.node_hash.get(sym)
    if sym not in base.node_def:
        kind = 'new'
    elif h is not None and (base.node_hash.get(sym) is None or base.node_hash[sym][1] != h[1]):
        kind = 'body'
    elif head.edges.get(sym, set()) != base.edges.get(sym, set()):
        kind = 'callees'
    else:
        continue
    d = ts.strip(head.node_def[sym])
    why[kind, d.split('::')[0]] += 1
    rows.append((kind, d, sym))
for k, n in why.most_common():
    print(n, *k)
print()
for kind, d, sym in sorted(rows, key=lambda x: (x[0], x[1]))[:show]:
    extra = ''
    if kind == 'callees':
        added = head.edges.get(sym, set()) - base.edges.get(sym, set())
        removed = base.edges.get(sym, set()) - head.edges.get(sym, set())
        extra = f' +{[ts.strip(head.node_def.get(x, x))[:60] for x in list(added)[:2]]} -{[ts.strip(base.node_def.get(x, x))[:60] for x in list(removed)[:2]]}'
    print(kind, d[:120], extra)
