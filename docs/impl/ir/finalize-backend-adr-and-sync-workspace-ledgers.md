---
id: "IMPL-034JC0drffSR2IoiSS2k4M"
type: "impl"
status: "completed"
title: "Finalize backend ADR and sync workspace ledgers"
created: "2026-10-21"
updated: "2026-10-21"
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
        path: "docs/designs/overview.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/finalize-backend-adr-and-sync-docs.md"
  implemented-by: []
  depends-on:
    - "run-device-poc-examples-and-record-experiment-log.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../plans/implement-audio-device-port-and-cpal-poc.md"
    - "../../designs/audio-device-boundary-and-poc.md"
    - "../../adr/select-audio-device-backend.md"
    - "../../specs/audio-device-backend-evaluation-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/audio-device-port-and-cpal-adapter.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl"
    rejected: []
---

# Finalize backend ADR and sync workspace ledgers

## Summary

task `TASK-034JglHTIvjo18tMdbO3DP`（plan `PLAN-034JgkWCimxdvr81k6b0ow` Phase D）として、採否 ADR `ADR-034Jc9RslnH7QSQ1nZEEHL` の `Verification` 全 6 項目を Phase A〜C の証跡へ対応付けて確認し、`proposed` → `accepted` へ昇格した。再評価トリガ（duplex 要件化・cpal メンテナンス再停滞・致命的挙動不良）はいずれも未観測。あわせて `AGENTS.md` ワークスペース構成記述への adapter 追記と `docs/designs/overview.md` 索引の同期を行い、spec `SPEC-034Jc9QEqYLVfg1CXXTiPV` の Acceptance Criteria 全件充足を確認・記録した。

## Context

- 実施タスク: [../../tasks/finalize-backend-adr-and-sync-docs.md](../../tasks/finalize-backend-adr-and-sync-docs.md)
- 上流: [plan](../../plans/implement-audio-device-port-and-cpal-poc.md) Phase D、[design](../../designs/audio-device-boundary-and-poc.md)「ADR 確定計画」節（確認項目↔証跡対応表）、[ADR](../../adr/select-audio-device-backend.md)（確定対象）、[spec](../../specs/audio-device-backend-evaluation-poc.md)（受け入れ基準）
- 前提: [run-device-poc-examples-and-record-experiment-log.md](run-device-poc-examples-and-record-experiment-log.md)（`IMPL-034JywI99DQ33dDEZVnnqC`）の実験ログが昇格判定の証跡

## Implementation

### Changes

- `docs/adr/select-audio-device-backend.md` — `status: accepted`、`updated: 2026-10-21` へ昇格。`Confirmation` 節を証跡対応（実験ログ・3 IR・assert_no_alloc テスト・モック結合テスト）＋再評価トリガ未観測＋未観測項目の引き継ぎ記録へ更新。`Verification` 全 6 項目を `[x]` + 証跡参照付きで確定
- `AGENTS.md` — ワークスペース構成節に `crates/ruviform-audio-device` の記述を追記（adapter 層の責務・依存方向・cpal 閉じ込め・公開面非露出・実機 example が CI 対象外である旨）
- `docs/designs/overview.md` — Detailed Design Documents 索引に `audio-device-boundary-and-poc.md` の行を追加
- ルート `Cargo.toml` — T2 で `members` + Adapters コメント追加済みのため変更不要を確認

### Decisions During Implementation

- **ADR 昇格（採用）を確定** — Verification 全項目に証跡が存在し、再評価トリガ 3 条件のいずれも実験ログで未観測。`Fixed(480)` 拒否は「致命的挙動不良」ではなく設計想定内の `UnsupportedConfig` 型表現（`Default` フォールバックで開通済み）と判定
- **spec の AC 対応表を IR に記録・spec status は据え置き** — task の changes 宣言が spec を含まないため、spec 文書自体は変更せず AC 充足の判定結果を本 IR に記録（下記 Validation）。spec status の遷移はレビュー方針に委ねる
- **未観測項目は ADR Confirmation に明記** — ALSA vs PipeWire・Send/Sync ばらつき・WASAPI exclusive/ASIO・初回 callback timestamp=0 を後続評価事項として ADR 本体に記録し、未観測を以て昇格を阻害しない判断根拠（Windows/WASAPI 実機での主要経路は実証済み）を残す

## Related Experiments

### Adopted

- [device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl](../exp/device-poc-real-machine-run-list-devices-and-play-sine-on-wasapi.jsonl) — ADR `Verification` 項目 (c)(d) の直接証跡として採用

### Rejected

なし

## Validation

- ADR `Verification` 6 項目すべて `[x]` + 証跡参照記録済み（同 ADR 本文参照）
- spec `SPEC-034Jc9QEqYLVfg1CXXTiPV` Acceptance Criteria 対応表:
  1. 候補比較文書化＋cpal 採否・代替候補の扱い確定 — discovery + ADR `accepted`（Decision Outcome・他候補の扱い節）で充足
  2. audio-core ポート定義・外部型非露出・全公開 enum `#[non_exhaustive]` — `device.rs` 実装 + `cargo doc` grep 0 件 + `exhaustive_enums` lint ゲートで充足
  3. `ruviform-audio-device` 分離クレート・cpal 閉じ込め・feature 意図命名・コアからの強制依存なし — `cargo metadata` 実測で充足
  4. PoC「列挙→既定構成→開始→コールバック到達」実証・実機手順文書化・CI はデバイス非依存テストのみ — 実験ログ（`RESULT: OK`）+ モック結合テストで充足
  5. コールバック非 alloc 契約明示 + 検証 — trait doc RT 契約 + `rt_output_callback_path_allocates_nothing`（assert_no_alloc）+ 実機計数アロケータ観測で充足
  6. `mise run fmt-check`/`check`/`lint`/`test` + `prek run --all-files` パス・`unsafe` は局所許可のみ — 本 IR 記録の検証列どおり全パス
- `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` — 全て exit 0（T3 完了時点で実測済み。T4 は文書変更のみ）
- `mise exec -- prek run --all-files` — 全 hook 合格
- `mise exec -- cargo metadata --format-version 1` — cpal は `ruviform-audio-device` の `[dependencies]` にのみ存在（T2 で実測済み）
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` — findings 0（67 ファイル）

## Risks

- **未観測項目が別 OS で顕在化する可能性** — Linux/ALSA・macOS/CoreAudio 未検証のため、同一契約の再観測は各プラットフォームの初回実機検証時に必要（CI の OS matrix はデバイス非依存テストのみをカバー）
- **初回 callback timestamp=0 の原因未同定** — `MasterClock` 配線で timestamp 精度が問題になる場合は engine 層 Issue で追検証

## Follow-ups

- 実機依存の残課題（ALSA vs PipeWire・Send/Sync ばらつき・WASAPI exclusive/ASIO・初回 timestamp 異常）は ADR `Confirmation` に引き継ぎ済み。duplex 要件化・cpal 再停滞時は ADR 再評価トリガとして同節に宣言済み
