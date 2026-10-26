"""Test selection from MIR call graphs, replayed on moq's history.

Library used by the replay and report scripts. Python only reads what rust2-callgraph
wrote and what git and cargo say; it never compiles.
"""
import json, os, re, subprocess
from collections import defaultdict

# ---------------------------------------------------------------- moq's baseline

# dispatch.sh: the rs module's scope, and the root orchestration that runs everything.
RS_SCOPE = re.compile(r'^(sh/local\.(sh|ts)$|rs/|demo/carla/bridge/|sh/rs/|Cargo\.(toml|lock)$|rust-toolchain\.toml$|\.config/nextest\.toml$)')
ROOT_ALL = re.compile(r'^(justfile|sh/(dispatch|changed)\.sh|sh/local\.(sh|ts))$')
# select.sh: inputs every crate shares.
SELECT_ALL = re.compile(r'^(sh/local\.(sh|ts)|Cargo\.(toml|lock)|rust-toolchain\.toml|rs/justfile|sh/rs/(select|platform|nextest|(net|tokio|media)-features)\.sh|\.config/nextest\.toml)$')


def git(moq, *args):
    return subprocess.run(['git', '-C', moq, *args], check=True, capture_output=True, text=True).stdout


def metadata(moq):
    """Workspace packages as select.sh sees them: name, dir parts, dependency names."""
    meta = json.loads(subprocess.run(['cargo', 'metadata', '--format-version', '1', '--no-deps'], cwd=moq,
                                     check=True, capture_output=True, text=True).stdout)
    pkgs = []
    for p in meta['packages']:
        parts = p['manifest_path'].replace('\\', '/').split('/')
        if parts[-3] == 'rs' or parts[-4] == 'demo':
            pkgs.append({'name': p['name'], 'parts': parts, 'deps': [d['name'] for d in p['dependencies']]})
    return pkgs


def dependents(pkgs, seeds):
    """seeds plus every workspace package depending on one, as select.sh walks them."""
    want = set(seeds)
    grew = True
    while grew:
        grew = False
        for p in pkgs:
            if p['name'] not in want and any(d in want for d in p['deps']):
                want.add(p['name'])
                grew = True
    names = {p['name'] for p in pkgs}
    return want & names


def baseline(paths, pkgs):
    """What `just ci` tests for these changed paths: None, 'ALL', or a set of packages."""
    if not any(RS_SCOPE.search(p) for p in paths):
        return None
    if any(ROOT_ALL.search(p) or SELECT_ALL.search(p) for p in paths):
        return 'ALL'
    if not any(re.match(r'^(rs|demo)/', p) for p in paths):
        return None
    if any(p['parts'][-3] == 'rs' and p['parts'][-2] != p['name'] for p in pkgs):
        return 'ALL'
    benches = {m.group(1) for p in paths if (m := re.match(r'^rs/([^/]*)/benches/', p))}
    seeds = {m.group(1) for p in paths if not re.match(r'^rs/[^/]*/benches/', p) and (m := re.match(r'^rs/([^/]*)/', p))}
    for p in pkgs:
        if p['parts'][-4] == 'demo' and any(x.startswith(f"demo/{p['parts'][-3]}/{p['parts'][-2]}/") for x in paths):
            seeds.add(p['name'])
    selected = dependents(pkgs, seeds) | ({p['name'] for p in pkgs} & benches)
    if not selected:
        return None
    # The device features pull these two in beside their dependents.
    if selected & {'moq-video', 'moq-audio', 'moq-transcode'} or 'moq-video' in selected or 'moq-audio' in selected:
        selected |= {'moq-video', 'moq-audio'} & {p['name'] for p in pkgs}
    return selected


def owner(path, pkgs):
    """The workspace package a changed path seeds, as select.sh reads it."""
    m = re.match(r'^rs/([^/]*)/', path)
    if m:
        return m.group(1)
    for p in pkgs:
        if p['parts'][-4] == 'demo' and path.startswith(f"demo/{p['parts'][-3]}/{p['parts'][-2]}/"):
            return p['name']
    return None


