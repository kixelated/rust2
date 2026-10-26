# Renders a `just bench-nix` run as Markdown. Inputs: $host (host.json),
# $results (results.jsonl), and $generate (crate2nix's generate timings). Run
# with `jq -n -r`.

def fixed(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
def secs: if . == null then "-" elif . < 10 then fixed(2) else fixed(1) end;
def pct: (. * 100 | fixed(1)) as $p | if . > 0 then "+\($p)%" else "\($p)%" end;
def row: "| " + join(" | ") + " |";
def scenarios: ["clean", "second-package", "edit-hub", "lock-bump"];

$host[0] as $h
| ($results | map({key: "\(.scenario)/\(.variant)", value: .}) | from_entries) as $by
| ($results | map(.variant) | unique) as $variants
| [
	"# Nix build benchmark",
	"",
	"\($h.date) on \($h.cpu) (\($h.cpus) CPUs, \($h.memory)), \($h.os), runner `\($h.runner)`.",
	"rust2 `\($h.rust2[:12])`; moq `\($h.moq[:12])`; \($h.nix); \($h.mbx); \($h.crate2nix).",
	"",
	"CPU and wall seconds of one build each, user plus system time of every process it ran.",
	"Each variant runs the scenarios in order on one store, so each after `clean` starts warm.",
	"`vs crane` compares CPU time; evaluation is timed apart from the build.",
	"`derivations` counts the Nix derivations that compiled Rust, and `compiled` the crates rustc compiled:",
	"cargo's `Compiling` and `Checking` lines for crane, a derivation per crate for crate2nix, and mbx's misses where mbx ran.",
	"",
	(["scenario", "variant", "CPU", "vs crane", "wall", "eval CPU", "derivations", "compiled", "mbx hits"] | row),
	"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
	(
		scenarios[] as $s
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
			(.derivations // "-" | tostring),
			(.mbx.misses // .compiled | tostring),
			(.mbx.hits // "-" | tostring)
		]
		| row
	),
	(
		$variants[] as $v
		| [$results[] | select(.variant == $v) | .build.cpu + (.eval.cpu // 0)] | add
		| [$v, "all scenarios", (. | secs)]
		| "",
		"Total \(.[0]): \(.[2]) CPU seconds over \(.[1]), evaluation included."
	),
	(
		$generate[]
		| "",
		"`crate2nix generate`, which a project reruns when its lockfile changes and commits, took \(.cpu | secs) CPU seconds and \(.wall | secs) wall at the \(.lock) lockfile."
	)
]
| .[]
