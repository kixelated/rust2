"""Recompute each mutant's selection with every unit's copy of the mutated fn.

Usage: recheck.py MOQ HEAD_BUILD_JSON HEAD_TESTS_JSON MUTANT_DIR
"""
import glob, json, os, sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

moq, head_json, head_tests, mdir = sys.argv[1:5]
head = {(r['unit'], r['name']): r for r in json.load(open(head_tests))}
build = ts.Build(moq, head_json, os.path.join(here, 'graphs'))
pkgs = ts.metadata(moq)
pkg_of = {k: u['pkg'] for k, u in build.units.items()}
out = []
for f in sorted(glob.glob(os.path.join(mdir, '*.json'))):
    if f.endswith(('.build.json', '.tests.json')):
        continue
    m = json.load(open(f))
    rows = build.defs[m['file']]
    lo, hi = next((r[0], r[1]) for r in rows if r[3] == m['def'])
    change = ts.Change()
    change.defs |= {r[3] for r in rows if (r[0], r[1]) == (lo, hi)}
    affected = ts.affected_tests(build, change)
    base = ts.dependents(pkgs, {ts.owner(m['file'], pkgs)})
    baseline = {t for t in head if pkg_of.get(t[0]) in base and head[t]['status'] == 0}
    ran = {(r['unit'], r['name']): r for r in json.load(open(f[:-5] + '.tests.json'))}
    failed = {t for t, r in ran.items() if r['status'] != 0 and t in baseline}
    selected = baseline & affected
    missed = sorted(failed - selected)
    row = {'def': ts.strip(m['def']), 'copies': len(change.defs), 'baseline': len(baseline), 'selected': len(selected),
           'failed': len(failed), 'missed': missed}
    out.append(row)
    print(row['def'], f"baseline {row['baseline']} selected {row['selected']} failed {row['failed']} missed {len(missed)}",
          missed[:3])
json.dump(out, open(os.path.join(mdir, 'recheck.json'), 'w'), indent=1)
