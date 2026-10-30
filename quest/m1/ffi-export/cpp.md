# [S] C++ bindings on MSVC and Apple Clang

## Goal

The nightly's `Export moq-net to C++` jobs pass on MSVC and Apple Clang: the exporter's C++
header passes `just ffi test cpp` and `just ffi moq-cpp` with MSVC on Windows and Apple
Clang on macOS, as it does with GCC and Clang on Linux.

## Plan

- The jobs landed before any run of theirs: read the first nightly's, and fix what it
  finds at the source, in the generator or the recipes, never by loosening flags.
- Unproven on those runners: the exporter, a rustc driver, running on Windows; cl's flags
  and the environment `Launch-VsDevShell.ps1` sets; the standin's paths under Git Bash;
  and MSVC's `/W4` warnings with no GCC or Clang counterpart, such as printf's formats.
