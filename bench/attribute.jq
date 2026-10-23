# Input: the events of one cargo build analysis log (`jq -s`), plus $corpus,
# $step, $proc_macros (package ids), and $profile (the self-profile summary).
# Output: where the build's time went, in unit-seconds per kind of unit and per
# compiler section.

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
| (map(select(.reason == "unit-finished")) | by_index) as $finished
| (map(select(.reason | startswith("unit-section-")))
	| group_by(.index)
	| map({key: (.[0].index | tostring), value: (map({key: "\(.section).\(.reason | ltrimstr("unit-section-"))", value: .elapsed}) | from_entries)})
	| from_entries) as $sections
| [
	.[] | select(.reason == "unit-started")
	| (.index | tostring) as $i
	| $units[$i] as $unit
	| ($sections[$i] // {}) as $s
	| ($finished[$i].elapsed - .elapsed) as $total
	| (if $unit.mode == "run-custom-build" then 0
		else (($s["codegen.started"] // $s["link.started"] // $finished[$i].elapsed) - .elapsed) end) as $frontend
	| (($s["codegen.finished"] // 0) - ($s["codegen.started"] // 0)) as $codegen
	| (($s["link.finished"] // 0) - ($s["link.started"] // 0)) as $link
	| {class: ($unit | class), $total, $frontend, $codegen, $link, other: ($total - $frontend - $codegen - $link)}
]
| {
	$corpus,
	$step,
	wall: ([$finished[].elapsed] | max),
	classes: (group_by(.class) | map({
		class: .[0].class,
		units: length,
		frontend: (map(.frontend) | add),
		codegen: (map(.codegen) | add),
		link: (map(.link) | add),
		other: (map(.other) | add),
		total: (map(.total) | add)
	}) | sort_by(-.total)),
	profile: $profile[0]
}
