# Renders a run as Markdown. Inputs: $host (host.json), $results (results.jsonl),
# and $attribution (every attribution summary). Run with `jq -n -r`.

def fixed(p): (. * pow(10; p) | round) / pow(10; p) | tostring;
def secs: if . < 10 then fixed(2) else fixed(1) end;
def pct: (. * 100 | fixed(1)) as $p | if . > 0 then "+\($p)%" else "\($p)%" end;
def share($whole): if $whole > 0 then (. / $whole * 100 | round | tostring) + "%" else "-" end;
def row: "| " + join(" | ") + " |";

$host[0] as $h
| [
	"# Build benchmark",
	"",
	"\($h.date) on \($h.cpu) (\($h.cpus) CPUs, \($h.memory)), \($h.os), runner `\($h.runner)`.",
	"rust2 `\($h.rust2[:12])`; \($h.stable); \($h.nightly); \($h.hyperfine); \($h.mold); \($h.wild).",
	"",
	"Wall time in seconds: the median and mean ± standard deviation over the runs.",
	"The change against the variant's base compares medians, so one slow run does not move it; CPU time and instructions are medians too.",
	"Each control variant repeats its base, so its change is this run's noise floor.",
	(
		$results | group_by(.corpus)[] | . as $runs | .[0].corpus as $corpus
		| ($runs | map({key: "\(.scenario)/\(.variant)", value: .}) | from_entries) as $by
		| "",
		"## \($corpus)",
		"",
		(["scenario", "variant", "median", "mean ± σ", "vs base", "CPU", "instructions vs base", "peak RSS"] | row),
		"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
		(
			$runs[]
			| $by["\(.scenario)/\(.base)"] as $base
			| (.variant == .base) as $is_base
			| [
				.scenario,
				.variant,
				(.median | secs),
				"\(.mean | secs) ± \(.stddev // 0 | secs)",
				(if $is_base then "base" else (.median / $base.median - 1 | pct) end),
				(.cpu | secs),
				(if .instructions == null then "n/a"
					elif $is_base then "\(.instructions / 1e9 | fixed(1))G"
					else (.instructions / $base.instructions - 1 | pct) end),
				"\(.rss / 1048576 | round) MiB"
			]
			| row
		)
	),
	(
		$attribution | group_by(.corpus)[]
		| "",
		"## Where the time goes: \(.[0].corpus)",
		"",
		"Nightly, with self-profiling on. Unit-seconds summed over every crate cargo compiled, so parallel work adds up past the wall time.",
		(
			.[]
			| ([.classes[].total] | add) as $all
			| "",
			"### \(.step): \(.wall | secs) s wall, \($all | secs) unit-s",
			"",
			(["unit", "count", "frontend", "codegen", "link", "other", "total", "share"] | row),
			"| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
			(
				.classes[]
				| [.class, (.units | tostring), (.frontend | secs), (.codegen | secs), (.link | secs), (.other | secs), (.total | secs), (.total | share($all))]
				| row
			),
			"",
			"Macro expansion \(.profile.macro_expansion | secs) s, running proc macros \(.profile.proc_macros | secs) s, of \(.profile.self_time | secs) s rustc self-time across \(.profile.invocations) invocations.",
			"Costliest queries: \(.profile.top | map("`\(.label)` \(.self_time | secs) s") | join(", ")).",
			(
				# A clean build compiles hundreds of units; an edit's few are worth listing.
				select(.step | startswith("edit-"))
				| .wall as $wall
				| ([.units[] | select(.cause == "dependency") | .path] | add // 0) as $carried
				| "",
				"Each rebuilt unit: why cargo rebuilt it, the wall seconds it held the critical path, and rustc's self-time by stage.",
				"",
				(["unit", "rebuilt for", "critical path", "expand & resolve", "analysis", "metadata", "incremental", "mono", "backend", "link"] | row),
				"| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
				(["cargo", "", (.startup | secs), "", "", "", "", "", "", ""] | row),
				(
					.units[]
					| [
						"\(.name) (\(.class))",
						(.cause // "?"),
						(if .path > 0 then .path | secs else "" end),
						(.stages as $s | ["expand", "analysis", "metadata", "incremental", "mono", "backend", "link"][]
							| if $s == null then "?" else $s[.] // 0 | secs end)
					]
					| row
				),
				"",
				"Units rebuilt only because a dependency changed hold \($carried | secs) s of the \($wall | secs) s critical path (\($carried | share($wall)))."
			)
		)
	)
]
| .[]
