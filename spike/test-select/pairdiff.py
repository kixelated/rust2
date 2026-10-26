"""Build two commits from scratch in a side checkout, then print what MIR hashing sees
change between them: pairdiff.py CHECKOUT TARGET A B PACKAGE...

The same commit twice checks that the hashes are deterministic.
"""
import os, shutil, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import ts

moq, target, commits, packages = os.path.realpath(sys.argv[1]), sys.argv[2], sys.argv[3:5], sys.argv[5:]
args = ['test', '--no-run', '--locked', '--tests', '-j', '8'] + [a for p in packages for a in ('-p', p)]
snaps = []
for i, c in enumerate(commits):
    ts.git(moq, 'checkout', '-q', '--detach', c)
    subprocess.run(['python3', os.path.join(here, 'cleanws.py'), moq, target], check=True)
    out = os.path.join(here, 'pair-graphs')
    shutil.rmtree(out, ignore_errors=True)
    subprocess.run(['nix', 'develop', moq, '--command', os.path.join(here, 'bin/graph'), moq, out, target, *args],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    snap = os.path.join(here, f'pair-{i}')
    shutil.rmtree(snap, ignore_errors=True)
    shutil.move(out, snap)
    snaps.append(snap)
subprocess.run(['python3', os.path.join(here, 'hashdiff.py'), *snaps, r'^(const )?(moq|kio|qmux|hang|integration)'])
