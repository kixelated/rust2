# [M] C++ bindings on MSVC and Clang

## Goal

The exporter's C++ header, which GCC builds and tests on Linux in `just ffi test` and
`just ffi moq`, builds and passes the same tests with MSVC on Windows and Clang on Linux
and macOS, at C++17, 20, and 23, with and without exceptions and RTTI, in CI at least
nightly.

## Plan

- One nightly job per platform, as `uniffi-cpp` runs the fork's. The exporter is a rustc
  driver that runs on the host, so Windows either runs it there or builds an export made
  on Linux; the tests' link flags (`-lpthread -ldl -lm`) and compiler flags are GCC's and
  need MSVC's.
- Clang 21 passed every configuration locally when the generator landed; CI has no Clang
  yet.
