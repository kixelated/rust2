"""Replay moq commits: build each with rust2-callgraph, keep its graphs, and select its tests.

Usage: replay.py MOQ SURVEY_JSON COUNT OUTDIR
Builds the COUNT newest surveyed commits whose baseline tests some packages, oldest
first, in one target directory, and writes OUTDIR/<commit>.json for each. A commit's
parent is built too when the previous build is not the same Rust tree.

Each build keeps its graphs in OUTDIR/snap/<commit>.tar.zst and its tests in
OUTDIR/<commit>.run.json, so reselect.py can redo the selection without rebuilding.
"""
import json, os, resource, shutil, subprocess, sys, time

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

moq, survey, count, outdir = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
snapdir = os.path.join(outdir, 'snap')
os.makedirs(snapdir, exist_ok=True)
rows = [r for r in json.load(open(survey)) if r['baseline'] == 'some'][:count]
rows.reverse()
RUST_INPUTS = ['rs', 'demo', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']
graphs = os.path.join(here, 'graphs')


def listing(exe, *extra):
    r = subprocess.run([exe, '--list', '--format', 'terse', *extra], capture_output=True, text=True, timeout=120)
    if r.returncode != 0:
        raise RuntimeError(f'listing {exe}: {r.stderr[:300]}')
    return [l[:-len(': test')] for l in r.stdout.split('\n') if l.endswith(': test')]


def build_at(c):
    """Check out and build commit c: (Build or None, CPU seconds, wall seconds, tests nextest runs)."""
    ts.git(moq, 'checkout', '-q', '--detach', c)
    build_json = os.path.join(outdir, c + '.build.json')
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.monotonic()
    status = subprocess.run(['quest', 'gate', os.path.join(here, 'bin/moq-build'), moq, graphs,
                             os.path.join(here, 'moq-target'), build_json],
                            stdout=subprocess.DEVNULL, stderr=open(os.path.join(outdir, c + '.log'), 'w')).returncode
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    wall = time.monotonic() - start
    if status != 0:
        return None, cpu, wall, None
    build = ts.Build(moq, build_json, graphs)
    run = set()
    for key, unit in build.units.items():
        if unit['exe'] and unit['test']:
            ignored = set(listing(unit['exe'], '--ignored'))
            run |= {(key, name) for name in listing(unit['exe']) if name not in ignored}
    json.dump(sorted(run), open(os.path.join(outdir, c + '.run.json'), 'w'))
    files = sorted({os.path.basename(u['graph']) for u in build.units.values()})
    subprocess.run(['tar', '--zstd', '-cf', os.path.join(snapdir, c + '.tar.zst'), '-C', graphs, *files], check=True)
    return build, cpu, wall, run


prev = None  # (commit, Build, run)
for row in rows:
    c, parent = row['commit'], row['parent']
    out = os.path.join(outdir, c + '.json')
    if os.path.exists(out):
        prev = None
        continue
    same = prev is not None and not ts.git(moq, 'diff', '--name-only', prev[0], parent, '--', *RUST_INPUTS).strip()
    parent_cpu = 0
    if not same:
        b, parent_cpu, _, run = build_at(parent)
        prev = (parent, b, run) if b else None
    build, cpu, wall, run = build_at(c)
    result = {'commit': c, 'parent': parent, 'base': prev and prev[0], 'paths': row['paths'], 'build_cpu': cpu,
              'build_wall': wall, 'parent_build_cpu': parent_cpu}
    if build is None:
        result['error'] = 'build failed'
        json.dump(result, open(out, 'w'))
        print(c[:10], 'build failed', flush=True)
        prev = None
        continue
    result.update(ts.select_row(moq, row, prev and prev[1], prev and prev[2], build, run))
    json.dump(result, open(out, 'w'))
    print(c[:10], f'build {cpu:.0f}s (parent {parent_cpu:.0f}s)', f"baseline {len(result['baseline_tests'])}",
          f"lines {len(result['selected'])}", f"upper {len(result['upper'])}", f"mir {len(result.get('mir', []))}",
          f"scrubbed {len(result.get('mir_scrubbed', []))}", 'fallback', sorted(result['fallback']), flush=True)
    prev = (c, build, run)
