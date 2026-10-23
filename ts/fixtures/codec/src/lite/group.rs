use crate::coding::*;

use super::{Message, Version};

#[derive(Clone, Debug)]
pub struct Group {
	// The subscribe ID.
	pub subscribe: u64,

	// The group sequence number
	pub sequence: u64,

	// The index of the first frame on this stream. 0 (the common case) means the stream
	// carries the group from its beginning; a higher value means the leading frames are
	// missing, either because the subscription started partway in or because the
	// publisher only holds the tail. Lite06+ only; older versions always start at 0.
	pub frame_start: u64,
}

impl Message for Group {
	fn decode_msg(r: &mut Decoder<'_>, version: Version) -> Result<Self, DecodeError> {
		let subscribe = r.varint()?;
		let sequence = r.varint()?;
		let frame_start = match version.has_frame_bounds() {
			true => r.varint()?,
			false => 0,
		};

		Ok(Self {
			subscribe,
			sequence,
			frame_start,
		})
	}

	fn encode_msg(&self, w: &mut Encoder<'_>, version: Version) -> Result<(), EncodeError> {
		w.varint(self.subscribe)?;
		w.varint(self.sequence)?;

		if version.has_frame_bounds() {
			w.varint(self.frame_start)?;
		} else if self.frame_start != 0 {
			// The peer would number the frames from 0 and silently misalign the group.
			return Err(EncodeError::Version);
		}

		Ok(())
	}
}
