---
id: "TSPEC-630consumerOracle01"
type: "test-spec"
status: "approved"
title: "Issue #630 AC7 consumer-oracle slice — test specification"
created: "2026-11-21"
updated: "2026-11-25"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-consumer-oracle.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-630-consumer-oracle.md"
  implements:
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-consumer-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
  related:
    - "docs/discovery/issue-630-consumer-oracle.md"
    - "evidence/rf630/consumer-oracle-verification.md"
    - "evidence/rf630/consumer-oracle/manifest.json"
  verifies:
    - "docs/specs/issue-630-consumer-oracle.md"
    - "docs/designs/issue-630-consumer-oracle.md"
---

# Issue #630 AC7 consumer-oracle slice — test specification

docs+evidence 完結の slice のため、検証はコード実走ではなく
「manifest・README・対応表・evidence が再現可能な手順で整合
すること」の確認に絞る。全チェックは headless で完結する。

## Feature: manifest の正本表としての成立

- **Rule**: `evidence/rf630/consumer-oracle/manifest.json` は
  `schema` フィールドに `rf630-fixture-contract/1` を持つ JSON
  として parse できる。
  **Example**: `python -c "import json; json.load(open(...))"`
  が成功する。
- **Rule**: 全 7 fixture class（`test_gain`/`test_fault`/
  `test_editor`/`test_stateful`/`test_stateful_single`/
  `test_instrument`/`test_spawn_fail`）が `fixtureClasses` に
  存在し、それぞれ descriptor identity と少なくとも 1 つの
  contract face pin を持つ。
  **Example**: `test_instrument` は `midiTransport`・`voiceLaw`・
  `transportClick` の pin を持つ。
- **Rule**: `contractFaces` の各 `pin` は `#<section anchor>` 形式
  で、pin 先節が対象 crate README に実在する。
  **Example**: `#test_fault-semantics` → 両 README の
  `## `test_fault` semantics` 節が存在。
- **Rule**: `authoritativeContract.sources[].readmeSha256` が
  freeze 時点の README ファイル内容と一致する。
  **Example**: `sha256sum crates/ruviform-plugin-test-vst3/README.md`
  の出力が manifest の `readmeSha256` と一致。

## Feature: consumer oracle 対応表の網羅

- **Rule**: `consumerOracleMap` は `scarecrow173/synvelis#101`・
  `#127`・`#128`・`#176`・`scarecrow173/audio-lens#127` の
  5 行を持つ。
- **Rule**: 各 synvelis 行の `oracleAnchors` は path + symbol +
  測定時 repo SHA を持ち、al#127 行は anchor を持たず
  `NotApplicable` を記録する。
  **Example**: sv#101 行の anchor は
  `crates/synvelis-player/src/binding.rs` の
  `PluginBindingSession`/`check_frame_base`。
- **Rule**: `contractShape.disposition` は
  `headless-contract-match` か `external-owner` のいずれかで、
  consumer 実走 legs は `consumerRun.disposition =
  external-owner` のまま残る。
  **Example**: sv#176 行は contractShape が
  `headless-contract-match`、consumerRun が external-owner
  （sv#176 + ruviform#631）。
- **Rule**: al#127 行は `requirementState = NotApplicable` で、
  Issue 本文の NotApplicable 宣言を引用して記録する（実走 leg
  を捏造しない）。

## Feature: dep 非汚染の実測

- **Rule**: audio-lens `Cargo.lock` の fixture crate 名 grep を
  repo ごとに実行し、測定時点の含有/非含有を記録する。2026-11-25
  時点の観測では `ruviform-plugin-test-{vst3,clap}`・
  `ruviform-plugin-dock-test` が非含有だった。
  **Example**: `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test"
  ../audio-lens/Cargo.lock` は 2026-11-25 時点で exit 1
  （no match）。
- **Rule**: synvelis `Cargo.lock` の fixture crate 名 grep は
  audio-lens と分けて実行し、match の有無だけで violation を
  判定しない。match がある場合は依存経路を確認する。2026-11-25
  時点の観測では fixture crate 名を含有したが、経路は
  `synvelis-app` の dev-dep だった。
  **Example**: `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test"
  ../synvelis/Cargo.lock` は 2026-11-25 時点で exit 0。
- **Rule**: synvelis production DAG には fixture crate が非含有
  であることを `crates/synvelis-app/Cargo.toml` の section 単位で
  確認する。dev-dep 経路の自主採用は production DAG 汚染や
  ruviform による強制ではない。
  **Example**: `sed -n '/^\[dependencies\]/,/^\[/p'
  ../synvelis/crates/synvelis-app/Cargo.toml | grep -nE
  "^(ruviform-plugin-test|ruviform-plugin-dock-test)"` は exit 1
  （production `[dependencies]` 非含有）、`sed -n
  '/^\[dev-dependencies\]/,/^\[/p'
  ../synvelis/crates/synvelis-app/Cargo.toml | grep -nE
  "^(ruviform-plugin-test|ruviform-plugin-dock-test)"` は exit 0
  （dev-dep 経路含有）— 2026-11-25 時点。依存行に anchor して、
  コメント中の fixture 名を含有と誤認しない。
- **Rule**: 実行コマンド・両 repo SHA・exit code・観測結果を
  evidence に記録し、測定時点の事実として固定する。combined grep
  はどちらか一方の repo で match すれば exit 0 になるため、
  repo ごとの結論には使わない。

## Feature: 衛生チェックと lease 境界

- **Rule**: `mise run fmt-check` が exit 0（`.rs` 非回帰の確認
  として実行 — 本 slice は `.rs` を触らないため非破壊確認の
  意味を持つ）。
- **Rule**: `mise exec -- prek run --all-files` が exit 0
  （markdown/構成ファイル衛生チェックの canonical）。
- **Rule**: 禁止パスに差分がない。
  **Example**: `git diff` で `*.rs`・`tests/`・`mise.toml`・
  `docs/**/README.md`・`Cargo.toml`・`Cargo.lock`・`.github/**`・
  他リポジトリ（`../synvelis/**`・`../audio-lens/**`）の差分が
  空。

## Lane

- `mise run fmt-check`（正規入口・非破壊確認）
- `mise exec -- prek run --all-files`（markdown/構成ファイル
  衛生の canonical — `AGENTS.md` の手順）
- 上記以外の cargo/`*-full`/3OS lane は本 slice の必須検証に
  含めない（POLICIES §5: 文書・構成のみの変更は軽量チェックが
  原則。Daily Full CI・手動 Full CI に委譲）
- evidence 用 grep 実測は素のコマンドで行い lane 化しない
  （POLICIES: 重複 lane 増設の禁止）
