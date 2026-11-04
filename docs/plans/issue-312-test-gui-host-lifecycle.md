---
id: "PLAN-034S312gtUiTestPlnEE55"
type: "plan"
status: "approved"
title: "Issue #312 ruviform-ui-test implementation plan"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-312-test-gui-host-lifecycle.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-312-test-gui-host-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
  defers: []
  deferred-by: []
---

# Issue #312 ruviform-ui-test implementation plan

## Goal

Ship `ruviform-ui-test`: the RF-GT-03 test-only GUI host lifecycle + secure control adapter — `rvf.uitest.v1` ops over reused `rvf.ipc.v1` envelopes, `TestIo` port, `FakeHost`/`HeadlessIo` headless path, stdio+loopback-TCP transports, `TestDriver`, `dummy_app` reference consumer — with the full acceptance surface verified under `mise run test`. Zero production-crate changes.

## Scope

One plan, one new crate, three ordered slices plus closeout. All code lives under `crates/ruviform-ui-test`; registration edits are confined to `Cargo.toml` members + `crates/README.md` + `AGENTS.md` task-table line. T2 depends on T1 types; T3 depends on both.

## Upstream Documents

- `docs/specs/issue-312-test-gui-host-lifecycle.md` (approved; AC1–AC5)
- `docs/adr/issue-312-test-control-transport-security.md` (accepted)
- `docs/discovery/issue-312-test-gui-host-lifecycle.md` (resolved)

## Design Inputs

- `docs/designs/issue-312-test-gui-host-lifecycle.md` (approved) — module map, op table, `TestIo`/`TestApp` shapes, serve loop, fail-closed matrix, honest clock scope.

## File Map

- `crates/ruviform-ui-test/Cargo.toml` — package + deps (`ruviform-ui`, `serde`, `serde_json`) + bin target.
- `crates/ruviform-ui-test/src/lib.rs` — crate docs (contract sources, RT clause, isolation statement) + re-exports.
- `crates/ruviform-ui-test/src/proto.rs` — op names, codes, all serde contract types.
- `crates/ruviform-ui-test/src/journal.rs` — `Journal` + `JournalHandle`.
- `crates/ruviform-ui-test/src/fake.rs` — `FakeHost` + `FakeWindow`.
- `crates/ruviform-ui-test/src/fixture.rs` — `FixtureController`, `TestApp` traits.
- `crates/ruviform-ui-test/src/headless.rs` — `HeadlessIo` (`TestIo` impl).
- `crates/ruviform-ui-test/src/agent.rs` — `TestAgent` + `serve`/`serve_stdio`/`serve_tcp`/`from_env`.
- `crates/ruviform-ui-test/src/transport.rs` — frame codec + env interpretation + secret gen.
- `crates/ruviform-ui-test/src/driver.rs` — `TestDriver`.
- `crates/ruviform-ui-test/src/bin/dummy_app.rs` — reference consumer.
- `crates/ruviform-ui-test/tests/*.rs` — unit + cross-process E2E.
- `crates/ruviform-ui-test/README.md` — crate contract + wiring guide.
- `Cargo.toml` members, `crates/README.md` row, `AGENTS.md` (crate count 29→30 only if the README/index tracks it — verify; else skip AGENTS.md).
- `docs/impl/ir/issue-312-*.md` — implementation record (T4).

## Dependency Graph

```text
T1 contract types + fake host + journal + headless io (proto/journal/fake/fixture/headless + lib skeleton + workspace registration)
  └─ T2 agent + transport + driver + dummy_app (agent/transport/driver + bin)
        └─ T3 tests + README + registration docs
              └─ T4 verification + impl record
```

## Task Granularity

- **T1 — contract + headless core**: crate skeleton, `proto` types, `Journal`/`JournalHandle`, `FakeHost`, `FixtureController`/`TestApp`, `HeadlessIo`, workspace member registration. Unit tests for journal/fake/headless behavior.
- **T2 — control plane**: `agent` (handle + serve + from_env + capability/auth gates), `transport` (frames/env/secret), `driver`, `dummy_app` bin.
- **T3 — tests + docs**: unit tests for agent gates (pre-auth, capability, malformed, shutdown, EOF cleanup), cross-process E2E (AC1 lifecycle, AC2 matrix, AC3 fail-closed incl. TCP), crate README, crates/README.md row.
- **T4 — verification + record**: `mise run fmt-check`/`check`/`lint`/`test` green; isolation check (`git diff` on production crates = empty); impl record with NotRun/Unsupported inventory for #313/#243.

## Risk Register

- `TestIo` shape too headless-specific for #313 windowed wiring — mitigated: trait consumes only `ruviform-ui` port types; `emit`/`create_window`/`request_shutdown` map 1:1 onto `WindowEvent`/`WindowHandle`; risk: residual mismatch → documented handoff note + additive trait methods allowed (non_exhaustive-style evolution on a v1 test crate).
- Frame codec divergence stdio↔TCP — mitigated: single `read_frame`/`write_frame` used by both `serve` paths and driver.
- FakeHost diverging from `run_multi` semantics — mitigated: port same invariants (bounded backlog, Destroyed via queue, default-approve post-emit is the app's job via close policy; FakeHost does not auto-approve — see tests).
- Secret generation weak — mitigated: RandomState+time+pid is adequate for loopback test threat model; declared in ADR; never reused across runs.
- `CARGO_BIN_EXE_dummy_app` unavailable — fallback: spawn via `cargo run` path; verify early in T3.

## Rollback Strategy

New crate is additive; rollback = `git revert` per task commit. No persistent formats, no feature flags, no production-crate edits to migrate back.

## Verification Matrix

| Spec AC | Test / check | Where |
| --- | --- | --- |
| AC1 lifecycle E2E | `tests/e2e_stdio.rs` spawn→hello→caps→fixture→observe→shutdown→exit | T3 |
| AC2 ownership matrix | `tests/e2e_stdio.rs` (dual window, veto, restart, ipc fault, EOF) + `tests/agent.rs` | T3 |
| AC3 fail-closed | `tests/agent.rs` (pre-auth/capability/malformed/disable) + `tests/e2e_tcp.rs` (secret/loopback) + `git diff` isolation | T3/T4 |
| AC4 non-test consumers | `mise run check`/`lint`/`test` workspace green + prod crates diff empty | T4 |
| AC5 NotRun reporting | impl record inventory | T4 |

## Self-Review

- File map covers every spec'd surface; each op has a test-spec rule.
- No production-crate edits anywhere — isolation holds by construction.
- Task order keeps same-file edits inside one task; each task independently revertible.
