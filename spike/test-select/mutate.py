"""Check the selector against real failures: panic in one workspace fn, rebuild, run the
baseline tests of its package, and confirm every test that fails was selected.

Usage: mutate.py MOQ HEAD_BUILD_JSON HEAD_TESTS_JSON OUTDIR SEED COUNT
"""
import json, os, random, re, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

moq, head_json, head_tests, outdir, seed, count = sys.argv[1:7]
os.makedirs(outdir, exist_ok=True)
rng = random.Random(int(seed))
head = {(r['unit'], r['name']): r for r in json.load(open(head_tests))}
build = ts.Build(moq, head_json, os.path.join(here, 'graphs'))
pkgs = ts.metadata(moq)
pkg_of = {k: u['pkg'] for k, u in build.units.items()}

# Candidate fns: plain fns and methods in a library's own source (not tests), reached by a test.
lib_files = {f for f, keys in build.files.items() if any(not build.units[k]['test'] and 'lib' in build.units[k]['kind'] for k in keys)}
cands = []
for f, rows in build.defs.items():
    if f not in lib_files or '/tests/' in f or f.endswith(('tests.rs', 'test.rs')):
        continue
    for lo, hi, kind, dfn in rows:
        if kind in ('Fn', 'AssocFn') and hi - lo >= 2 and '::tests::' not in dfn and '::test::' not in dfn:
            cands.append((f, lo, hi, dfn))
rng.shuffle(cands)

done = 0
for f, lo, hi, dfn in cands:
    if done >= int(count):
        break
    text = open(os.path.join(moq, f)).read().split('\n')
    # The body opens on the first line from the signature that ends with `{`.
    body = next((i for i in range(lo - 1, hi) if text[i].rstrip().endswith('{')), None)
    sig = ' '.join(text[lo - 1:(body or lo - 1) + 1])
    if body is None or 'const fn' in sig or re.search(r'\bunsafe\s+extern|extern\s+"', sig):
        continue
    change = ts.Change()
    # The fn is compiled once per unit that includes its file: a library and its unit tests.
    change.defs |= {r[3] for r in build.defs[f] if (r[0], r[1]) == (lo, hi)}
    affected = ts.affected_tests(build, change)
    if not affected:
        continue
    pkg = ts.owner(f, pkgs)
    base = ts.dependents(pkgs, {pkg})
    baseline = {t for t in head if pkg_of.get(t[0]) in base and head[t]['status'] == 0}
    selected = baseline & affected
    if len(selected) == len(baseline):
        continue
    name = re.sub(r'\W+', '_', ts.strip(dfn))[:80]
    print(f'mutant {dfn} at {f}:{body + 1}: baseline {len(baseline)}, selected {len(selected)}', flush=True)
    original = '\n'.join(text)
    text.insert(body + 1, 'if ::std::hint::black_box(true) { panic!("rust2 mutant") }')
    open(os.path.join(moq, f), 'w').write('\n'.join(text))
    try:
        bjson = os.path.join(outdir, name + '.build.json')
        status = subprocess.run([os.path.join(here, 'bin/moq-build'), moq, os.path.join(here, 'graphs'),
                                 os.path.join(here, 'moq-target'), bjson], stdout=subprocess.DEVNULL,
                                stderr=open(os.path.join(outdir, name + '.log'), 'w')).returncode
        if status != 0:
            print('  build failed', flush=True)
            continue
        results = os.path.join(outdir, name + '.tests.json')
        subprocess.run(['python3', os.path.join(here, 'runtests.py'), moq, bjson, os.path.join(here, 'graphs'), results, '6',
                        ','.join(sorted(base))], check=True, stdout=subprocess.DEVNULL)
        ran = {(r['unit'], r['name']): r for r in json.load(open(results))}
        failed = {t for t, r in ran.items() if r['status'] != 0 and t in baseline}
        missed = failed - selected
        report = {'def': dfn, 'file': f, 'line': body + 1, 'baseline': len(baseline), 'selected': len(selected),
                  'failed': len(failed), 'failed_selected': len(failed & selected), 'missed': sorted(missed)}
        json.dump(report, open(os.path.join(outdir, name + '.json'), 'w'))
        print(f'  failed {len(failed)}, of them selected {len(failed & selected)}, missed {sorted(missed)[:5]}', flush=True)
        done += 1
    finally:
        open(os.path.join(moq, f), 'w').write(original)
