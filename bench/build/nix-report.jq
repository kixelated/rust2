# Renders a `just bench-nix` run as Markdown. Inputs: $host (host.json),
# $results (results.jsonl), and $generate (crate2nix's generate timings). Run
# with `jq -n -r`.

def fixed(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
def secs: if . == null then "-" elif . < 10 then fixed(2) else fixed(1) end;
def pct: (. * 100 | fixed(1)) as $p | if . > 0 then "+\($p)%" else "\($p)%" end;
def count: if . == null then "-" else tostring end;
def row: "| " + join(" | ") + " |";

$host[0] as $h
| ["clean", "second-package", "edit-hub", "lock-bump"] as $scenarios
| ($results | map({key: "\(.scenario)/\(.variant)", value: .}) | from_entries) as $by
| ($results | map(.variant) | unique) as $variants
| [
	"# Nix build benchmark",
	"",
	"\($h.date) on \($h.cpu) (\($h.cpus) CPUs, \($h.memory)), \($h.os), runner `\($h.runner)`.",
	"rust2 `\($h.rust2[:12])`; moq `\($h.moq[:12])`; \($h.nix); \($h.mbx); crate2nix \($h.crate2nix).",
	"",
	"One build per row, in CPU seconds: user plus system time of every process it ran.",
	"Each variant runs the scenarios in this order on one store, so every row after `clean` starts warm.",
	"`vs crane` compares CPU time. Evaluation is timed apart, and the total adds it.",
	"`derivations` counts the Nix derivations that compiled Rust; `compiled` counts rustc runs, as cargo's `Compiling` and `Checking` lines for crane, a derivation per crate for crate2nix, and mbx's misses for mbx; `restored` counts mbx's hits.",
	"",
	(["scenario", "variant", "CPU", "vs crane", "wall", "eval CPU", "derivations", "compiled", "restored"] | row),
	"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
	(
		$scenarios[] as $s
		| $variants[] as $v
		| $by["\($s)/\($v)"] // empty
		| $by["\($s)/crane"] as $base
		| [
			$s,
			$v,
			(.build.cpu | secs),
			(if $v == "crane" then "base" elif $base == null then "-" else .build.cpu / $base.build.cpu - 1 | pct end),
			(.build.wall | secs),
			(.eval.cpu | secs),
			(.derivations | count),
			(.compiled | count),
			(.restored | count)
		]
		| row
	),
	"",
	(["variant", "total CPU", "vs crane"] | row),
	"| --- | ---: | ---: |",
	(
		($results | map(select(.variant == "crane") | .build.cpu + .eval.cpu) | add) as $base
		| $variants[] as $v
		| ($results | map(select(.variant == $v) | .build.cpu + (.eval.cpu // 0)) | add) as $total
		| [$v, ($total | secs), (if $v == "crane" then "base" elif $base == null then "-" else $total / $base - 1 | pct end)]
		| row
	),
	(
		select($generate | length > 0)
		| "",
		"`crate2nix generate`, which a project reruns and commits when its lockfile changes, took "
			+ ($generate | map("\(.cpu | secs) CPU seconds at the \(.lock) lockfile") | join(" and ")) + "."
	)
]
| .[]
