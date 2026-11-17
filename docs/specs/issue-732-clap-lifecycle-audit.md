---
id: "SPEC-732clapLifecycleAudit02"
type: "spec"
status: "approved"
title: "Issue #732 production CLAP adapter lifecycle 契約監査と同型欠陥修正"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/732"
    - "docs/discovery/issue-732-clap-lifecycle-audit.md"
    - "docs/specs/issue-630-abi-validator.md"
    - "evidence/rf722/lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-732-clap-lifecycle-audit.md"
  implements:
    - "docs/discovery/issue-732-clap-lifecycle-audit.md"
  implemented-by:
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
  blocks: []
---

# Issue #732 production CLAP adapter lifecycle 契約監査 — spec

## 背景

Issue #722 で production VST3 adapter に lifecycle 契約違反 2 件
（terminate 非終端化・setupProcessing 受理過狭）を発見・修正した。
production CLAP adapter（`crates/ruviform-plugin-clap`）は同じ共有
中立 Lifecycle 機械（`ruviform-plugin-core`）の上に構築されているが、
CLAP 固有の契約面（instance 生存期間・audio-ports extension・
process status・reset 意味論）は未監査である。

## 目的

production CLAP adapter の lifecycle・port/buffer・status 写像経路を
CLAP 契約（`clap/plugin.h`）と clack 0.2.0 binding の生存期間型に
照合し、rf#722 の発見パターンに対応する欠陥を修正するか、欠陥なしを
証拠付きで確認する。

## 受入条件

- AC1: lifecycle 経路（init/activate/process/deactivate/destroy・
  reset・latency/tail 報告）を CLAP 契約と照合し判定を evidence に記録
- AC2: 発見欠陥は修正 + TDD red-green。欠陥なしなら監査範囲と判定
  根拠を evidence に記録
- AC3: headless/virtual-first で検証可能な範囲を mise 正規タスクで
  実行し結果を記録（native 依存は NotRun/Unsupported 正直記録）
- AC4: docs chain（discovery/spec/design/plan/task/test-spec/impl）を
  整備

## スコープ

- 対象: `crates/ruviform-plugin-clap/` 全体、`ci/plugin-abi-validator/`
  （production CLAP leg 拡張が必要な場合）、`evidence/rf732/`、
  本 docs chain。
- 条件付き lease: `crates/ruviform-plugin-core/` は共有 Lifecycle 機械
  自体の欠陥が確定した場合のみ変更可（変更時は PR 本文に理由明記）。
- 範囲外: `ruviform-plugin-test-clap`（rf#630 占有）・
  `ruviform-plugin-vst3`/`-test-vst3`・`ruviform-plugin-dock-test`・
  docs index・`.github/**`・CI lane 追加・`host.rs` GUI bridge。

## 非目的

- CLAP adapter への新機能追加（latency/tail extension の新規実装、
  state extension 等）。
- clap-validator 0.4.1 の param-fuzz panic（rf#726 が upstream 判断
  を担当）の修正 — leg 単位の正直 verdict で切り離す。
- fixture suite（`ruviform-plugin-test-clap`）の変更。

## 判定語彙

Pass / Fail / NotRun / Unsupported（POLICIES.md）。外部 validator が
未 provision・artifact 未 build の場合は NotRun であり Pass としない。
