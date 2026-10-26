"""Summarize the replay: what a function-level selector skips beyond moq's package selection.

Usage: report.py SURVEY_JSON REPLAY_DIR HEAD_TESTS_JSON
"""
import glob, json, os, statistics, sys
from collections import Counter, defaultdict

survey, rdir, head_tests = sys.argv[1:4]
head = json.load(open(head_tests))
by_key = {(r['unit'], r['name']): r['cpu'] for r in head}
by_crate = {}
crate_cpu = defaultdict(list)
for r in head:
    crate = r['unit'].rsplit('-', 1)[0]
    by_crate[(crate, r['name'])] = r['cpu']
    crate_cpu[crate].append(r['cpu'])
median_all = statistics.median(r['cpu'] for r in head)
imputed = Counter()


def cpu(t):
    key, name = t
    if (key, name) in by_key:
        return by_key[(key, name)]
    crate = key.rsplit('-', 1)[0]
    if (crate, name) in by_crate:
        return by_crate[(crate, name)]
    imputed['tests'] += 1
    return statistics.median(crate_cpu[crate]) if crate in crate_cpu else median_all


rows = []
for f in sorted(glob.glob(os.path.join(rdir, '*.json'))):
    if f.endswith(('.build.json', '.run.json')):
        continue
    r = json.load(open(f))
    if 'error' in r:
        print('skipped', r['commit'][:10], r['error'])
        continue
    rows.append(r)

order = {r['commit']: i for i, r in enumerate(json.load(open(survey)))}
rows.sort(key=lambda r: order[r['commit']])
oldest = max(order[r['commit']] for r in rows)
window = json.load(open(survey))[:oldest + 1]
kinds = Counter(r['baseline'] for r in window)
print(f'window: {len(window)} commits ({kinds["some"]} select some packages, {kinds["ALL"]} everything, '
      f'{kinds["none"]} no Rust tests); replayed {len(rows)}')

METHODS = [
    ('baseline_tests', 'package selection (today)'),
    ('selected', 'changed lines; item edits fall back to the package'),
    ('mir', 'changed MIR'),
    ('mir_scrubbed', 'changed MIR, `line!()` ignored'),
    ('mir_fs', 'changed MIR, `line!()` ignored, data files rerun file readers only'),
    ('upper', 'changed lines, item edits ignored (unsound)'),
]
full_cpu = sum(r['cpu'] for r in head)
tot, n = Counter(), Counter()
per = []
for r in rows:
    p = {'commit': r['commit'][:10], 'build': r['build_cpu']}
    for k, _ in METHODS:
        tests = r.get(k, r['baseline_tests'])
        p[k] = (len(tests), sum(cpu(tuple(t)) for t in tests))
        n[k] += p[k][0]
        tot[k] += p[k][1]
    tot['build'] += r['build_cpu']
    p['fallback'] = r['fallback']
    p['outside'] = r['affected_outside_baseline']
    p['unlisted'] = r['unlisted']
    sel, ms = {tuple(t) for t in r['selected']}, {tuple(t) for t in r.get('mir_scrubbed', [])}
    p['clean'] = not r['fallback']
    p['mir_more'] = len(ms - sel)
    p['mir_less'] = len(sel - ms)
    per.append(p)

base = tot['baseline_tests']
print(f'imputed CPU for {imputed["tests"]} of {n["baseline_tests"] * len(METHODS)} test runs, absent at HEAD')
print(f'full run at HEAD: {len(head)} tests, {full_cpu:.0f} CPU s')
print()
print('| selection | tests run | test CPU s | of today | median commit |')
print('| --- | ---: | ---: | ---: | ---: |')
med = lambda k: statistics.median(p[k][1] / p['baseline_tests'][1] for p in per if p['baseline_tests'][1])
for k, label in METHODS:
    print(f'| {label} | {n[k]} | {tot[k]:.0f} | {100 * tot[k] / base:.0f}% | {100 * med(k):.0f}% |')
print()
all_n = kinds['ALL']
scale = kinds['some'] / len(rows)
print(f'replayed commits: build CPU {tot["build"]:.0f} s (rust2-callgraph, -j8), test CPU {base:.0f} s, '
      f'so tests are {100 * base / (base + tot["build"]):.0f}% of build + test')
for k, label in METHODS[1:]:
    saved = base - tot[k]
    window_tests = base * scale + all_n * full_cpu
    print(f'  {label}: saves {100 * saved / base:.0f}% of their test CPU, {100 * saved / (base + tot["build"]):.0f}% of build + test; '
          f'{100 * saved * scale / window_tests:.0f}% of the window\'s test CPU with its {all_n} run-everything commits at {full_cpu:.0f} s')
print()
clean = [p for p in per if p['clean']]
print(f'commits without a line fallback: {len(clean)}; MIR selects tests lines do not on {sum(1 for p in clean if p["mir_more"])}, '
      f'fewer on {sum(1 for p in clean if p["mir_less"])}')
print('commits with a line fallback:', sum(1 for p in per if p['fallback']), 'of', len(per))
print('commits where MIR falls back (manifest, build script, data file):',
      sum(1 for p in per if any(x in ('manifest', 'build script', 'data file') for v in p['fallback'].values() for x in v)))
print('commits selecting no test:', {k: sum(1 for p in per if p[k][0] == 0) for k, _ in METHODS})
print('affected tests outside the baseline (must be 0):', sum(p['outside'] for p in per),
      '; listed tests missing from the graph:', sum(p['unlisted'] for p in per))
reasons = Counter()
for r in rows:
    for v in r['fallback'].values():
        for x in v:
            reasons[x] += 1
print('fallback reasons (commit-package pairs):', reasons.most_common())
flags = defaultdict(set)
base_all = set()
for r in rows:
    base_all |= {tuple(t) for t in r['baseline_tests']}
    for k, v in r['flags'].items():
        flags[k] |= {tuple(t) for t in v}
print('distinct baseline tests:', len(base_all), 'reaching:', {k: f'{100 * len(v) / len(base_all):.0f}%' for k, v in flags.items()})
print()
cols = [('baseline_tests', 'today'), ('selected', 'lines'), ('mir_scrubbed', 'MIR'), ('mir_fs', 'MIR, files'),
        ('upper', 'lines, unsound')]
print('| commit | ' + ' | '.join(c for _, c in cols) + ' | build CPU s |')
print('| --- |' + ' ---: |' * (len(cols) + 1))
for p in per:
    print(f"| {p['commit']} | " + ' | '.join(f'{p[k][0]} / {p[k][1]:.0f} s' for k, _ in cols) + f" | {p['build']:.0f} |")
