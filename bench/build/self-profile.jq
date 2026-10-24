# Input: every `summarize --json` output of one build (`jq -s`). Output: macro
# expansion time and the costliest queries, in seconds summed over all rustc
# invocations and threads.

def secs: .secs + .nanos / 1e9;

[.[].query_data[]] as $queries
| def total($name): [$queries[] | select(.label == $name) | .time | secs] | add // 0;
def own($name): [$queries[] | select(.label == $name) | .self_time | secs] | add // 0;
{
	invocations: length,
	self_time: ([$queries[].self_time | secs] | add),
	# All expansion work, macro_rules and name resolution of expanded code included.
	macro_expansion: total("macro_expand_crate"),
	# Running proc macros, derives included (they nest expand_proc_macro).
	proc_macros: (total("expand_proc_macro") + own("expand_derive_proc_macro_outer")),
	top: ($queries | group_by(.label) | map({label: .[0].label, self_time: (map(.self_time | secs) | add)})
		| sort_by(-.self_time) | .[:12])
}
