# [XS] Bump quest to quest land

## Goal

Condition: a kixelated/quest commit on `main` ships `quest land`
([its plan](https://github.com/kixelated/quest/blob/main/quest/m0/helpers/land.md)).
Check with `git ls-remote https://github.com/kixelated/quest main` and
`quest land --help` from that revision. Then bump the `quest` flake input in
`flake.nix` to that revision, and delete this quest.
