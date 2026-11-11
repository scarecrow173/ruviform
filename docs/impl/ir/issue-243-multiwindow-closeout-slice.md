---
id: "IMPL-034VCg0FFHoUGWo9FDCg0f"
type: "impl"
status: "completed"
title: "Issue #243 Windows multiwindow verdict 再実走・帳簿整合 closeout slice"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "docs/specs/issue-243-windows-multiwindow-smoke.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-243-multiwindow-closeout-slice.md"
    modified:
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json"
      - type: "file"
        path: "evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "docs/impl/ir/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/impl/ir/issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/tasks/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/specs/issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/specs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/test-specs/issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/test-specs/issue-243-windows-multiwindow-smoke.md"
      - type: "file"
        path: "docs/plans/implement-issue-243-gpui-realwin-windows.md"
      - type: "file"
        path: "docs/plans/issue-243-windows-multiwindow-smoke.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/tasks/issue-243-windows-multiwindow-smoke.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-243-windows-multiwindow-smoke.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../ir/issue-243-windows-multiwindow-smoke.md"
    - "../ir/issue-243-gpui-realwin-windows.md"
    - "../ir/issue-552-sendinput-preflight-read-back-environment-blocked.md"
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

# Issue #243 Windows multiwindow verdict 再実走・帳簿整合 closeout slice

## Summary

Issue #243 Windows multiwindow slice の帳簿整合のみを行う closeout slice
（コード変更なし）。最新 main（`4eea9115`）で canonical lane
`mise run example-ui-gpui-real-os-multiwindow-smoke` を対話 desktop で
再実走し、`evidence/issue-243-multiwindow-verify/
verdict-multiwindow-windows.json` を現行 schema v2（`preflight` 付き）で
再生成した（exit 0・verdict=pass・16 checks green）。併せて
`evidence/rf313-os-capability-matrix/` の stale 行（
`real-os-multiwindow-reverify` NotRun・environment-blocked 記述）を実測に
整合させ、`environment-blocked-sendinput.md` に解決経緯（Vanguard
armed 最有力仮説・unload 後の SendInput 復旧・#552 preflight
fail-closed による再発防止）を追記し、#243 関連 task status を `done`
に更新して管理 index を再生成した。Issue #243 全体は Open 継続
（macOS/Linux・動的 DPI/モニター切断・Native Plugin Dock・consumer
非回帰・OS-resource-exhaustion 失敗注入・`post_event` 実窓配送観測は
別 Gate/別 Issue の残件）。

## Context

- task: `docs/tasks/issue-243-windows-multiwindow-smoke.md`
- 親 slice の IR: `docs/impl/ir/issue-243-windows-multiwindow-smoke.md`
  （Risks/Follow-ups に本 slice で解消した 2 項目が記録されていた）
- 先行 slice の IR: `docs/impl/ir/issue-243-gpui-realwin-windows.md`
  （Follow-ups に index 再生成の未実施が記録されていた）
- 環境ブロックの解消実測: PR #551（`9ec8c03d` — Vanguard unload 後の
  再実走 pass・schema v1 時点）
- preflight 導入: Issue #552 / PR #556（`4b8fa2b7` — schema v1→v2・
  verdict `environment_blocked`/exit 3 追加）
- rf552 側の v2 pass verdict は
  `evidence/rf552-sendinput-preflight/verdict-multiwindow-windows.json`
  に既存だが、canonical path（`evidence/issue-243-multiwindow-verify/`）
  は v1 のまま残っていた — 本 slice で再生成して整合させた。

## Environment

- OS: Windows 11 Pro x64・対話 console session（実カーソル占有 lane）
- toolchain: mise 管理 rustc 1.99.0・GPUI 0.2.2
- revision: `4eea9115`（`git merge origin/main` 後の最新 main と同一）
- 実施日: 2026-11-11
- モニタ環境: 1920x1080 × 2 面（scale=1.0、静的）

## Implementation

### Changes

- `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`:
  lane 実走により v1 → v2 schema で再生成（`preflight.effective=true`・
  pointer/keyboard 両 probe 有効・16 checks 全 ok・verdict=pass）。
- `evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`:
  `## Resolution (2026-11-11 追記)` を追記 — Vanguard `vgk.sys` armed
  状態が最有力仮説（PR #551 の独立 Rust プローブ・LL hook 実測・回復
  時系列）、因果未確証の明記、#552 preflight による再発防止、
  本 slice の v2 再実走 pass。
- `evidence/rf313-os-capability-matrix/results.jsonl`:
  - `real-os-multiwindow-lifecycle`: reason を実測に更新（環境ブロック
    記述 → 解消済み + v2 再実走 pass）、revision/ran_at_utc を更新。
  - `real-os-multiwindow-reverify`: NotRun → Pass（再検証実施済み・
    preflight effective・exit 0）。owner フィールドは解消済みのため除去。
- `evidence/rf313-os-capability-matrix/report.md`: #243 multiwindow
  slice の判定表を実測に整合（Pass 化）、環境ブロック記録に解決済み
  追記、再現手順の exit code 表に 3=`environment_blocked` を追記。
