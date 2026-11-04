# CI Baseline Metrics Report

generated: 2026-11-04T10:59:40.443Z
repos: scarecrow173/ruviform
runs per workflow: up to 5
cache log analysis: run ids 41816075282, 41816923217
contract: docs/designs/issue-327-cross-repo-ci-baseline.md

## scarecrow173/ruviform

### Required contexts

- branch_protection strict=true contexts(12):
  - `verify (ubuntu-latest)`
  - `verify (windows-latest)`
  - `verify (macos-latest)`
  - `l1-real-verify (ubuntu-latest)`
  - `l1-real-verify (windows-latest)`
  - `l1-real-verify (macos-latest)`
  - `ui-verify (ubuntu-latest)`
  - `ui-verify (windows-latest)`
  - `ui-verify (macos-latest)`
  - `rt-verify`
  - `icc-verify`
  - `bench`
- rulesets: 0 entries

### workflow `ci`  (total 156, sampled 5, success&attempt1 3)

- events: {"pull_request":5}  conclusions: {"null":1,"success":4}
- wall_time_s: n=3 values=[294,308,326] (below p50 threshold; raw observations)

| run_id | event | concl | attempt | wall_s | queue_s | exec_span_s | critical_path_s | runner_total_s | jobs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 41817451132 | pull_request | null | 1 | 10 | 0 | 171 | 171 | 699 | 12 |
| 41816923217 | pull_request | success | 2 | 469 | 307 | 466 | 271 | 3180 | 24 |
| 41816549243 | pull_request | success | 1 | 294 | 0 | 290 | 289 | 1534 | 12 |
| 41816075282 | pull_request | success | 1 | 308 | 0 | 304 | 303 | 1730 | 12 |
| 41815446750 | pull_request | success | 1 | 326 | 0 | 322 | 321 | 1834 | 12 |

#### representative run 41816923217 — per-job timings

| job | lane | wait_s | exec_s | runner |
| --- | --- | --- | --- | --- |
| verify (ubuntu-latest) | rust-verify | 0 | 141 | ubuntu-latest |
| icc-verify | icc-verify | 0 | 26 | ubuntu-latest |
| verify (windows-latest) | rust-verify | 0 | 188 | windows-latest |
| ui-verify (macos-latest) | ui-verify | 0 | 227 | macos-latest |
| l1-real-verify (windows-latest) | l1-real-verify | 0 | 75 | windows-latest |
| bench | bench | 0 | 67 | ubuntu-latest |
| ui-verify (ubuntu-latest) | ui-verify | 0 | 191 | ubuntu-latest |
| l1-real-verify (macos-latest) | l1-real-verify | 0 | 62 | macos-latest |
| rt-verify | rt-verify | 0 | 75 | ubuntu-latest |
| ui-verify (windows-latest) | ui-verify | 0 | 230 | windows-latest |
| verify (macos-latest) | rust-verify | 0 | 271 | macos-latest |
| l1-real-verify (ubuntu-latest) | l1-real-verify | 0 | 29 | ubuntu-latest |
| verify (ubuntu-latest) | rust-verify | 5 | 157 | ubuntu-latest |
| rt-verify | rt-verify | 0 | 75 | ubuntu-latest |
| ui-verify (windows-latest) | ui-verify | 0 | 230 | windows-latest |
| icc-verify | icc-verify | 0 | 26 | ubuntu-latest |
| l1-real-verify (windows-latest) | l1-real-verify | 0 | 75 | windows-latest |
| l1-real-verify (ubuntu-latest) | l1-real-verify | 0 | 29 | ubuntu-latest |
| l1-real-verify (macos-latest) | l1-real-verify | 0 | 62 | macos-latest |
| bench | bench | 0 | 67 | ubuntu-latest |
| ui-verify (ubuntu-latest) | ui-verify | 0 | 191 | ubuntu-latest |
| verify (windows-latest) | rust-verify | 0 | 188 | windows-latest |
| ui-verify (macos-latest) | ui-verify | 0 | 227 | macos-latest |
| verify (macos-latest) | rust-verify | 0 | 271 | macos-latest |

- lane totals: {"rust-verify":1216,"icc-verify":52,"ui-verify":1296,"l1-real-verify":332,"bench":134,"rt-verify":150}
- step lane totals: {"runner-overhead":88,"checkout":76,"verify-other":162,"mise":346,"dep-fetch":94,"cache-restore":54,"rust":2164,"playwright":12,"evidence":108}
- cache analysis (job log):
  - verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=14447264
  - icc-verify: verdict=hit hits=1 misses=1 bytes=14447264
  - verify (windows-latest): verdict=hit hits=1 misses=1 bytes=41819369
  - ui-verify (macos-latest): verdict=hit hits=1 misses=1 bytes=10281630
  - l1-real-verify (windows-latest): verdict=hit hits=1 misses=1 bytes=41819369
  - bench: verdict=hit hits=1 misses=1 bytes=14447264
  - ui-verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=13267271
  - l1-real-verify (macos-latest): verdict=hit hits=1 misses=1 bytes=11484267
  - rt-verify: verdict=hit hits=1 misses=1 bytes=14447264
  - ui-verify (windows-latest): verdict=hit hits=1 misses=1 bytes=26220764
  - verify (macos-latest): verdict=hit hits=1 misses=1 bytes=11484267
  - l1-real-verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=14447264
  - verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=14447264
  - rt-verify: verdict=hit hits=1 misses=1 bytes=14447264
  - ui-verify (windows-latest): verdict=hit hits=1 misses=1 bytes=26220764
  - icc-verify: verdict=hit hits=1 misses=1 bytes=14447264
  - l1-real-verify (windows-latest): verdict=hit hits=1 misses=1 bytes=41819369
  - l1-real-verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=14447264
  - l1-real-verify (macos-latest): verdict=hit hits=1 misses=1 bytes=11484267
  - bench: verdict=hit hits=1 misses=1 bytes=14447264
  - ui-verify (ubuntu-latest): verdict=hit hits=1 misses=1 bytes=13267271
  - verify (windows-latest): verdict=hit hits=1 misses=1 bytes=41819369
  - ui-verify (macos-latest): verdict=hit hits=1 misses=1 bytes=10281630
  - verify (macos-latest): verdict=hit hits=1 misses=1 bytes=11484267

## Metric definitions

See docs/designs/issue-327-cross-repo-ci-baseline.md (normative). Raw rows in runs.jsonl.
