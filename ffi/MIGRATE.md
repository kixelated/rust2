# Migrating

## Unreleased

- In `exclude` and `rename`, a path that a field shares with a method or constant of its
  type, such as `Backoff::initial`, names only the method, as in rustdoc. Name the field
  `field@Backoff::initial`: an `exclude` that left out both now needs both entries.
