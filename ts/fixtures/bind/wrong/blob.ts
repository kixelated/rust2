// `blob::Blob`, bound wrong: `slice` is missing, `len` returns a string, and `eq` takes
// what no `Blob` is.

import * as rt from "../rt.ts";

export type Blob = rt.ByteSlice;

export function copyFromSlice(data: rt.ByteSlice): Blob {
	return data;
}

export function from$From_Vec_u8(vec: rt.ByteVec): Blob {
	return rt.byteVecSlice(vec);
}

export function len(self: Blob): string {
	return `${self.length}`;
}

export function isEmpty(self: Blob): boolean {
	return self.length === 0;
}

export function clone$Clone(self: Blob): Blob {
	return self;
}

export function deref$Deref(self: Blob): rt.ByteSlice {
	return self;
}

export function eq$PartialEq(self: Blob, other: string): boolean {
	return self.length === other.length;
}
