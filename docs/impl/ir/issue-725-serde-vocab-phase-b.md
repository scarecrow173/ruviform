---
id: "IMPL-034Z7e3kdbNfHBXFrRomHP"
type: "impl"
status: "completed"
title: "Issue #725 serde 語彙 Phase B — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
    - "evidence/rf725/consumer-impact-assessment.md"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/specs/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/designs/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/plans/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/tasks/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/test-specs/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/adr/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/impl/ir/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-b.md"
      - type: "file"
        path: "docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl"
      - type: "file"
        path: "evidence/rf725/phase-b-verification.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-test/src/metadata.rs"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/metadata.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/Cargo.toml"
      - type: "file"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
      - type: "file"
        path: "docs/designs/issue-629-provisioning-runbook.md"
      - type: "file"
        path: "docs/specs/issue-629-virtual-endpoint-lane.md"
      - type: "file"
        path: "docs/specs/issue-629-provisioning-runbook.md"
      - type: "file"
        path: "docs/test-specs/issue-629-virtual-endpoint-lane.md"
  implements:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
  implemented-by:
    - "docs/impl/exp/issue-725-serde-vocab-phase-b.md"
  depends-on:
    - "docs/tasks/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
    - "docs/designs/issue-725-serde-vocab-phase-a.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-725-serde-vocab-phase-b.md"
    - "docs/adr/issue-725-serde-vocab-phase-b.md"
    - "evidence/rf725/consumer-impact-assessment.md"
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
    adopted: ["docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl"]
    rejected: []
---

# Issue #725 serde 語彙 Phase B — 実装記録

## Summary

`EnvironmentKind::RealAbiTestPlugin` の serde emit 側を Case
Schema v1 / consumer-verify 契約語彙 `test-plugin` に統一した
（Phase A design の第2 slice — atomic flip）。serde
`#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
と `as_str()` → `"test-plugin"` を同一 slice で flip し、
`ruviform-plugin-dock-test` の literal assert・コメント、
`ruviform-audio-test` の emit pin テストを更新。deserialize は
新旧両綴りを恒久的に受容し続け、historical evidence の
読み戻しを維持する。`ci/consumer-verify/**`・`.github/**`・
consumer repo・historical evidence・docs index は変更なし。

## Context

- Task: `docs/tasks/issue-725-serde-vocab-phase-b.md`（T1-T9）
- Plan: `docs/plans/issue-725-serde-vocab-phase-b.md`
- Design: `docs/designs/issue-725-serde-vocab-phase-b.md`（
  approved — flip 後の挙動表・atomic flip の根拠）
- ADR: `docs/adr/issue-725-serde-vocab-phase-b.md`（accepted —
  canonical 綴り統一の決定記録）
- 移行方針の正本: `docs/designs/issue-725-serde-vocab-phase-a.md`
  （approved — 移行期間中の挙動 :81-87・atomic flip :93-98・
  第2 slice 手順 :100-111）
- AC1 記録: `evidence/rf725/consumer-impact-assessment.md`
  （consumer 影響ゼロ・pin bump 不要）

## Implementation

### Changes

- `crates/ruviform-audio-test/src/metadata.rs`:
  `RealAbiTestPlugin` の serde attr を
  `#[serde(rename = "test-plugin", alias = "real-abi-test-plugin")]`
  へ切替（Phase A の `#[serde(alias = "test-plugin")]` から反転
  — canonical wire 綴りを schema 語彙に変更し旧綴りを alias で
  受容継続）。`as_str()` を `"real-abi-test-plugin"` →
  `"test-plugin"` に flip。enum doc（旧「Known vocabulary gap」
  記述）と variant doc を Phase B 完了状態に更新。型名
  `RealAbiTestPlugin`・variant 集合・`#[non_exhaustive]` は不変。
- `crates/ruviform-audio-test/tests/metadata.rs`:
  `environment_kind_wire_spellings_match_case_schema_v1` に
  `RealAbiTestPlugin` → `"test-plugin"` pin を追加（全 variant が
  schema 語彙に一致する状態へ）。
  `environment_kind_deserialize_accepts_both_vocab_spellings` の
  emit 期待値を `"test-plugin"` に flip（deserialize 両綴り受容・
  未知語彙拒否の assert は維持）。
  `run_metadata_from_json_accepts_test_plugin_spelling` の
  re-emit 期待値を `"test-plugin"` に flip し、旧綴り文書の
  deserialize 受容と canonical re-emit 正規化を pin。
  `environment_kind_serde_matches_as_str` は不変条件として
  変更なし。
- `crates/ruviform-plugin-dock-test/tests/plugin_dock.rs`:
  :168 `environment["environment_kind"]` assert と :211
  per-case `environment_kind` assert を `"test-plugin"` に更新
  （fail-closed degrade 設計 :137-149 は不変 — 綴りのみ）。
- `crates/ruviform-plugin-dock-test/Cargo.toml`: :77-79 依存
  コメントの語彙言及を `test-plugin` に更新（依存エッジ・
  feature・version は不変）。
- `crates/ruviform-plugin-dock-test/src/scenario.rs`:
  **変更なし** — :280 `json!(kind.as_str())` が `as_str()` flip
  に自動追従（lease 外・編集不要を確認）。
- `docs/designs|specs|test-specs/issue-629-*.md` 5 ファイル:
  Phase A が記録した `real-abi-test-plugin`↔`test-plugin` gap
  記述へ「解消済み」status を append-only で追記（flip 実装・
  検証後に実施 — 既存記述の書き換え・削除なし）。
- `evidence/rf725/phase-b-verification.md`: Phase B 検証記録
  （実施コマンド・exit code・実効カバレッジ・環境情報）。
- docs chain: `docs/{discovery,specs,designs,plans,tasks,
  test-specs,adr,impl/ir,impl/exp}/issue-725-serde-vocab-phase-b.md`
  + `docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl`。
- docs index（`docs/**/README.md`）は一切変更なし —
  Coordinator が統合時に再生成。

### Decisions During Implementation

- serde `rename`+`alias` の向き: canonical = `test-plugin`
  （schema 正本 `ci/consumer-verify/evidence.mjs`
  `ENVIRONMENT_KINDS` に収束）、`real-abi-test-plugin` は
  deserialize alias で恒久受容 — Phase A design :81-87 の
  右列の実現。旧綴りを含む historical evidence・flip 前文書の
  読み戻しを維持。
- `environment_kind_serde_matches_as_str` は変更せず不変条件
  として維持 — serde emit == `as_str()` を全 variant で強制する
  pin が flip の atomic 性を検証する側として機能。
- `run_metadata_from_json_accepts_test_plugin_spelling` に
  旧綴り文書の re-emit 正規化 pin を追加 — 旧綴り入力が
  canonical `test-plugin` に正規化されることを明示的に固定
  （TS-725B-4 の具体化）。
- fmt-check 初回失敗（rustfmt line-join 指摘 2 箇所）は
  整形のみの修正で対応 — 意味変更なし、修正後に test 再実行で
  確認。
- `docs/adr/issue-725-serde-vocab-phase-b.md` を作成 — emit flip
  の canonical 綴り統一決定を MADR として記録（Coordinator
  指示の docs chain 構成に含まれるため。移行方針自体は Phase A
  design が正本）。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-725-serde-vocab-phase-b-exp.jsonl` —
  本 slice の実装判断（atomic flip・実効カバレッジ確認・検証
  結果）を seq 1-5 で記録。