# ---------------------------------------------------------------- the build


class Build:
    """One build's units, tests, and joined call graph."""

    def __init__(self, moq, build_json, graph_dir):
        self.moq = os.path.realpath(moq)
        self.units = {}  # key -> {'pkg', 'kind', 'test', 'exe', 'graph'}
        for line in open(build_json):
            if not line.startswith('{'):
                continue
            msg = json.loads(line)
            if msg.get('reason') != 'compiler-artifact':
                continue
            pid = msg['package_id']
            if not pid.startswith('path+file://' + self.moq):
                continue
            kind = msg['target']['kind']
            if 'custom-build' in kind or 'proc-macro' in kind:
                continue
            files = ([msg['executable']] if msg.get('executable') else []) + msg['filenames']
            key = None
            for f in files:
                base = os.path.basename(f)
                base = re.sub(r'\.(rlib|rmeta|so|a|dylib)$', '', base)
                if base.startswith('lib') and not msg.get('executable'):
                    base = base[3:]
                if os.path.exists(os.path.join(graph_dir, base + '.json')):
                    key = base
                    break
            if key is None and 'bin' in kind and not msg['profile']['test']:
                key = self._bin_graph(graph_dir, msg['target']['name'].replace('-', '_'))
            if key is None:
                raise SystemExit(f'no graph for {msg["target"]["name"]} of {pid}: {files}')
            name = re.search(r'([^/#]*)#', pid)
            pkg = msg['target']['name'] if False else self._pkg(pid)
            self.units[key] = {'pkg': pkg, 'kind': kind, 'name': msg['target']['name'],
                               'test': msg['profile']['test'], 'exe': msg.get('executable'),
                               'graph': os.path.join(graph_dir, key + '.json')}
        self._load()

    def _bin_graph(self, graph_dir, crate):
        # Cargo names an uplifted binary without its hash: take the newest graph of a
        # non-test crate by that name with a root `main`.
        best = None
        for f in os.listdir(graph_dir):
            if not re.fullmatch(re.escape(crate) + r'(-[0-9a-f]+)?\.json', f):
                continue
            path = os.path.join(graph_dir, f)
            d = json.load(open(path))
            if d['test'] or not any(re.fullmatch(re.escape(crate) + r'\[[0-9a-f]+\]::main', x[0]) for x in d['defs']):
                continue
            if best is None or os.path.getmtime(path) > best[0]:
                best = (os.path.getmtime(path), f[:-5])
        return best and best[1]

    def _pkg(self, pid):
        # path+file:///.../rs/moq-net#0.3.10 or ...#name@version
        m = re.match(r'path\+file://[^#]*/([^/#]+)#(.*)$', pid)
        rest = m.group(2)
        return rest.split('@')[0] if '@' in rest else m.group(1)

    def _load(self):
        self.node_def = {}
        self.node_hash = {}  # symbol -> (raw, scrubbed) MIR hash, where a unit codegens it
        self.edges = defaultdict(set)
        self.defs = defaultdict(list)  # rel file -> [(lo, hi, kind, def)]
        self.items = defaultdict(list)  # rel file -> [(lo, hi, kind, def)]
        self.files = defaultdict(set)  # rel file -> unit keys
        self.tests = []  # (unit key, name, node)
        self.unit_env = {}
        self.unit_nodes = defaultdict(set)
        hashes = defaultdict(set)
        defined = defaultdict(set)  # unit-local symbol -> its per-unit nodes
        for key, unit in self.units.items():
            d = json.load(open(unit['graph']))
            syms = []
            for n in d['nodes']:
                sym = n[0]
                # `#[no_mangle]` exports and `#[global_allocator]` shims keep one name in
                # every unit that defines them, each with its own body.
                if len(n) > 2 and (not sym.startswith(('_R', 'const ')) or '7___rustc' in sym):
                    sym = f'{sym}@{key}'
                    defined[n[0]].add(sym)
                syms.append(sym)
                self.node_def[sym] = n[1]
                self.unit_nodes[key].add(sym)
                if len(n) > 2:
                    hashes[sym].add((n[2], n[3]))
            for a, bs in d['edges']:
                self.edges[syms[a]].update(syms[b] for b in bs)
            for f in d['files']:
                rel = self.rel(f)
                if rel:
                    self.files[rel].add(key)
            for table, rows in ((self.defs, d['defs']), (self.items, d['items'])):
                for dfn, kind, span in rows:
                    if span:
                        rel = self.rel(span[0])
                        if rel:
                            table[rel].append((span[1], span[2], kind, dfn))
            for name, node in d['tests']:
                self.tests.append((key, name, syms[node]))
            self.unit_env[key] = d['env']
        # A unit that only calls such a symbol may link any unit's definition of it.
        for sym, copies in defined.items():
            self.edges[sym] |= copies
        # An `#[inline]` fn's hash differs only where a crate compiles it differently.
        for sym, hs in hashes.items():
            hs = sorted(hs)
            self.node_hash[sym] = hs[0] if len(hs) == 1 else ('|'.join(h[0] for h in hs), '|'.join(h[1] for h in hs))
        for table in (self.defs, self.items):
            for rel in table:
                table[rel] = sorted(set(table[rel]))
        self._hidden_edges()
        self.reverse = defaultdict(set)
        for a, bs in self.edges.items():
            for b in bs:
                self.reverse[b].add(a)
        self.def_nodes = defaultdict(set)
        for n, dfn in self.node_def.items():
            self.def_nodes[dfn].add(n)

    def _hidden_edges(self):
        """Calls no MIR names: the global allocator, and a test that runs a binary."""
        alloc = defaultdict(set)
        mains = defaultdict(set)
        for key, nodes in self.unit_nodes.items():
            unit = self.units[key]
            for n in nodes:
                d = strip(self.node_def[n])
                # #[global_allocator] runs on every allocation in the binary, including
                # those in upstream code the graph does not follow.
                if n.endswith('@' + key) and re.search(r'::__rust_(alloc|dealloc|realloc|alloc_zeroed)$', d):
                    alloc[key].add(n)
                if 'bin' in unit['kind'] and not unit['test'] and d.endswith('::main') and d.count('::') == 1:
                    mains[unit['name']].add(n)
        for key, name, node in self.tests:
            self.edges[node] |= alloc[key]
            # CARGO_BIN_EXE_<bin> runs that binary's main.
            for var in self.unit_env[key]:
                if var.startswith('CARGO_BIN_EXE_'):
                    self.edges[node] |= mains[var[len('CARGO_BIN_EXE_'):]]

    def rel(self, path):
        # Cargo hands rustc a workspace member's paths relative to the workspace root.
        if not os.path.isabs(path):
            return path
        root = self.moq + '/'
        if path.startswith(root):
            return path[len(root):]
        return None

    def reach_back(self, nodes):
        """Every node that can reach one of `nodes`."""
        seen = set(nodes)
        stack = list(nodes)
        while stack:
            n = stack.pop()
            for m in self.reverse.get(n, ()):
                if m not in seen:
                    seen.add(m)
                    stack.append(m)
        return seen


