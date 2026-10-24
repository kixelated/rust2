use crate::coding::{Decode, DecodeError, Decoder, Encode, EncodeError, Encoder};

use super::Version;

// Lite control messages are buffered whole before decoding, so the limit is checked as
// soon as the length prefix arrives. The same ceiling as SETUP: paths, track names, and
// hop chains fit with room to spare, and the JavaScript reader matches it.
pub(super) const MAX_MESSAGE_SIZE: usize = u16::MAX as usize;

/// Read a lite message's varint size prefix, refusing one past `max`.
pub(super) fn decode_size(r: &mut Decoder<'_>, max: usize) -> Result<usize, DecodeError> {
	let size = r.varint()?;
	match usize::try_from(size) {
		Ok(size) if size <= max => Ok(size),
		_ => Err(DecodeError::MessageTooLarge {
			size: usize::try_from(size).unwrap_or(usize::MAX),
			max,
		}),
	}
}

/// A trait for lite messages that are automatically size-prefixed during encoding/decoding.
///
/// Lite messages use a varint size prefix.
pub trait Message: Sized + std::fmt::Debug {
	/// The control metric class, or none for media headers.
	const CONTROL: Option<crate::stats::Control> = None;
	/// The largest body this receiver accepts for this message.
	const MAX_SIZE: usize = MAX_MESSAGE_SIZE;

	/// Encode this message body (without size prefix).
	fn encode_msg(&self, w: &mut Encoder<'_>, version: Version) -> Result<(), EncodeError>;

	/// Decode a message body (without size prefix).
	fn decode_msg(r: &mut Decoder<'_>, version: Version) -> Result<Self, DecodeError>;
}

impl<T: Message> Encode<Version> for T {
	fn encode(&self, w: &mut Encoder<'_>, version: Version) -> Result<(), EncodeError> {
		tracing::trace!(?self, "encoding");
		let prefix = w.prefix_varint();
		self.encode_msg(w, version)?;
		// Never emit a body our own receiver would refuse.
		if w.since(&prefix) > Self::MAX_SIZE {
			w.discard(prefix);
			return Err(EncodeError::TooLarge);
		}
		w.fill(prefix)?;
		w.control = T::CONTROL;
		Ok(())
	}
}

impl<T: Message> Decode<Version> for T {
	fn decode(r: &mut Decoder<'_>, version: Version) -> Result<Self, DecodeError> {
		decode_with(r, |body| T::decode_msg(body, version))
	}
}

/// Decode a size-prefixed `T`, reading its body with `decode` in place of
/// [`Message::decode_msg`], for a body whose bounds come from the session.
pub(super) fn decode_with<T: Message>(
	r: &mut Decoder<'_>,
	decode: impl FnOnce(&mut Decoder<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
	let size = decode_size(r, T::MAX_SIZE)?;
	let mut body = r.sub(size)?;

	// The body is complete, so running short inside it is malformed, not a wait for more.
	let result = decode(&mut body)
		.map_err(DecodeError::complete)
		.and_then(|msg| match body.is_empty() {
			true => Ok(msg),
			false => Err(DecodeError::Long),
		});

	match &result {
		Ok(msg) => tracing::trace!(?msg, "decoded"),
		Err(err) => tracing::warn!(%err, "decode failed"),
	}
	if result.is_ok() {
		r.control = T::CONTROL;
	}
	result
}
