---
id: "TASK-034JglHTIvjo18tMdbO3DP"
type: "task"
status: "todo"
title: "採否 ADR 確定とワークスペース帳簿・索引の同期"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/adr/select-audio-device-backend.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "docs/designs/overview.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "run-device-poc-examples-and-exp-log.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../designs/audio-device-boundary-and-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
---

# 採否 ADR 確定とワークスペース帳簿・索引の同期

## Work

plan `PLAN-034JgkWCimxdvr81k6b0ow` の Phase D。設計正本 `DESIGN-034JcC9QK924kDbu4WhibC`「ADR 確定計画」節の対応表どおり、PoC 証跡を ADR の確認項目へ対応付けて採否を確定し、クレート追加に伴う帳簿を同期して総合検証を行う。

- [ ] `docs/adr/select-audio-device-backend.md`（`ADR-034Jc9RslnH7QSQ1nZEEHL`・proposed）の `Confirmation`/`Verification` 項目を、設計の対応表どおり証跡へマッピングして確認する: (a) audio-core 公開 API に cpal 由来の型・エラー・feature 名が無い（`cargo doc` レビュー＋`#[non_exhaustive]` lint ゲート）、(b) audio-device 無効構成でコアがビルド可・cpal が adapter のみ（`cargo metadata`＋`cargo check -p ruviform-audio-core`）、(c) 列挙→既定構成→出力開始→コールバック到達の実証（`play_sine` 実験ログ）、(d) コールバック内非 alloc の検証（assert_no_alloc 試行結果 or コードレビュー＋静的 grep 証跡）、(e) 第 2 backend を追加可能な trait 形状（モック結合テスト＋trait 分解の設計レビュー）、(f) `mise run` 系パス
- [ ] 全項目が確認できたら ADR を `status: proposed → accepted` へ昇格し、`updated`・`Confirmation` 節のチェック・証跡への参照（実験ログ・テスト）を記録する
- [ ] 再評価トリガ（duplex 要件化・cpal メンテナンス再停滞・致命的挙動不良）が実験ログで観測された場合は昇格を保留し、保留判断と条件見直し版 ADR の起票方針を記録する（昇格しない場合は本タスクの結果として明示）
- [ ] `AGENTS.md` のワークスペース構成記述に `crates/ruviform-audio-device` を追記する（adapter の責務・依存方向・cpal 閉じ込めを含む記述と整合させる）
- [ ] ルート `Cargo.toml` の `members` 記述（コメント含む）と `docs/designs/overview.md` の索引を必要に応じ同期する
- [ ] spec `SPEC-034Jc9QEqYLVfg1CXXTiPV` の Acceptance Criteria 全件が充足されるか対応表を確認し、結果を記録する（spec status の扱いはレビュー方針に従う）
- [ ] 総合検証を実行し結果を記録する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-audio-device-port-and-cpal-poc.md](../plans/implement-audio-device-port-and-cpal-poc.md) — `PLAN-034JgkWCimxdvr81k6b0ow`（approved）。Phase D の正本
- [../designs/audio-device-boundary-and-poc.md](../designs/audio-device-boundary-and-poc.md) — `DESIGN-034JcC9QK924kDbu4WhibC`（approved）。ADR 確定計画（確認項目↔証跡対応表）と帳簿同期要件の正本
- [../adr/select-audio-device-backend.md](../adr/select-audio-device-backend.md) — `ADR-034Jc9RslnH7QSQ1nZEEHL`（proposed）。確定対象
- [../specs/audio-device-backend-evaluation-poc.md](../specs/audio-device-backend-evaluation-poc.md) — `SPEC-034Jc9QEqYLVfg1CXXTiPV`。受け入れ基準の正本

## Dependencies

- 前提タスク: [run-device-poc-examples-and-exp-log.md](run-device-poc-examples-and-exp-log.md)（`TASK-034JglHs8cNMDOvyZFeUpc`）— ADR 昇格判定の証跡となる実験ログを提供。推移的に [implement-audio-core-device-port.md](implement-audio-core-device-port.md)・[implement-audio-device-cpal-adapter.md](implement-audio-device-cpal-adapter.md) の成果物（非露出確認・モックテスト・`rt_` テスト証跡）にも依存
- 本タスクは plan の最終タスクであり後続をブロックしない

## Affected Files

- `docs/adr/select-audio-device-backend.md` — status 昇格または保留判断の記録
- `AGENTS.md` — ワークスペース構成記述へ ruviform-audio-device 追記
- `Cargo.toml`（root） — `members` 記述の確認・同期
- `docs/designs/overview.md` — 必要に応じ索引同期

## Done When

- [ ] ADR が `accepted` へ昇格した（または再評価トリガ観測による保留判断と条件見直し方針が記録された）
- [ ] ADR `Verification` 全項目に証跡が対応付けられている
- [ ] `AGENTS.md`・ルート `Cargo.toml`・必要なら `docs/designs/overview.md` が新クレートを反映している
- [ ] spec の Acceptance Criteria 全件への対応が確認・記録されている
- [ ] 総合検証がパスしている

## Verification

- [ ] `mise run fmt-check` exits 0
- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0
- [ ] `mise run test` exits 0
- [ ] `mise run test-rt` exits 0
- [ ] `mise run bench` exits 0
- [ ] `mise exec -- prek run --all-files` exits 0
- [ ] `mise exec -- cargo metadata --format-version 1` で cpal が `ruviform-audio-device` の `[dependencies]` にのみ存在することを確認
- [ ] `mise exec -- cargo doc -p ruviform-audio-core` 出力に cpal 由来の型・エラー・feature 名が存在しないことを確認
- [ ] `grep -n "status:" docs/adr/select-audio-device-backend.md` が `accepted`（または保留判断記録付き）を示す
