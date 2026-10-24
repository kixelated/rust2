# Renders a run as Markdown. Inputs: $host (host.json) and $results
# (results.jsonl). Run with `jq -n -r`.

def fixed(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
def num: if . >= 100 then round | tostring elif . >= 10 then fixed(1) elif . >= 1 then fixed(2) else fixed(3) end;
def pct: (. * 100 | fixed(1)) as $p | if . > 0 then "+\($p)%" else "\($p)%" end;
def row: "| " + join(" | ") + " |";
# How much more a result costs than another: time and CPU as they are, throughput inverted.
def cost($other): if .unit == "MB/s" then $other.trimmed / .trimmed else .trimmed / $other.trimmed end;
# Baselines first, then the wrappers in a stable order.
def runners: ["rust-net", "rust-ffi", "go", "python", "kotlin", "cpp", "dart", "swift", "browser"] as $known
	| (map(.runner) | unique) as $seen
	| ($known | map(select(. as $r | $seen | index($r)))) + ($seen - $known);
# In the order rust-ffi reports them; a runner may order its cases differently.
def cases: group_by(.name)
	| map({name: .[0].name, unit: .[0].unit, order: ((map(select(.runner == "rust-ffi") | .order) | min) // 1e9)})
	| sort_by(.order);

# One table of `$variant`, each cell drawn by `cell` from {result, by}.
def matrix($variant; cell):
	map(select(.variant == $variant)) as $rows
	| ($rows | map({key: "\(.runner)/\(.name)", value: .}) | from_entries) as $by
	| ($rows | runners) as $columns
	| (["case", "unit"] + $columns | row),
		("| --- | --- |" + ($columns | map(" ---: |") | join(""))),
		(
			$rows | cases[] | . as $case
			| [$case.name, $case.unit]
				+ ($columns | map($by["\(.)/\($case.name)"] as $r | if $r == null then "" else {result: $r, by: $by} | cell end))
			| row
		);

# The change of each of `$variant`'s results against its base's.
def change($variant):
	. as $all
	| ($all | map(select(.variant == $variant)) | first | .base) as $base
	| ($all | map(select(.variant == $base)) | map({key: "\(.runner)/\(.name)", value: .}) | from_entries) as $baseline
	| $all | matrix($variant; .result as $r | $baseline["\($r.runner)/\($r.name)"] as $b | if $b == null then "" else ($r | cost($b)) - 1 | pct end);

$host[0] as $h
| $results as $results
| [
	"# Binding benchmark",
	"",
	"\($h.date) on \($h.cpu) (\($h.cpus) CPUs, \($h.memory)), \($h.os), runner `\($h.runner)`; load average \($h.load_start) before, \($h.load_end) after.",
	"rust2 `\($h.rust2[:12])`\(if $h.base == "" then "" else ", base `\($h.base[:12])`" end); moq `\($h.moq[:12])`\(if ($h.ubrn // "") == "" then "" else ", ubrn `\($h.ubrn[:12])`" end); \($h.rounds) round\(if $h.rounds == 1 then "" else "s" end) of \($h.seconds) s per case.",
	"\([$h.tools[] | select(. != "missing")] | join("; ")).",
	"",
	"Each cell is the trimmed mean over the rounds (without the fastest and slowest), and the trimmed mean of its cost against `rust-ffi` in the same round: above 1x is slower or costlier.",
	"`rust-ffi` calls moq-ffi's API from Rust without crossing the FFI; `rust-net` runs the same rounds on moq-net, below moq-ffi's handles and runtime.",
	"",
	"## fork",
	"",
	($results | matrix("fork"; .result as $r | "\($r.trimmed | num)" + (if $r.ratio == null then "" else " (\($r.ratio | num)x)" end))),
	"",
	"## Noise: control against fork",
	"",
	"`control` reruns fork's builds in the same rounds, so its change is this run's noise floor.",
	"Change in trimmed-mean cost (time per call, CPU per MB, or the inverse of throughput): negative is cheaper.",
	"",
	($results | change("control")),
	(
		if $h.base == "" then empty else
			"",
			"## Change: fork against base",
			"",
			"The change in cost from base to fork. Compare it with the noise floor above.",
			"",
			($results | change("fork"))
		end
	)
]
| .[]
