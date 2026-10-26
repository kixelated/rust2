"""Delete the workspace members' units from a target dir, so the next build reruns rust2-callgraph on them.

Usage: cleanws.py MOQ TARGET_DIR
"""
import json, os, shutil, subprocess, sys

moq, target = sys.argv[1:3]
meta = json.loads(subprocess.run(['cargo', 'metadata', '--format-version', '1', '--no-deps'], cwd=moq,
                                 check=True, capture_output=True, text=True).stdout)
for p in meta['packages']:
    d = os.path.join(target, 'debug', 'build', p['name'])
    if os.path.isdir(d):
        shutil.rmtree(d)