- `docs/impl/ir/issue-243-windows-multiwindow-smoke.md`: Risks/
  Follow-ups の 2 項目を解消済みに更新（歴史記録は打消し線で保持）。
- `docs/impl/ir/issue-243-gpui-realwin-windows.md`: Follow-ups の index
  再生成未実施項目を実施済みに更新。
- `docs/tasks/implement-issue-243-gpui-realwin-windows.md` /
  `docs/tasks/issue-243-windows-multiwindow-smoke.md`:
  status `in-progress` → `done`（実装完了・マージ済み・本 slice で
  Windows 再検証も完了）。
- spec / test-spec / plan の #243 関連文書 status（Coordinator
  承認による編集範囲拡張・2026-11-11）:
  - `docs/specs/issue-243-gpui-realwin-windows.md`: proposed → implemented
  - `docs/specs/issue-243-windows-multiwindow-smoke.md`: approved → implemented
  - `docs/test-specs/issue-243-gpui-realwin-windows.md`: in-progress → implemented
  - `docs/test-specs/issue-243-windows-multiwindow-smoke.md`: approved → implemented
  - `docs/plans/implement-issue-243-gpui-realwin-windows.md`: in-progress → completed
  - `docs/plans/issue-243-windows-multiwindow-smoke.md`: approved → completed
- `docs/{tasks,specs,test-specs,plans,designs}/README.md`:
  doc_maintenance apply で再生成（#243 行の status/配置更新に加え、
  直前に main へ着地した #313 design/task 行の index-ordering /
  metadata-mismatch も同時に解消）。
- `docs/impl/ir/README.md`: hand-curated index のため手動で本 IR 行を
  追加。

### Decisions During Implementation

- **exit code をそのまま記録**: lane は 0=pass で終了したため verdict=pass
  を記録。environment_blocked 再発時は exit 3 のまま記録する方針だったが
  実際には発生しなかった（preflight effective=true を実測）。
- **Vanguard 因果は「最有力仮説」と明記**: `sc query vgk` STOPPED を
  根拠に初回は否定と記録されていたが、service 停止 ≠ kernel driver
  unload であり得る点が後続観測（LoL 終了 → vgk ログ停止 → 全回復）と
  整合する。時系列相関ベースの仮説として記録し、確証とは書かない。
- **spec/test-spec/plan の status 更新は Coordinator 承認の範囲拡張で
  実施**: 初期の編集許可範囲は `docs/tasks/` 配下の status 更新に限定
  されていたため、`docs/specs`・`docs/test-specs`・`docs/plans` の
  6 ファイルについて `orca orchestration ask` で拡張を依頼し、全件の
  status 更新 + index 再生成が承認された（回答: 派生 index を直接
  直すのではなく front matter の源を修正するのが正しい境界、
  未検証項目を偽装しない旨の条件付き）。design の status は語彙上
  `approved` が実態と一致するため変更していない。

## Related Experiments

### Adopted

<!-- 実験ログなし（機械的帳簿整合が中心の slice） -->

### Rejected

<!-- なし -->

## Validation

| コマンド | exit | 結果 |
| --- | --- | --- |
| `git fetch origin && git merge origin/main` | 0 | Already up to date（`4eea9115`） |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` clean |
| `mise run example-ui-gpui-real-os-multiwindow-smoke` | 0 | verdict=pass・preflight effective・16 checks green・`verdict-multiwindow-windows.json` を v2 schema で再生成 |

補助証拠: verdict JSON を `python -c json.load` で検査 — `schema=
rf243-real-os-multiwindow-smoke/v2`、`verdict=pass`、`preflight.effective=
true`（pointer/keyboard 両 check）、`problems=[]`、`checks` 16 件全 ok、
`run.elapsed_ms=93928`。

## Risks

- SendInput 環境ブロックの Vanguard 因果は未確証。再発した場合は
  preflight が exit 3 `environment_blocked` で fail-closed するため、
  verdict の誤記録リスクは残らない。恒久 filter が再発する場合の
  backend 代替判断は #313 owner の残件。
- Issue #243 の残件（macOS/Linux 実機・動的 DPI/モニター切断・
  Native Plugin Dock・consumer 非回帰・OS-resource-exhaustion 失敗注入・
  `post_event` 実窓配送観測）は本 slice 対象外であり、Pass 扱いしない。

## Follow-ups

- ~~spec/test-spec/plan の #243 関連文書 status 行整合~~ **実施済み
  （2026-11-11）**: Coordinator 承認の範囲拡張で 6 ファイルの status
  front matter を更新し、管理 index を再生成済み（Changes 参照）。
  design の status は `approved` が実態と一致するため不変。
- macOS/Linux での同等 lane 実走、動的 DPI/モニター切断シナリオ —
  後続 #243 dispatch。
- Native Plugin Dock 受入 — #388/#389・Synvelis #176 の OS 別 Gate。
- consumer 非回帰 — audio-lens#166・synvelis#113/#114 が owner。
