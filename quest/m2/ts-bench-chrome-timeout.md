# [XS] Fail the TS benchmark when Chrome hangs

## Goal

A hung Chromium renderer fails `just ts bench` loudly, naming the engine and what
it was running, instead of stalling the run until the job times out.

## Plan

Seen twice overnight, first in [#15](https://github.com/kixelated/rust2/pull/15):
the renderer sat idle and never posted a result, while a rerun passed. The limit
only turns a hang into a failure; it never retries, and if the hang's cause shows
up, fix it at the source.

Public API: none.
