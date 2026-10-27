// `bytes::Bytes`, bound wrong: `slice` is missing, `len` returns a string, and `eq` takes
// what no `Bytes` is.

import * as rt from "../rt.ts";

export type Bytes = rt.ByteSlice;

export function copyFromSlice(data: rt.ByteSlice): Bytes {
	return data;
}

export function from$From_Vec_u8(vec: rt.ByteVec): Bytes {
	return rt.byteVecSlice(vec);
}

export function len(self: Bytes): string {
	return `${self.length}`;
}

export function isEmpty(self: Bytes): boolean {
	return self.length === 0;
}

export function clone$Clone(self: Bytes): Bytes {
	return self;
}

export function deref$Deref(self: Bytes): rt.ByteSlice {
	return self;
}

export function eq$PartialEq(self: Bytes, other: string): boolean {
	return self.length === other.length;
}
