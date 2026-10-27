// `raw::Held`, bound wrong: it is what it holds, so a `Held<Option<u8>>` may be `undefined`,
// as `None` is.

export type Held<T> = T;

function make<T>(value: T): Held<T> {
	return value;
}
export { make as new };
