# Input: the events of one cargo build analysis log (`jq -s`), plus $corpus,
# $step, $proc_macros (package ids), and $profile (the self-profile summary).
# Output: where the build's time went, in unit-seconds per kind of unit and per
# compiler section, and per compiled unit with its share of the critical path.

def by_index: map({key: (.index | tostring), value: .}) | from_entries;

def class:
	if .mode == "run-custom-build" then "build script (run)"
	elif .target.kind == "build-script" then "build script (compile)"
	elif .package_id as $id | any($proc_macros[]; . == $id) then "proc macro"
	elif .target.kind != "lib" then "binary"
	elif .package_id | startswith("path+") then "workspace lib"
	else "dependency"
	end;

(map(select(.reason == "unit-registered")) | by_index) as $units
| (map(select(.reason == "unit-started")) | by_index) as $started
| (map(select(.reason == "unit-rmeta-finished")) | by_index) as $rmeta
| (map(select(.reason == "unit-finished")) | by_index) as $finished
| (map(select(.reason == "unit-fingerprint")) | by_index) as $fingerprints
| (map(select(.reason | startswith("unit-section-")))
	| group_by(.index)
	| map({key: (.[0].index | tostring), value: (map({key: "\(.section).\(.reason | ltrimstr("unit-section-"))", value: .elapsed}) | from_entries)})
	| from_entries) as $sections
# The event of compiled unit $d that let a dependent starting at $t go: its
# metadata when the dependent pipelines on it, else its finish.
| def released($d; $t): [$rmeta[$d].elapsed, $finished[$d].elapsed | values | select(. <= $t)] | max;
# The compiled dependency that $i started last after, and when.
def blocker($i):
	$started[$i].elapsed as $t
	| [($units[$i].dependencies // [])[] | tostring | select($started[.]) | {unit: ., at: released(.; $t)}]
	| max_by(.at);
# The critical path back from unit $i, which held it until $until.
def path($i; $until):
	blocker($i) as $b
	| [{key: $i, value: ($until - $started[$i].elapsed)}] + (if $b then path($b.unit; $b.at) else [] end);
($finished | to_entries | max_by(.value.elapsed)) as $last
| (if $last then path($last.key; $last.value.elapsed) | from_entries else {} end) as $path
| [
	$started | keys[]
	| . as $i
	| $units[$i] as $unit
	| $started[$i].elapsed as $start
	| ($sections[$i] // {}) as $s
	| ($finished[$i].elapsed - $start) as $total
	| (if $unit.mode == "run-custom-build" then 0
		else (($s["codegen.started"] // $s["link.started"] // $finished[$i].elapsed) - $start) end) as $frontend
	| (($s["codegen.finished"] // 0) - ($s["codegen.started"] // 0)) as $codegen
	| (($s["link.finished"] // 0) - ($s["link.started"] // 0)) as $link
	# rustc names the crate after the target, and only a lib or proc macro writes metadata.
	| ($unit.target.name | gsub("-"; "_")) as $crate
	| ($unit.target.kind == "lib") as $metadata
	| [$profile[0].crates[] | select(.crate == $crate and .metadata == $metadata)] as $profiles
	| {
		name: $unit.target.name,
		class: ($unit | class),
		# Whether a source file changed, or only something the unit depends on.
		cause: ($fingerprints[$i].cause | if .fs_status == "stale-item" then "edited"
			elif .fs_status == "stale-dep-fingerprint" then "dependency" else null end),
		$start,
		path: ($path[$i] // 0),
		$total, $frontend, $codegen, $link,
		other: ($total - $frontend - $codegen - $link),
		# Duplicate crate names (two versions, or many build scripts) leave the stages unknown.
		stages: (if ($profiles | length) == 1 then $profiles[0].stages else null end)
	}
] as $compiled
| {
	$corpus,
	$step,
	wall: ([$finished[].elapsed] | max),
	# Cargo's own time before the first unit started, the head of the critical path.
	startup: ([$started[].elapsed] | min),
	classes: ($compiled | group_by(.class) | map({
		class: .[0].class,
		units: length,
		frontend: (map(.frontend) | add),
		codegen: (map(.codegen) | add),
		link: (map(.link) | add),
		other: (map(.other) | add),
		total: (map(.total) | add)
	}) | sort_by(-.total)),
	units: ($compiled | sort_by(.start)),
	profile: ($profile[0] | del(.crates))
}
