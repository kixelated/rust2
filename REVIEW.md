# Review

Be adversarial: assume the change is wrong until the diff and its tests show otherwise.

- Generated output must match native Rust exactly, including integer widths and overflow, drop order, laziness, and error paths. Ask for the differential case that proves it.
- An unsupported construct must be refused with a source span, never a silent fallback or a runtime guess.
- A test must fail without the change it covers.
- Report real defects only; leave style to `just check` and AGENTS.md.
