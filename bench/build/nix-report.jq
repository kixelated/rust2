# Renders a `just bench-nix` run as Markdown. Inputs: $host (host.json),
# $results (results.jsonl), and $generate (crate2nix's generate timings). Run
# with `jq -n -r`.

def fixed(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
def secs: if . == null then "-" elif . < 10 then fixed(2) else fixed(1) end;
def giga: if . == null then "n/a" else (. / 1e9 | round | tostring) + "G" end;
def pct: (. * 100 | fixed(1)) as $p | if . > 0 then "+\($p)%" else "\($p)%" end;
def vs($base): if $base == null or $base == 0 or . == null then "-" else . / $base - 1 | pct end;
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
	"One build per row: CPU seconds (user plus system time of every process it ran) and retired instructions.",
	"Instructions barely move with machine load, so they are the comparison on a busy host; `vs crane` compares them.",
	"Each variant runs the scenarios in this order on one store, so every row after `clean` starts warm.",
	"Evaluation is timed apart, and the totals add it.",
	"`derivations` counts the Nix derivations that compiled Rust; `compiled` counts rustc runs, as cargo's `Compiling` and `Checking` lines for crane, a derivation per crate for crate2nix, and mbx's misses for mbx; `restored` counts mbx's hits.",
	"",
	(["scenario", "variant", "CPU", "instructions", "vs crane", "wall", "eval CPU", "derivations", "compiled", "restored"] | row),
	"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
	(
		$scenarios[] as $s
		| $variants[] as $v
		| $by["\($s)/\($v)"] // empty
		| [
			$s,
			$v,
			(.build.cpu | secs),
			(.build.instructions | giga),
			(if $v == "crane" then "base" else .build.instructions | vs($by["\($s)/crane"].build.instructions) end),
			(.build.wall | secs),
			(.eval.cpu | secs),
			(.derivations | count),
			(.compiled | count),
			(.restored | count)
		]
		| row
	),
	"",
	(["variant", "total CPU", "total instructions", "vs crane"] | row),
	"| --- | ---: | ---: | ---: |",
	(
		def total($v; f): $results | map(select(.variant == $v) | (.build | f) + (.eval // {} | f // 0)) | add;
		total("crane"; .instructions) as $base
		| $variants[] as $v
		| [$v, (total($v; .cpu) | secs), (total($v; .instructions) | giga),
			(if $v == "crane" then "base" else total($v; .instructions) | vs($base) end)]
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
