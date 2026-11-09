---
id: "PLAN-034UwjBloBOcV5J6YJcNw9"
type: "plan"
status: "approved"
title: "Issue #309 PROBE-TIME slice 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-309-probe-time-align-fixture.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-309-probe-time-align-fixture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-309-probe-time-context-and-alignment.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-309-probe-time-align-fixture.md"
    - "docs/designs/issue-309-probe-time-align-fixture.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-309-probe-time-align-fixture.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #309 PROBE-TIME slice 実装計画 Implementation Plan

## Goal

実 `RtRuntime` + 実遅延 DSP + 実 audio fixture で、複数 Probe の
時間アライメント契約（offset/interval/Peak/RMS/gaps・
Incomparable/Conditional・loss mask 区別）を検証する統合試験を
`crates/ruviform-audio/tests/` に追加する。

## Scope

単一実装ストリーム・1 plan・1 task。変更は
`crates/ruviform-audio/tests/typed_probe_time_align.rs`（新規）と
本 doc チェーン・index に限定。`src/` の公開 API・実装は変更
しない（fixture が契約破綻を検出した場合のみ最小修正を別途
判断）。

## Upstream Documents

- `docs/specs/issue-309-probe-time-align-fixture.md`
- `docs/designs/issue-309-probe-time-align-fixture.md`
- `docs/test-specs/issue-309-probe-time-align-fixture.md`
- `docs/specs/issue-309-probe-time-frame-clock-epoch-alignment.md`
- `docs/designs/issue-309-probe-time-context-and-alignment.md`
- `docs/plans/issue-309-probe-time-context-and-alignment.md`

## Design Inputs

- `docs/designs/issue-309-probe-time-align-fixture.md`（D1–D5:
  実遅延 DSP・test 内 span 再構成・epoch/generation 代表機構・
  実経路 loss 誘発・F64Native 手組み span）

## File Map

| file | 変更 | 内容 |
| --- | --- | --- |
| `crates/ruviform-audio/tests/typed_probe_time_align.rs` | add | DelayDsp・fixture graph・drain→span/coverage helper・期待値計算・検証ケース |
| `docs/specs/issue-309-probe-time-align-fixture.md` | add | slice spec |
| `docs/designs/issue-309-probe-time-align-fixture.md` | add | slice design |
| `docs/test-specs/issue-309-probe-time-align-fixture.md` | add | 検証契約 |
| `docs/plans/issue-309-probe-time-align-fixture.md` | add | 本計画 |
| `docs/tasks/issue-309-f1-align-fixture.md` | add | 実装 task |
| `docs/impl/ir/issue-309-probe-time-align-fixture.md` | add | 実装 record（完了時） |
| `docs/{specs,designs,test-specs,plans,tasks,impl/ir}/README.md` | mod | index 更新 |

## Task Graph

- `issue-309-f1-align-fixture`（唯一の task・直列）

## Risks

- `DelayDsp` の delay line 実装ミス → PCM 不一致として顕在化
  （fixture の検出目的どおり）。独立期待値 + bit-exact 照合で
  切り分ける。
- channel 容量・pool 設定ミスによる意図しない loss 混入 →
  loss 非対象試験は既定容量 + 毎 block drain で決定的にする。
- doc audit の relation 規約違反 → scaffold script + 先行
  slice の front matter を雛形にし、`doc_status.js audit` で
  確認する。

## Verification

- `mise run fmt-check` / `mise run check` / `mise run lint` /
  `mise run test` / `mise run test-rt` / `mise run test-msrv`
  （存在するもの全て）。
- `cargo test -p ruviform-audio --test typed_probe_time_align`
  は `mise run test` に内包（補助証拠として直接実行も可）。
- `git merge origin/main` 後に再検証。

## Rollback

- test file と doc chain の追加のみのため、revert で完全に戻る。
  `src/` 変更が発生した場合はその差分を別 commit に分離する。
