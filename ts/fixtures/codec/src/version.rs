// Trimmed from moq-net's version.rs: only the enum the codec matches on.

use crate::{ietf, lite};

/// A MoQ protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Version {
	/// A `moq-lite` draft, the simplified protocol this project specifies.
	Lite(lite::Version),
	/// An IETF `moq-transport` draft.
	Ietf(ietf::Version),
}