def strip(def_path):
    return re.sub(r'\[[0-9a-f]+\]', '', def_path)


# ---------------------------------------------------------------- mapping a diff

FN_KINDS = {'Fn', 'AssocFn', 'Closure', 'SyntheticCoroutineBody'}
CONTAINERS = {'Impl { of_trait: false }', 'Impl { of_trait: true }', 'Trait', 'Mod', 'ForeignMod'}
ITEM_KW = re.compile(r'^\s*(pub(\([^)]*\))?\s+)?((async|const|unsafe|default|extern\s+"[^"]*")\s+)*(fn|struct|enum|union|impl|trait|use|mod|const|static|type|macro_rules!)\b')


LINT_ATTR = re.compile(r'^#!?\[(allow|expect|warn|deny|doc)\b')


def trivial(text):
    """A line no test observes: blank, a comment, a doc comment nextest never runs, or a lint level."""
    t = text.strip()
    return t == '' or t.startswith('//') or t.startswith('*') or t.startswith('/*') or t == '*/' or bool(LINT_ATTR.match(t))


def parse_diff(diff):
    """`git diff -U0` of one file: hunks of (new_start, new_count, removed, added)."""
    hunks = []
    for line in diff.split('\n'):
        m = re.match(r'^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', line)
        if m:
            hunks.append((int(m.group(3)), int(m.group(4) or 1), [], []))
        elif hunks and line.startswith('-') and not line.startswith('---'):
            hunks[-1][2].append(line[1:])
        elif hunks and line.startswith('+') and not line.startswith('+++'):
            hunks[-1][3].append(line[1:])
    return hunks


