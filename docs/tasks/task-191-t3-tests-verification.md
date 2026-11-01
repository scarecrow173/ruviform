---
id: "TASK-034QhxcLDQJmQx1cut9ge3"
type: "task"
status: "done"
title: "Issue 191 T3: integration tests + verification lanes + impl record inputs"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
  implemented-by:
    - "docs/impl/ir/issue-191-pcm-wav-encode-mux-backend-with-mediasink-stagedoutput.md"
  depends-on:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
    - "docs/tasks/task-191-t2-wav-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-191-pcm-wav-output-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 191 T3: integration tests + verification lanes + impl record inputs

## Work

Write the integration test matrix per `docs/test-specs/issue-191-pcm-wav-output-backend.md`: round-trip write→reopen/decode via `ruviform-io-symphonia` (44.1k/48k/96k × mono/stereo/5.1 × S16/S24/F32); edge cases (0-frame, channel mismatch, NaN/Inf, clip, oversized); drain-vs-finalize and write-error distinctions; staged publish E2E (same-name dest, abort, simulated disk-full, replace/fail-if-exists); capability rejection. Then run the full verification lane set and merge `origin/main` before final gates.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-191-pcm-wav-output-backend.md`
- `docs/test-specs/issue-191-pcm-wav-output-backend.md`
- `docs/specs/issue-191-pcm-wav-output-backend.md`

## Dependencies

- `docs/tasks/task-191-t2-wav-backend.md`

## Affected Files

- `crates/ruviform-io-wav/tests/{roundtrip,errors,staged}.rs` (new)
- `crates/ruviform-io-wav/Cargo.toml` (dev-deps)

## Done When

- [x] All AC1–AC6 examples pass as tests
- [x] `mise run fmt-check`/`check`/`lint`/`test`/`verify-ci` green
- [x] `cargo test --workspace --locked` green
- [x] `git merge origin/main` done, `Cargo.lock` consistent, no offline-driver regression

## Verification

- [x] Full `mise run verify-ci` + `cargo test --workspace --locked` output captured for impl record
