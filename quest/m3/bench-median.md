# [XS] Median for short benchmark scenarios

## Goal

The [build-speed benchmark](/bench/build/README.md) reports scenarios under
10 s robustly to an occasional outlier, by the median beside the mean or by
trimming outliers, so edit rebuild numbers compare night to night.

## Plan

Decided with the maintainer on 2026-10-23, from the first report: short
scenarios get occasional outliers that skew the mean, and the min column was the
workaround.

Public API: the benchmark report's columns.
