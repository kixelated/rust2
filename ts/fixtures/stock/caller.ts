// The harness, with inputs it names for `differential.ts`: `buf::reserves`' edges of
// `rt.BytesMut`'s limit, which only these reach, as each allocates 512 MiB.

import { cases as all, run } from "./harness.ts";

export { run };

const reserves = all().length - 1;

/** Each edge but shared storage past the limit, which throws, read past by none and by some. */
export const cases: Uint8Array[] = [0, 1, 2].flatMap((kind) => [0, 3].map((read) => Uint8Array.of(reserves, ...new TextEncoder().encode("edge"), kind, read)));