- `docs/impl/exp/issue-725-serde-vocab-phase-a-exp.jsonl` —
  Phase A の移行方針決定（deserialize alias・emit flip は第2
  slice）の根拠として参照。

### Rejected

- なし（新規の実験的分岐は発生しなかった — Phase A design の
  第2 slice 手順をそのまま実行）。

## Validation

実施環境: Windows（worktree
`G:/orca/workspaces/ruviform/rf725-serde-vocab-phase-b`）、
rustc 1.99.1。canonical = `mise run` 経由。

| コマンド | exit code | 結果 |
| --- | --- | --- |
| `mise run check ruviform-audio-test` | 0 | Passed |
| `mise run lint ruviform-audio-test` | 0 | Passed（clippy `-D warnings`） |
| `mise run test ruviform-audio-test` | 0 | Passed（metadata 13 tests — flip pin・両綴り受容・未知語彙拒否・round-trip 全 pass） |
| `mise run check ruviform-plugin-dock-test` | 0 | Passed |
| `mise run lint ruviform-plugin-dock-test` | 0 | Passed |
| `mise run test ruviform-plugin-dock-test` | 0 | Passed（`fixture_run_reports_honest_verdicts` 含む 4/4 — full assert 経路で `test-plugin` 綴りを実効検証） |
| `mise run test-ci-consumer-verify` | 0 | Passed（84/84 — schema 側非回帰） |
| `mise run fmt-check` | 0 | Passed（初回 2 箇所の line-join 指摘を修正後） |
| `mise exec -- prek run --all-files` | 0 | Passed |

実効カバレッジ: `plugin_dock.rs` の fail-closed degrade 設計
（:137-149）に対し、`ruviform_plugin_test_vst3.dll` が
`target/debug/deps/` に存在し `artifact::resolve_artifact` で
解決されるため、`fixture_run_reports_honest_verdicts` は
degrade 経路ではなく full assert 経路を実行 — :168,:211 の
`"test-plugin"` assert が実効的に検証された（degrade のみの
NotRun ではない）。

NotRun: `*-full`/`*-msrv`/3OS 横断・opt-in lane
（`test-ui-plugin-dock*` 系）の実走 — daily CI 委譲・運用判断。

## Risks

- `#[serde(rename)]` と `as_str()` の flip が非 atomic だと
  `environment_kind_serde_matches_as_str` が失敗する — 同一
  slice で実施し pin テストが検証（実施済み・pass）。
- plugin-dock-test literal assert の更新漏れ — :168,:211・
  Cargo.toml コメントを同 slice で更新済み。cdylib 解決環境で
  実効検証済み。
- 旧綴り受容の喪失 — `alias = "real-abi-test-plugin"` を恒久的に
  維持し `environment_kind_deserialize_accepts_both_vocab_
  spellings` で pin。
- consumer 影響 — `evidence/rf725/consumer-impact-assessment.md`
  で影響ゼロ・pin bump 不要と判定済み（本 slice で consumer
  repo は変更なし）。

## Follow-ups

- issue-629 docs 5 ファイルへの「解消済み」status 追記
  （append-only — flip 検証後に実施）。
- `evidence/rf725/phase-b-verification.md` の記録。
- review-protocol レビュー・blocker 解消。
- `git fetch origin` → `origin/main` merge → Conventional
  Commits でコミット → `git push -u origin
  feature/725-serde-vocab-phase-b` → `gh pr create`（日本語本文）。
- docs index（`docs/**/README.md`）は Coordinator が統合時に
  再生成 — 本 worker は編集・`index apply` 実行しない。
- Issue #725 の merge・close は Coordinator が行う。