def containing(rows, lo, hi):
    return [r for r in rows if r[0] <= lo and r[1] >= hi]


class Change:
    """What one commit changed, mapped onto a build of it."""

    def __init__(self):
        self.defs = set()  # def paths of changed fns and statics
        self.fallback = defaultdict(set)  # package -> reasons
        self.reasons = defaultdict(int)  # reason -> count, for the report
        self.notcompiled = set()

    def fall(self, pkg, reason):
        self.fallback[pkg].add(reason)
        self.reasons[reason] += 1


def map_change(moq, parent, commit, paths, build, pkgs):
    change = Change()
    for path in paths:
        pkg = owner(path, pkgs)
        if pkg is None:
            continue
        if not path.endswith('.rs'):
            if path in build.files:
                change.fall(pkg, 'included file')
            elif path.endswith('Cargo.toml'):
                change.fall(pkg, 'manifest')
            elif re.search(r'(^|/)(README|CHANGELOG|MIGRATE|AGENTS|CLAUDE)[^/]*\.md$', path) or path.endswith('.md'):
                change.reasons['markdown'] += 1
            else:
                change.fall(pkg, 'data file')
            continue
        if path not in build.files:
            # Deleted, or compiled into nothing CI's test build links: a host-only
            # crate (build script, proc macro) or a cfg this platform never sets.
            new = git(moq, 'ls-tree', '--name-only', commit, '--', path).strip()
            if not new:
                change.fall(pkg, 'deleted file')
            elif path.endswith('build.rs'):
                change.fall(pkg, 'build script')
            elif '/macros/' in path:
                change.fall(pkg, 'proc macro')
            else:
                change.notcompiled.add(path)
                change.reasons['not compiled'] += 1
            continue
        diff = git(moq, 'diff', '-U0', parent, commit, '--', path)
        text = git(moq, 'show', f'{commit}:{path}').split('\n')
        defs = build.defs.get(path, [])
        items = build.items.get(path, [])
        for new_start, new_count, removed, added in parse_diff(diff):
            lines = list(range(new_start, new_start + new_count)) if new_count else []
            for n in lines:
                classify_line(change, pkg, path, n, text, defs, items)
            if any(not trivial(t) for t in removed):
                if new_count:
                    lo, hi = new_start, new_start + new_count - 1
                else:
                    lo, hi = new_start, new_start + 1
                inside = [d for d in containing(defs, lo, hi) if d[2] in FN_KINDS]
                item_removed = any(ITEM_KW.match(t) for t in removed)
                if inside and item_removed:
                    # A signature edited in place keeps its fn's first line in the hunk.
                    starts = any(d[0] in lines for d in inside)
                    strict = [d for d in inside if d[0] < lo and d[1] > hi]
                    if not starts and not strict:
                        inside = []
                if inside:
                    change.defs.update(d[3] for d in inside)
                else:
                    kw = next((m.group(5) for t in removed if (m := ITEM_KW.match(t))), None)
                    change.fall(pkg, 'removed: ' + (kw or 'other'))
    return change


