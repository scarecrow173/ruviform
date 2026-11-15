---
id: "PLAN-034YbbuRqN5VUqXuQfKKHB"
type: "plan"
status: "approved"
title: "Issue #629 WASAPI streaming verifier implementation plan"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-wasapi-endpoint.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
    - "docs/tasks/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 WASAPI streaming verifier implementation plan

## Goal

Issue #629 4th slice: WASAPI render→capture streaming verifier
（`tests/virtual_endpoint.rs`）の新設、`endpoint_probe` の pair
candidate/AC4 記録拡張、および AC3 Test Runtime 結線 IF・AC6 lane
設計の docs 記録を、headless 範囲で完了する。

## Scope

単一 implementation stream。verifier・probe 拡張・docs chain・
evidence は同一 PR で論理コミットに分けて納める。実 provisioning・
lane 登録・Test Runtime 実結線は後続 slice へ hand-off（spec
Out of Scope と同一境界）。

## Upstream Documents

- `docs/specs/issue-629-wasapi-endpoint.md`
- `docs/designs/issue-629-wasapi-endpoint.md`（approved）
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
- `POLICIES.md` / Issue `scarecrow173/ruviform#627`（Case schema・
  `environment_kind`）

## Design Inputs

- `docs/designs/issue-629-wasapi-endpoint.md` — verifier 手順・
  独立 oracle・pair stem 正規化・probe 拡張点・lane 設計記録。

## File Map

- `crates/ruviform-audio-device/tests/virtual_endpoint.rs` -
  `real-device` feature + `#[ignore]` の WASAPI streaming verifier
  （新設）。
- `crates/ruviform-audio-device/examples/endpoint_probe.rs` -
  pair candidate 表・driver 署名/権限/cleanup の best-effort 記録・
  verdict 文言更新。
- `docs/specs|designs|plans|tasks|test-specs/issue-629-wasapi-
  endpoint.md` - doc chain。
- `docs/impl/ir/issue-629-wasapi-endpoint.md` +
  `docs/impl/exp/issue-629-wasapi-endpoint.md`（+ `*-exp.jsonl`）-
  実装記録・実験記録。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` - 解消・
  確定した gap の更新。
- `evidence/rf629/` - probe/verifier の Windows 実機出力・検証
  記録。

## Implementation Sequence

- [ ] T1: `tests/virtual_endpoint.rs` 新設 — 列挙・pair 選定・
  format 交渉・open/start・signal render・capture 観測・oracle
  照合・lifecycle・verdict（fail-closed NotRun 含む）。
- [ ] T2: `endpoint_probe.rs` 拡張 — pair candidate 表・driver
  署名（PowerShell `Get-AuthenticodeSignature` best-effort）・
  microphone 権限（registry best-effort）・cleanup/verdict 文言。
- [ ] T3: 検証 — fmt/check/lint/test・`lint-audio-real-input`・
  `prek`・probe/verifier 実機実行・evidence 記録。
- [ ] T4: docs 仕上げ — impl ir/exp・discovery 更新・lane 設計・
  #628 結線 IF の最終確認・review-protocol。

## Dependencies and Risks

- `ruviform-audio-test` dev-dep は coordinator 不承認 — oracle は
  テスト内独立 generator + Expected/Observed で自足。結線 IF は
  docs 記録のみ。
- name/stem match は candidate であり virtual の証明ではない —
  verdict 文言で `virtual-os-endpoint` を主張しない。
- cable SRC（48kHz 内部）前提 — bit-exact 禁止、marker/level/
  spectral の tolerance 系。
- `StreamInvalidated` は非対話再現不能 — NotRun 記録（AC5/後続）。
- 新規 dependency/feature/`unsafe`/`Cargo.toml`/`mise.toml` 変更
  なし — `real-device` feature 流用。

## Task Breakdown

- T1 → T2 は独立（同 crate 内の別 file）だが同一 PR。T3 は T1/T2
  完了後。T4 は全検証後。

## Verification

- [ ] `mise run fmt-check` / `mise run check
  ruviform-audio-device` / `mise run lint
  ruviform-audio-device` / `mise run test
  ruviform-audio-device` — Pass。
- [ ] `mise run lint-audio-real-input`（`--all-targets --features
  real-device` の `-D warnings`）— Pass。
- [ ] `mise exec -- prek run --all-files` — Pass。
- [ ] `mise exec -- cargo run -p ruviform-audio-device --example
  endpoint_probe`（Windows 実機）exit 0 +
  `evidence/rf629/endpoint_probe-windows-local.txt` 更新。
- [ ] 補助証拠: `cargo test -p ruviform-audio-device --features
  real-device --test virtual_endpoint -- --ignored
  --test-threads=1` — 未 provision 環境では `verdict: NotRun` を
  記録（Pass 主張なし）。
- [ ] MSRV lane — NotRun（daily CI 管轄、タスク指定どおり未実行）。

## Open Questions / Gaps

- pair 複数時の選択ポリシー — 現状は全列挙 + 先頭から順次試行、
  provisioning 実測後に必要なら env var/引数化を検討。
- marker/level/spectral 閾値は provisional — provisioned runner
  初回実走の実測で締め直す。
- #628 `PcmFixture`/`oracle`/`RunMetadata` との実結線 — dev-dep
  承認待ちの後続 slice。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装）。
- Independent work streams: none（T1/T2 は同一 worker が逐次実施）。
- User approval required before dispatch: yes → review-protocol の
  blocker なしをもって承認とみなす（worker 契約）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- reviewer は spec AC1–AC6 の headless 範囲と fail-closed 境界
  （`NotRun`/`Unsupported` を Pass にしない・name match を virtual
  証明にしない・`virtual-os-endpoint` を主張しない）を重点確認。
  probe の PowerShell/registry best-effort が probe を落とさない
  こと、verifier の RT callback が allocation-free/bounded である
  ことも確認対象。
