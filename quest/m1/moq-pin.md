# [M] Bump the moq pin past the fallback runtime

## Goal

`moq_rev`, the moq revision `ts` and `ffi` both build against, moves to moq's `main` at
[moq-dev/moq#7123](https://github.com/moq-dev/moq/pull/7123)'s merge, so moq-tokio's fallback
runtime reaches the C ABI. `just ffi moq`, `just ts moq`, `just ts census`, `just check`,
and `just test` pass at the new pin.

## Plan

Found on 2026-10-28 against moq's `main`, about 85 commits past the pin: `ffi/moq/ffi.toml`
fails with 17 errors, excludes that name nothing anymore and new items the exporter refuses,
and `just ts census` ICEs on a trait object of an associated type over a type parameter.

- The excludes are config the translators take, so keep them to what the export refuses;
  a refusal that needs a translator change rather than an exclude is a decision to report.
- Report the census before and after for `ts` and `ffi`.
- From C, `moq_origin_spawn` and `moq_connect_config_init` succeed at the new pin; at the
  old one the first aborts. Add that to `moq/moq.c`.
