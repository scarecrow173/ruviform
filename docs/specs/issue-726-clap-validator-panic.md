---
id: "SPEC-726clapValidatorPanic02"
type: "spec"
status: "approved"
title: "Issue #726 clap-validator 0.4.1 param-fuzz panic — 根因判断と leg 扱い確定"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/726"
    - "docs/discovery/issue-726-clap-validator-panic.md"
    - "docs/specs/issue-630-abi-validator.md"
    - "evidence/rf726/investigation.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-726-clap-validator-panic.md"
  implements:
    - "docs/discovery/issue-726-clap-validator-panic.md"
  implemented-by:
    - "docs/designs/issue-726-clap-validator-panic.md"
  depends-on:
    - "docs/specs/issue-630-abi-validator.md"
  blocks: []
---

# Issue #726 clap-validator param-fuzz panic — spec

## 背景

rf#630 で導入した opt-in ABI-validator lane（`ci/plugin-abi-validator/`）
の CLAP fixture suite で、clap-validator 0.4.1 が `test_fault` plugin の
param-mutating test 群を実行中に panic し、exit 101・JSON 未出力で
終了する事象が rf#630/rf#732 の evidence に記録されていた。
Issue #726 はこの panic の最小再現特定・根因所在の判断・upstream 報告
要否の決定・leg 扱いと無フィルタ run 再開条件の明文化を求める
調査・判断系 Issue である。

## 目的

panic の再現最小ケースを特定し、fixture・validator・runner のどこに
根因があるかを実測で確定する。upstream 報告の要否を判断し、
報告する場合は報告文ドラフトを残す。影響を受ける leg の扱い
（Unsupported 継続・filter・pin 変更待ち）と全 plugin suite 無フィルタ
run の再開条件を明文化する。runner の fail-closed 契約に違反があれば
`ci/plugin-abi-validator/` 内で是正する。

## 受入条件

- AC1: panic の再現最小ケースを特定するか、特定不能の理由を記録する
- AC2: upstream Issue 化の要否を判断し、結論と根拠を記録する
  （報告する場合は報告文ドラフトを残し、起票自体は Coordinator へ hand-off）
- AC3: 報告しない/報告するいずれの場合も、該当 leg の扱いを決定し
  runner/evidence に反映する
- AC4: 全 plugin suite 無フィルタ run の再開条件または NotRun 継続の
  根拠を記録する

## スコープ

- 対象: `ci/plugin-abi-validator/`（runner + contract test — 変更が
  必要な場合のみ）、`evidence/rf726/`（新規）、本 docs chain。
- read-only 参照: `crates/ruviform-plugin-test-clap/src/fault.rs`、
  upstream `free-audio/clap-validator` source、`evidence/rf630/`・
  `evidence/rf732/`（baseline）。
- 範囲外: fixture crate 本体、upstream tool の fork/patch/実起票、
  VST3 validator lane、docs index、`.github/**`、consumer リポジトリ、
  CI lane 追加。

## 非目的

- clap-validator 自体の修正（upstream の責務。本 slice は報告文
  ドラフトまで）。
- `test_fault` fixture の fault param 設計変更（rf#630 占有・設計どおり）。
- preset-discovery timeout・param-set-* failed など panic 以外の
  0.4.1 既知事象の解消（upstream release 待ち・別件として記録のみ）。

## 判定語彙

Pass / Fail / NotRun / Unsupported（POLICIES.md）。外部 validator の
異常終了を Pass と記録しない。panic を再現できなかった条件は推測で
埋めず NotRun/Unsupported で正直に記録する。
