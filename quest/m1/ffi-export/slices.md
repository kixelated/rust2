# [S] Slices of any type, and SocketAddrV4/V6

## Goal

`&[T]` of any type the exporter maps, and `std::net::SocketAddrV4` and `SocketAddrV6`,
cross the C ABI, so the census stops refusing them. Proven by the api fixture's transcript
from C, C++, and Kotlin, and from C in `moq/moq.c`.

## Plan

Found by the census on 2026-10-28, after
[#190](https://github.com/kixelated/rust2/pull/190) mapped `Vec<T>` as
`<prefix>_vec_<T>` and `SocketAddr` as `<prefix>_socket_addr`.

- A slice crosses as the same `<prefix>_vec_<T>`: a view passed in, and returned as an
  owned copy, as a returned `&str` is. A slice of references follows `Vec<&T>`'s rules.
  The 12 refusals at the pinned moq: `Pattern::segments`, `connect::Addr::addresses`,
  `Addrs::as_slice`, `Hops::as_slice`, `container::fmp4::Muxer::fragment`,
  `tls::PeerIdentity::chain`, `cli::check_env` (twice), and the constants `ALPNS`,
  `stats::Failure::ALL`, `stats::RefusalKind::ALL`, and `accept::Failure::ALL`.
- `SocketAddrV4` and `SocketAddrV6` share `<prefix>_socket_addr`, with `v6` fixed by the
  type: one passed in of the other family aborts, as an IPv4 address with IPv6 bytes does.
  Kotlin keeps `java.net.InetSocketAddress`. They refuse moq-tokio's
  `listen::Config::preferred_v4` and `preferred_v6`.
- Report the census before and after.

Public API: the C names the exporter emits.

## Related

- [Setters for an object's pub fields](/quest/m1/ffi-export/field-setters.md) - `preferred_v4` and `preferred_v6` are fields C sets