def classify_line(change, pkg, path, n, text, defs, items):
    t = text[n - 1] if n - 1 < len(text) else ''
    fns = [d for d in containing(defs, n, n) if d[2] in FN_KINDS]
    holders = containing(items, n, n)
    leaves = [i for i in holders if i[2] not in CONTAINERS and i[2] not in FN_KINDS]
    if fns:
        inner = min(fns, key=lambda d: d[1] - d[0])
        # An item the line also belongs to that is no fn's own local, such as a
        # struct a macro generates at the same call site, still changes types.
        if not any(not (inner[0] <= i[0] and i[1] <= inner[1] and (i[0], i[1]) != (inner[0], inner[1])) for i in leaves):
            change.defs.update(d[3] for d in fns)
            return
    if trivial(t):
        return
    statics = [i for i in leaves if i[2].startswith('Static')]
    if statics and len(leaves) == len(statics):
        # A static's initializer: its node is in the graph.
        change.defs.update(i[3] for i in statics)
        return
    if t.strip().startswith('#[') or t.strip().startswith('#!['):
        if 'cfg' in t:
            change.fall(pkg, 'cfg attribute')
            return
        # An attribute of the fn the following lines start, such as #[test] or #[ignore].
        for k in range(n + 1, min(n + 12, len(text) + 1)):
            starts = [d for d in defs if d[0] == k and d[2] in FN_KINDS]
            if starts:
                change.defs.update(d[3] for d in starts)
                return
            if not (text[k - 1].strip().startswith(('#', '//')) or text[k - 1].strip() == ''):
                break
    kind = min(leaves, key=lambda i: i[1] - i[0])[2] if leaves else (min(holders, key=lambda i: i[1] - i[0])[2] if holders else 'other')
    kind = re.sub(r' \{.*', '', kind)
    change.fall(pkg, 'item: ' + kind)


# ---------------------------------------------------------------- selection


def affected_tests(build, change):
    """Tests whose call graph reaches a changed fn or static."""
    targets = set()
    for dfn in change.defs:
        targets |= build.def_nodes.get(dfn, set())
    back = build.reach_back(targets)
    return {(key, name) for key, name, node in build.tests if node in back}


FLAGS = {
    # Def paths name inherent methods by impl, as `std::time::{impl#0}::now`.
    'fs': re.compile(r'^(std|tokio)::fs::|^std::path::\{impl#\d+\}::(exists|try_exists|is_file|is_dir|is_symlink|read_dir|read_link|canonicalize|metadata|symlink_metadata)$'),
    'env': re.compile(r'^std::env::(var|var_os|vars|vars_os|args|args_os|current_dir|temp_dir|home_dir)'),
    'clock': re.compile(r'^std::time::\{impl#\d+\}::now$|^tokio::time::|^std::thread::sleep'),
    'process': re.compile(r'^(std|tokio)::process::\{impl#\d+\}::(spawn|output|status)$'),
    'net': re.compile(r'^(std|tokio)::net::'),
}


def flagged_tests(build):
    """For each flag, the tests whose call graph reaches such an API directly."""
    out = {}
    for flag, pat in FLAGS.items():
        nodes = {n for n, dfn in build.node_def.items() if pat.search(strip(dfn))}
        back = build.reach_back(nodes)
        out[flag] = {(key, name) for key, name, node in build.tests if node in back}
    return out


# Reasons a sound selector falls back to a package's tests that a selector aware of
# types, items, and cfgs could narrow: the upper bound ignores them.
SYNTACTIC = ('item: ', 'removed: ', 'cfg attribute')


