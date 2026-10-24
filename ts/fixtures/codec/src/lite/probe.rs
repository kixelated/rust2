use crate::coding::*;

use super::{Message, Version};

/// Sent to probe the available bitrate and round-trip time.
///
/// Lite03+. Lite04 adds the `rtt` field.
/// On the wire, 0 means unknown (None). Some(0) is rounded up to Some(1).
#[derive(Clone, Debug)]
pub struct Probe {
	pub bitrate: Option<u64>,
	pub rtt: Option<u64>,
}

impl Message for Probe {
	const CONTROL: Option<crate::stats::Control> = Some(crate::stats::Control::Probe);
	fn decode_msg(r: &mut Decoder<'_>, version: Version) -> Result<Self, DecodeError> {
		match version {
			Version::Lite01 | Version::Lite02 => {
				return Err(DecodeError::Version);
			}
			_ => {}
		}

		// 0 means unknown, the same as RTT below. A publisher whose transport
		// exposes no congestion controller reports the RTT half alone.
		let bitrate = match r.varint()? {
			0 => None,
			v => Some(v),
		};
		let rtt = match version.has_probe_rtt() {
			false => None,
			true => match r.varint()? {
				0 => None,
				v => Some(v),
			},
		};

		Ok(Self { bitrate, rtt })
	}

	fn encode_msg(&self, w: &mut Encoder<'_>, version: Version) -> Result<(), EncodeError> {
		match version {
			Version::Lite01 | Version::Lite02 => {
				return Err(EncodeError::Version);
			}
			_ => {}
		}

		// 0 means unknown; round Some(0) up to 1.
		let wire = self.bitrate.map(|v| v.max(1)).unwrap_or(0);
		w.varint(wire)?;
		if version.has_probe_rtt() {
			let wire = self.rtt.map(|v| v.max(1)).unwrap_or(0);
			w.varint(wire)?;
		}
		Ok(())
	}
}
