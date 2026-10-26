"""CPU of rebuilding after a moq-net edit, plain rustc versus rust2-callgraph writing graphs.

Usage: overhead.py MOQ TARGET OUT ROUNDS
Every compile goes through the same nightly driver; with GRAPH_ROOT pointing nowhere it
compiles as plain rustc, so the difference is the graph's own cost.
"""
import os, resource, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
moq, target, out, rounds = os.path.realpath(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4])
cmd = ['nix', 'develop', moq, '--command', os.path.join(here, 'bin/graph'), moq, out, target, 'test', '--no-run',
       '--locked', '-p', 'moq-net', '-p', 'moq-relay', '-p', 'moq-tokio', '--tests', '-j', '8']


def build(root):
    env = dict(os.environ, GRAPH_ROOT=root) if root else dict(os.environ)
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    subprocess.run(cmd, env=env, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    return (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)


build('/nonexistent')
for i in range(rounds):
    for mode, root in (('plain', '/nonexistent'), ('graph', None)):
        os.utime(os.path.join(moq, 'rs/moq-net/src/lib.rs'))
        print(mode, round(build(root), 1), flush=True)
