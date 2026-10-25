//! The census: what a crate's `pub` API exports, and every item the export refuses,
//! ranked by why, so the next mapping to build is picked from numbers.

use std::collections::BTreeMap;
use std::fmt::Write;

use rustc_span::Span;

use crate::abi::Clash;
use crate::collect::{Collected, Refusal};

/// The census of `krate` as Markdown; `at` locates a span.
pub fn render(krate: &str, collected: &Collected, clashes: &[Clash], at: &dyn Fn(Span) -> String) -> String {
	let api = &collected.api;
	let refused = |kinds: &[&str]| collected.refusals.iter().filter(|r| kinds.contains(&r.kind)).count();
	let methods: usize = api.objects.iter().map(|o| o.methods.len()).sum::<usize>()
		+ api.records.iter().map(|r| r.methods.len()).sum::<usize>()
		+ api.enums.iter().map(|e| e.methods.len()).sum::<usize>();
	let types = api.records.len() + api.enums.len() + api.objects.len();

	let mut out = String::new();
	let _ = writeln!(out, "# Export census of `{krate}`\n");
	let _ = writeln!(
		out,
		"| Item | Exported | Refused | Share exported |\n| --- | ---: | ---: | ---: |"
	);
	let mut row = |name: &str, exported: usize, refused: usize| {
		let total = exported + refused;
		let share = match total {
			0 => "-".to_string(),
			_ => format!("{}%", exported * 100 / total),
		};
		let _ = writeln!(out, "| {name} | {exported} | {refused} | {share} |");
	};
	row("Types", types, refused(&["struct", "enum", "union"]));
	row("Functions", api.functions.len(), refused(&["fn"]));
	row("Methods of exported types", methods, refused(&["method"]));
	row("Traits", api.callbacks.len(), refused(&["trait"]));
	row("Constants", api.constants.len(), refused(&["const"]));
	row("Other items", 0, refused(&["use", "macro", "static", "item"]));
	let _ = writeln!(
		out,
		"\nExported types: {} records, {} enums, {} objects. A refused type's methods are not counted.",
		api.records.len(),
		api.enums.len(),
		api.objects.len()
	);

	// Each item counts once per construct, however many times it hits it.
	let mut constructs: BTreeMap<&str, Vec<&Refusal>> = BTreeMap::new();
	let mut details: BTreeMap<(&str, &str), Vec<&Refusal>> = BTreeMap::new();
	let mut occurrences: BTreeMap<&str, usize> = BTreeMap::new();
	for refusal in &collected.refusals {
		let mut seen = Vec::new();
		for why in &refusal.why {
			*occurrences.entry(why.construct).or_default() += 1;
			if !seen.contains(&why.construct) {
				seen.push(why.construct);
				constructs.entry(why.construct).or_default().push(refusal);
			}
			if !why.detail.is_empty() {
				let list = details.entry((why.construct, &why.detail)).or_default();
				if !list.iter().any(|r| std::ptr::eq(*r, refusal)) {
					list.push(refusal);
				}
			}
		}
	}

	let _ = writeln!(out, "\n## Refusals by construct\n");
	let _ = writeln!(
		out,
		"Items refused for each construct; an item refused for several counts under each.\n"
	);
	let _ = writeln!(out, "| Items | Uses | Construct | First |\n| ---: | ---: | --- | --- |");
	for (construct, list) in ranked(&constructs) {
		let _ = writeln!(
			out,
			"| {} | {} | {} | {} |",
			list.len(),
			occurrences[construct],
			construct,
			at(list[0].span)
		);
	}

	let _ = writeln!(out, "\n## Refusals by type or path\n");
	let _ = writeln!(out, "| Items | Construct | Names | First |\n| ---: | --- | --- | --- |");
	for ((construct, detail), list) in ranked(&details) {
		let _ = writeln!(
			out,
			"| {} | {} | `{}` | {} |",
			list.len(),
			construct,
			detail.replace('|', "\\|"),
			at(list[0].span)
		);
	}

	let _ = writeln!(out, "\n## Refused items\n");
	let mut refusals: Vec<&Refusal> = collected.refusals.iter().collect();
	refusals.sort_by(|a, b| a.path.cmp(&b.path));
	for refusal in refusals {
		let reasons: Vec<String> = refusal
			.why
			.iter()
			.map(|why| match why.detail.is_empty() {
				true => why.construct.to_string(),
				false => format!("{} `{}`", why.construct, why.detail),
			})
			.collect();
		let _ = writeln!(
			out,
			"- {} `{}` at {}: {}",
			refusal.kind,
			refusal.path,
			at(refusal.span),
			reasons.join("; ")
		);
	}

	if !collected.refusals.is_empty() {
		let _ = writeln!(out, "\n## The config that exports what maps\n");
		let _ = writeln!(out, "```toml\nexclude = [");
		let mut paths: Vec<&str> = collected.refusals.iter().map(|r| r.path.as_str()).collect();
		paths.sort();
		paths.dedup();
		for path in paths {
			let _ = writeln!(out, "    \"{path}\",");
		}
		let _ = writeln!(out, "]\n```");
	}
	if !clashes.is_empty() {
		let _ = writeln!(out, "\n## C names claimed twice\n");
		for (name, (first, _), (second, span)) in clashes {
			let _ = writeln!(out, "- `{name}`: {first}, and {second} at {}", at(*span));
		}
	}
	if !collected.opaque.is_empty() {
		let _ = writeln!(out, "\n## Values exported as objects, since a field does not map\n");
		for (path, why) in &collected.opaque {
			let _ = writeln!(out, "- `{path}`: {} `{}`", why.construct, why.detail);
		}
	}
	if !collected.excluded.is_empty() {
		let _ = writeln!(out, "\n## Excluded by the config\n");
		for (path, count) in &collected.excluded {
			let _ = writeln!(out, "- `{path}`: {count} items");
		}
	}
	if !collected.hidden.is_empty() {
		let _ = writeln!(out, "\n## `#[doc(hidden)]`, left out\n");
		for path in &collected.hidden {
			let _ = writeln!(out, "- `{path}`");
		}
	}
	if !collected.aliases.is_empty() {
		let _ = writeln!(out, "\n## Type aliases, which export nothing of their own\n");
		for path in &collected.aliases {
			let _ = writeln!(out, "- `{path}`");
		}
	}
	out
}

/// Groups by count, most first, then by key.
fn ranked<K: Ord, V>(groups: &BTreeMap<K, Vec<V>>) -> Vec<(&K, &Vec<V>)> {
	let mut ranked: Vec<_> = groups.iter().collect();
	ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(b.0)));
	ranked
}
