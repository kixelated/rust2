// The `land` environment holds the key of the App that main's ruleset lets bypass it,
// so only the Land workflow's land job may name an environment. Any other environment
// reference counts, since an expression can spell `land` without the word.
import { deepStrictEqual } from "node:assert";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

type Job = { environment?: unknown; uses?: unknown };

/** The `workflow:job` of every job that names an environment or calls another repository's workflow, which could. */
function users(name: string, text: string): string[] {
	const jobs: Record<string, Job> = Bun.YAML.parse(text)?.jobs ?? {};
	return Object.entries(jobs)
		.filter(([, job]) => job.environment !== undefined || (typeof job.uses === "string" && !job.uses.startsWith("./")))
		.map(([id]) => `${name}:${id}`);
}

const fixtures: [string, string[]][] = [
	["jobs:\n  build:\n    environment: land\n", ["ci.yml:build"]],
	["jobs:\n  build:\n    environment:\n      name: land\n", ["ci.yml:build"]],
	["jobs:\n  build: { environment: { name: land } }\n", ["ci.yml:build"]],
	["jobs:\n  build:\n    environment: ${{ format('{0}nd', 'la') }}\n", ["ci.yml:build"]],
	["jobs:\n  build:\n    uses: someone/else/.github/workflows/steal.yml@main\n", ["ci.yml:build"]],
	["jobs:\n  build:\n    uses: ./.github/workflows/check.yml\n", []],
	["jobs:\n  build:\n    runs-on: ubuntu-latest\n", []],
];
for (const [text, expected] of fixtures) deepStrictEqual(users("ci.yml", text), expected, text);

// GitHub runs both extensions.
const workflows = join(import.meta.dir, "workflows");
const found = readdirSync(workflows)
	.filter((name) => /\.ya?ml$/.test(name))
	.flatMap((name) => users(name, readFileSync(join(workflows, name), "utf8")));
deepStrictEqual(found, ["land.yml:land"], "only land.yml's land job may name an environment");
