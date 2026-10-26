"""How much of each selection's CPU the slowest tests at HEAD take: heavy.py REPLAY_DIR HEAD_TESTS_JSON."""
import glob, json, os, sys
from collections import Counter

head = json.load(open(sys.argv[2]))
ranked = sorted(head, key=lambda r: -r['cpu'])
top = ranked[:len(ranked) // 100]
total = sum(r['cpu'] for r in head)
print(f'{len(top)} slowest of {len(head)} tests: {sum(r["cpu"] for r in top):.0f} of {total:.0f} CPU s;'
      f' slowest {ranked[0]["unit"]} {ranked[0]["name"]} {ranked[0]["cpu"]:.1f} s')
by_crate = Counter()
for r in top:
    by_crate[r['unit'].rsplit('-', 1)[0]] += r['cpu']
print('slowest 1% by unit:', [(k, round(v)) for k, v in by_crate.most_common(8)])
cpu = {(r['unit'], r['name']): r['cpu'] for r in head}
heavy = {(r['unit'], r['name']) for r in top}
acc = Counter()
for f in glob.glob(os.path.join(sys.argv[1], '*.json')):
    if f.endswith(('.build.json', '.run.json')):
        continue
    r = json.load(open(f))
    for k in ('baseline_tests', 'mir_scrubbed', 'mir_fs'):
        for t in r.get(k, []):
            t = tuple(t)
            if t in cpu:
                acc[k] += cpu[t]
                if t in heavy:
                    acc[k + ' heavy'] += cpu[t]
for k in ('baseline_tests', 'mir_scrubbed', 'mir_fs'):
    if acc[k]:
        print(f'{k}: slowest 1% take {100 * acc[k + " heavy"] / acc[k]:.0f}% of its measured CPU')
