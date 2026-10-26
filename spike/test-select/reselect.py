"""Redo a replay's selections from its kept graphs, without rebuilding.

Usage: reselect.py BUILT_MOQ CHECKOUT REPLAY_DIR OUTDIR
BUILT_MOQ is the checkout the replay built in, which its cargo messages name;
CHECKOUT is a clone this script may check out commits in.
"""
import glob, json, os, shutil, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

built, checkout, rdir, outdir = sys.argv[1:5]
os.makedirs(outdir, exist_ok=True)
work = os.path.join(outdir, 'work')


def load(c):
    d = os.path.join(work, c)
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    subprocess.run(['tar', '--zstd', '-xf', os.path.join(rdir, 'snap', c + '.tar.zst'), '-C', d], check=True)
    build = ts.Build(built, os.path.join(rdir, c + '.build.json'), d)
    run = {tuple(t) for t in json.load(open(os.path.join(rdir, c + '.run.json')))}
    shutil.rmtree(d)
    return build, run


cache = {}
for f in sorted(glob.glob(os.path.join(rdir, '*.json'))):
    if f.endswith(('.build.json', '.run.json')):
        continue
    r = json.load(open(f))
    if 'error' in r:
        continue
    c = r['commit']
    for k in list(cache):
        if k not in (c, r['base']):
            del cache[k]
    for k in (r['base'], c):
        if k and k not in cache:
            cache[k] = load(k)
    base = cache.get(r['base'])
    ts.git(checkout, 'checkout', '-q', '--detach', c)
    keep = {k: r[k] for k in ('commit', 'parent', 'base', 'paths', 'build_cpu', 'build_wall', 'parent_build_cpu')}
    keep.update(ts.select_row(checkout, r, base and base[0], base and base[1], *cache[c]))
    json.dump(keep, open(os.path.join(outdir, c + '.json'), 'w'))
    print(c[:10], f"baseline {len(keep['baseline_tests'])} lines {len(keep['selected'])} upper {len(keep['upper'])}",
          f"mir {len(keep.get('mir', []))} scrubbed {len(keep.get('mir_scrubbed', []))}", flush=True)