def mir_changed(base, head, scrubbed):
    """Nodes of `head` that are new, compile from different MIR, or call different nodes."""
    i = 1 if scrubbed else 0
    changed = set()
    for sym in head.node_def:
        h = head.node_hash.get(sym)
        if sym not in base.node_def:
            changed.add(sym)
        elif h is not None and (base.node_hash.get(sym) is None or base.node_hash[sym][i] != h[i]):
            changed.add(sym)
        elif head.edges.get(sym, set()) != base.edges.get(sym, set()):
            changed.add(sym)
    return changed


def reached_tests(build, nodes):
    back = build.reach_back(nodes)
    return {(key, name) for key, name, node in build.tests if node in back}


def select_row(moq, row, base, base_run, build, run):
    """One replayed commit's test selections, given its parent's build (or None) and its own.

    `moq` must have `row['commit']` checked out.
    """
    c, parent = row['commit'], row['parent']
    pkgs = metadata(moq)
    base_pkgs = baseline(row['paths'], pkgs)
    if base_pkgs == 'ALL':
        base_pkgs = {u['pkg'] for u in build.units.values()}
    base_pkgs = base_pkgs or set()
    graph_tests = {(k, n) for k, n, _ in build.tests}
    change = map_change(moq, parent, c, row['paths'], build, pkgs)
    affected = affected_tests(build, change)
    fallback = dependents(pkgs, set(change.fallback)) if change.fallback else set()
    hard = {p for p, why in change.fallback.items() if any(not r.startswith(SYNTACTIC) and r != 'data file' for r in why)}
    data = {p for p, why in change.fallback.items() if 'data file' in why}
    hard_fallback = dependents(pkgs, hard) if hard else set()
    data_fallback = dependents(pkgs, data) if data else set()
    # What MIR cannot see: a manifest, what a build script links, and files read at run time.
    manifest = {p for p, why in change.fallback.items() if 'manifest' in why or 'build script' in why}
    mir_fallback = dependents(pkgs, manifest | data) if manifest | data else set()
    pkg_of = {k: u['pkg'] for k, u in build.units.items()}
    baseline_tests = {t for t in run if pkg_of[t[0]] in base_pkgs}
    selected = {t for t in baseline_tests if t in affected or pkg_of[t[0]] in fallback}
    flags = flagged_tests(build)
    upper = {t for t in baseline_tests if t in affected or pkg_of[t[0]] in hard_fallback
             or (pkg_of[t[0]] in data_fallback and t in flags['fs'])}
    mir = {}
    if base is not None:
        new_tests = run - base_run
        for variant, scrubbed in (('mir', False), ('mir_scrubbed', True)):
            changed = mir_changed(base, build, scrubbed)
            reached = reached_tests(build, changed)
            mir[variant] = sorted(t for t in baseline_tests if t in reached or t in new_tests or pkg_of[t[0]] in mir_fallback)
            mir[variant + '_changed'] = len(changed)
        # Only a test that reaches a file API can read a changed data file.
        manifest_fallback = dependents(pkgs, manifest) if manifest else set()
        mir['mir_fs'] = sorted(t for t in baseline_tests if t in reached or t in new_tests or pkg_of[t[0]] in manifest_fallback
                               or (pkg_of[t[0]] in data_fallback and t in flags['fs']))
    return {
        'baseline_packages': sorted(base_pkgs),
        'changed_defs': sorted(change.defs),
        'fallback': {k: sorted(v) for k, v in change.fallback.items()},
        'reasons': dict(change.reasons),
        'not_compiled': sorted(change.notcompiled),
        'unlisted': len(run - graph_tests),
        'baseline_tests': sorted(baseline_tests),
        'selected': sorted(selected),
        'upper': sorted(upper),
        **mir,
        'affected_outside_baseline': len({t for t in affected if t in run} - baseline_tests),
        'flags': {f: sorted(t for t in v if t in baseline_tests) for f, v in flags.items()},
        'units': {k: u['pkg'] for k, u in build.units.items()},
    }
