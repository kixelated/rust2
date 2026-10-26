"""Run every test of a build in its own process, as nextest does, recording its CPU time.

Usage: runtests.py MOQ BUILD_JSON GRAPH_DIR OUT_JSON [WORKERS [PACKAGES]]
"""
import json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ts

moq, build_json, graph_dir, out = sys.argv[1:5]
workers = int(sys.argv[5]) if len(sys.argv) > 5 else 6
only = set(sys.argv[6].split(',')) if len(sys.argv) > 6 else None

# Executables and their package directories, straight from cargo.
exes = {}
for line in open(build_json):
    if not line.startswith('{'):
        continue
    msg = json.loads(line)
    if msg.get('reason') == 'compiler-artifact' and msg.get('executable') and msg['profile']['test']:
        key = os.path.basename(msg['executable'])
        if only is not None and ts.Build._pkg(None, msg['package_id']) not in only:
            continue
        exes[key] = (msg['executable'], os.path.dirname(msg['manifest_path']), msg['package_id'])


def listing(exe, *extra):
    r = subprocess.run([exe, '--list', '--format', 'terse', *extra], capture_output=True, text=True, timeout=120)
    if r.returncode != 0:
        raise SystemExit(f'listing {exe} failed: {r.stderr[:500]}')
    return [l[:-len(': test')] for l in r.stdout.split('\n') if l.endswith(': test')]


jobs = []
for key, (exe, cwd, pid) in sorted(exes.items()):
    ignored = set(listing(exe, '--ignored'))
    for name in listing(exe):
        if name not in ignored:
            jobs.append((key, name, exe, cwd))
print(f'{len(jobs)} tests in {len(exes)} binaries', flush=True)


def run(job):
    key, name, exe, cwd = job
    env = dict(os.environ, NEXTEST='1', NEXTEST_EXECUTION_MODE='process-per-test', CARGO_MANIFEST_DIR=cwd)
    start = time.monotonic()
    p = subprocess.Popen([exe, '--exact', name, '--nocapture'], cwd=cwd, env=env,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    deadline = start + 180
    status = None
    while status is None:
        pid, st, ru = os.wait4(p.pid, os.WNOHANG)
        if pid:
            status = os.waitstatus_to_exitcode(st)
            break
        if time.monotonic() > deadline:
            os.killpg(p.pid, 9)
        time.sleep(0.005)
    p.returncode = status
    return {'unit': key, 'name': name, 'cpu': ru.ru_utime + ru.ru_stime, 'wall': time.monotonic() - start,
            'status': status}


results = []
with ThreadPoolExecutor(workers) as pool:
    for i, r in enumerate(pool.map(run, jobs)):
        results.append(r)
        if i % 500 == 0:
            print(i, flush=True)
json.dump(results, open(out, 'w'))
failed = [r for r in results if r['status'] != 0]
print(f'cpu {sum(r["cpu"] for r in results):.1f}s, {len(failed)} failed')
