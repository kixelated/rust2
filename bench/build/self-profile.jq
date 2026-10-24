# Input: every `summarize --json` output of one build (`jq -n`, read with
# `inputs` so each keeps its file name). Output: seconds summed over all rustc
# invocations and threads, overall and per invocation: macro expansion, the
# costliest queries, and the self-time of each compiler stage.

def secs: .secs + .nanos / 1e9;

# Each query's compiler stage. Expansion, name resolution, and HIR lowering
# rerun in full for every rebuilt crate. The backend is what Cranelift replaces:
# building LLVM IR and running LLVM, not monomorphization. Analysis is every
# query not named here (type and borrow checking, MIR, lints).
def stage:
	if test("^(LLVM_|codegen_crate$|codegen_module)") then "backend"
	elif test("^(run_linker|link_)") then "link"
	elif test("^(incr_comp_|encode_query_results|drop_dep_graph|write_dep_info|codegen_copy_artifacts_from_incr_cache|copy_all_cgu_workproducts)") then "incremental"
	elif test("^(generate_crate_metadata|metadata_)") then "metadata"
	elif test("^(parse_crate|expand_|macro_expand|late_resolve_crate|resolve_(crate|main|postprocess|check_unused|report_errors)$|lower_to_hir|early_lint_checks|pre_AST_expansion|AST_validation|drop_ast|index_ast|complete_gated_feature_checking|crate_injection|configure_and_expand|setup_global_ctxt|hir_crate)") then "expand"
	elif test("^(monomorphization_|items_of_instance|cgu_|codegen_unit$|collect_and_partition_mono_items|assert_symbols_are_distinct|symbol_name|find_cgu_reuse|size_estimate)") then "mono"
	else "analysis"
	end;

def stages: group_by(.label | stage) | map({key: (.[0].label | stage), value: (map(.self_time | secs) | add)}) | from_entries;

[
	inputs
	| {
		# rustc names each profile <crate>-<pid>.
		crate: (input_filename | split("/") | last | sub("-[0-9]+\\.json$"; "")),
		# Only a crate others link against writes metadata: a lib or proc macro, not a binary or build script.
		metadata: any(.artifact_sizes[]; .label == "crate_metadata"),
		queries: .query_data
	}
] as $runs
| [$runs[].queries[]] as $queries
| def total($name): [$queries[] | select(.label == $name) | .time | secs] | add // 0;
def own($name): [$queries[] | select(.label == $name) | .self_time | secs] | add // 0;
{
	invocations: ($runs | length),
	self_time: ([$queries[].self_time | secs] | add),
	# All expansion work, macro_rules and name resolution of expanded code included.
	macro_expansion: total("macro_expand_crate"),
	# Running proc macros, derives included (they nest expand_proc_macro).
	proc_macros: (total("expand_proc_macro") + own("expand_derive_proc_macro_outer")),
	top: ($queries | group_by(.label) | map({label: .[0].label, self_time: (map(.self_time | secs) | add)})
		| sort_by(-.self_time) | .[:12]),
	stages: ($queries | stages),
	crates: ($runs | map({crate, metadata, stages: (.queries | stages)}))
}
