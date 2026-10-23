---
id: "IMPL-034L7tPv92QOGFgOtTLZNN"
type: "impl"
status: "completed"
title: "Issue #98 core video contract — Task 1 implementation"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plain_param.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/tests/video_contract.rs"
      - type: "file"
        path: "docs/impl/ir/issue-98-core-video-contract-task-1-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/descriptor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-core/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/tasks/ofx-core-contract-implementation.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-98-ofx-host-adapter.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ofx-core-contract-implementation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #98 core video contract — Task 1 implementation

## Summary

Issue #98 Task 1 — `ruviform-plugin-core` の中立 video 契約を TDD で実装:
借用記述ビュー `PluginDescriptorView<'a>` + `PluginDescriptor.view()`、
typed plain 制御 `PlainParamValue`/`PlainParamInfo<'a>`、`VideoProcessConfig`、
NONRT `VideoPluginInstance` trait（14 メソッド）、`plugin-core→video-core` 依存、
`mise.toml` 2 task、contract tests。`param.rs`・audio `PluginProcessor`・
event/wire/normalizer は不変（C1 defer どおり）。

## Context

approved plan `docs/plans/implement-issue-98-ofx-host-adapter.md` の Task 1
（`docs/tasks/ofx-core-contract-implementation.md`・actions 1–4）に従う。
upstream: approved spec/ADR/design（ofx-host-adapter 系）・core TSpec
`video-plugin-plain-contract-tests.md`。native adapter は Task 2・未着手。

## Implementation

### Changes

- `descriptor.rs`: `PluginDescriptorView<'a>`（`&'a str`/`&'a [&'a str]`）+ `view()` pure borrow・positive doctest + owner-escape `compile_fail` doctest（E0515 系）
- `plain_param.rs`（新規）: `PlainParamValue`（`#[non_exhaustive]` Copy 4 variant・raw NaN 構築可）・`PlainParamInfo<'a>`（借用 metadata・hard/display domain 分離・Choice contract を rustdoc に明記）
- `video.rs`（新規）: `VideoProcessConfig`（private fields・`new()` 非ゼロ dims のみ検証・Copy getters・非 RGBA 受理）・`VideoPluginInstance`（`Error: std::error::Error + 'static`・全操作 NONRT・trait doc に owner-escape `compile_fail`）
- `lib.rs`: 新 leaf module（private `mod` + flat `pub use`・既存 style）・crate doc に video port/plain channel/借用 view を追記
- `lifecycle.rs`: `Processing` variant doc を "RT processing" → "processing activity（RT 保証ではない）"（FSM コード不変）+ module doc note
- `Cargo.toml`: `ruviform-video-core` 依存追加・`crates/ruviform-plugin-core/README.md` 許可依存 ledger（audioCore・videoCore・mediaCore）・crate duties に video port/plain channel/view 追記・baseline DAG の plugin-core 行を実依存どおりに更新
- `mise.toml`: `test-plugin-core`/`lint-plugin-core`（plan 確定 TOML どおり）
- `tests/video_contract.rs`: view/static+owned borrow、Plain 値 Copy/double NaN 構築、metadata Double+Choice、config 非ゼロ dims + Yuv420p 受理、generic consumer（core 型のみ）

### Decisions During Implementation

- 実装 stack: implementation-flow（incremental-implementation→test-driven-development）+ impl-doc。cost_optimization 非適用（公開 API/lifetime 契約は boilerplate ではない）。レビューは requesting-code-review/review-protocol で parent 委譲。
- normalized `ParamValue` 経路・追加 normalizer・width 計算 helper は作らない — plain チャネルは typed 生値の独立経路（design 決定どおり）。
- `PlainParamValue::Double` は raw 型として NaN 構築可能のまま — finite 強制は setter/境界側の責務。
- `VideoPluginInstance` は `mod` private + flat `pub use` で既存 export style に合わせた（lead 選択）。

## Related Experiments

### Adopted

なし — frozen plan の素直な実装で新規試行は不要だった。

### Rejected

なし。

## Validation

- A2 red（実 API 前）: `mise run test-plugin-core` exit 101 — `E0432` 未解決 import（新公開 API 不在）→ `t1-red.log`。これは contract 欠如の正当な red（missing file/command/env ではない）
- A4 green（実装後）: `mise run test-plugin-core` exit 0 → `t1-green.log`・`mise run lint-plugin-core` exit 0 → `t1-lint.log`（must_use/unused import/assert_is_empty/useless_vec の 4 件を修正後）
- rework 後 fresh 実行: `t1-fix-green.log`・`t1-fix-lint.log` — 実値 **32 unit + 8 integration + 21 doctest + 2 compile_fail（計 63）**・lint exit 0
- 最終 fresh 実行（本 rework 後）: `t1-final-green.log`・`t1-final-lint.log` — 実値 **32 unit + 8 integration + 21 doctest + 2 compile_fail**・lint exit 0
- historical original green: **32 unit + 6 integration + 20 doctest（全 pass・`t1-green.log`）** — 初回緑の必須証拠として保存
- borrow-escape 検証: rustc probe で `compile_fail` snippet を real crate 対象に再現し、実 diagnostic code を確認済み（`probe-*.log`）
- 未実施: native adapter・fixture・MSRV/Windows 実機証明・full workspace gate（Task 2 以降）

## Risks

- `PlainParamValue::Double` は NaN 構築可能な raw 型 — setter 側検証が未実装なら downstream が NaN を通し得る（setter 境界義務は doc 明記済み・adapter 側 Task 3 で検証）
- 借用 view の lifetime 誤用は `compile_fail` doctest + real borrow error で保護（green 後は unresolved symbol ではない真の borrow 診断を確認）

## Review History

- Cycle 1: K1-K6 六 roots を Validator 指摘として修正（K1=borrow doctest/probe・K2=Yuv test・K3=metadata split・K4=lifecycle doc・K5=lib.rs deps/README/baseline・K6=IR canonical sections）
- Cycle 2: targeted Validator（`scratch/core-reverify.md`）+ independent Judge（`scratch/core-judge-final.md`）— **ACCEPT**・scoped Core K1-K6 全 CONFIRMED-FIXED
- negative proof: `probe-view.log`/`probe-trait.log` で E0515 実 borrow error（exit 1・unresolved symbol ではない）
- final 検証: `t1-final-green.log`（32 unit + 8 integration + 21 doctest + 2 compile_fail・exit 0）・`t1-final-lint.log`（exit 0）
- fresh IR audit: `ir-audit-current.json` + `ir-audit-core-scoped.json` — 本 IR findings = 0（他 IR の 13 findings は別 file・未修正の既存 finding として分離）

## Follow-ups

- Core scope review approved — 本 bookkeeping は buildable Core commit に同梱。native 未着手・次 run gate は commit 後
- Task 2（native OFX runtime）は未着手・Core commit 後の dispatch 待ち
