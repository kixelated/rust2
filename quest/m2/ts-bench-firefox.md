# [S] Firefox in the TS benchmark

## Goal

`just ts bench` runs every route in Firefox (SpiderMonkey) alongside Chrome, Bun, and
Node, with the same A/A control, in CI and locally.

## Plan

Decided with the maintainer on 2026-10-26: the translator targets browsers, so
browser engines decide speed calls. Chrome (V8) and Bun (JavaScriptCore, as in
Safari) are covered; SpiderMonkey isn't. Node stays as a V8 proxy that's cheap to
run. Drive Firefox through the same harness Chrome uses, and report its numbers in
the bench summary.

Public API: none.
