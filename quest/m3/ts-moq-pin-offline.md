# [XS] moq-pin without a network when the pin is present

## Goal

`just ts moq-pin` makes no network request when `target/moq` already holds the pinned
revision, so `just ci` doesn't fail on a fetch that changes nothing.

## Plan

Found by [#340](https://github.com/moq-dev/rust2/pull/340)'s agent, whose local
`just ci` sometimes failed in `moq-pin`'s quiet `git fetch` (`ts/justfile`), which no
change of its touched; each rerun passed. Fetch only when `git cat-file -e` doesn't find
the pinned commit. A failed fetch still fails loud, with git's error shown.
