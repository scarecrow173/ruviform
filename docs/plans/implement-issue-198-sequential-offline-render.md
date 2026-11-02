---
id: "PLAN-TH7W8rd4PbQx5UHmtKkzs0"
type: "plan"
status: "approved"
title: "Issue #198 sequential offline render implementation plan"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "docs/designs/issue-198-sequential-offline-render.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/designs/issue-198-sequential-offline-render.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-198-sequential-offline-render.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-198-sequential-offline-render.md"
  references: []
  defers: []
  deferred-by: []
---

# Plan — Issue #198 sequential offline render

## Tasks

- `task-198-t1-render-core` — `OfflineRenderSupport` + `GraphProcessor::offline_support` + `PreparedRender` + `Progress::bump_by`; deps wiring in `Cargo.toml`s.
- `task-198-t2-driver-output` — `RenderTransport` staging/preflight, sequential drive loop, emit algebra, `RenderOutput` lifecycle, `RenderError`/`RenderReceipt`.
- `task-198-t3-tests-verification` — OF-01..OF-07 integration tests (`tests/typed_render.rs` + `tests/common` fixtures), mise verification, merge `origin/main`, commit, impl record.

## Dependency order

T1 → T2 → T3 (same crate, strictly sequential).

## Files

- `crates/ruviform-audio/src/render.rs` (new, `pub mod render`)
- `crates/ruviform-audio/src/lib.rs` (export)
- `crates/ruviform-audio/src/typed/runtime.rs` (`OfflineRenderSupport` lives in render.rs; trait method added here)
- `crates/ruviform-audio/Cargo.toml` (+ `ruviform-media-io`, `ruviform-exec`; dev: `ruviform-io-wav`, `ruviform-io-symphonia`, `ruviform-media-reader`, `tempfile`-adjacent helper if needed)
- `crates/ruviform-exec/src/offline.rs` or `progress` site — `Progress::bump_by`
- `crates/ruviform-audio/tests/typed_render.rs`, `tests/common/mod.rs` (fixture additions: `OnePoleDsp`, `DelayDsp`, `FailSink`, position-ramp feeder)

## Verification gates

- `cargo test -p ruviform-audio --locked` and `-p ruviform-exec --locked` (recorded per OF-07 note)
- `mise run fmt-check`, `check`, `lint`, `test`, `test-rt`, `verify-ci`
- `git fetch origin main && git merge origin/main`, `cargo generate-lockfile`, re-verify before commit.
