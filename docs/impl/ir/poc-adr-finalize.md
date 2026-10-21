---
id: "IMPL-034JEE2YEByoDOnrH2M0XT"
type: "impl"
status: "completed"
title: "PoC 採否 ADR 確定（エビデンス集約と confirm/revise）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/exp/ac7-adr-finalize.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "doc"
        path: "docs/adr/ui-tech-selection-poc.md"
      - type: "doc"
        path: "docs/specs/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/plans/issue-28-ui-poc.md"
      - type: "doc"
        path: "docs/tasks/poc-adr-finalize.md"
      - type: "doc"
        path: "docs/adr/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/poc-adr-finalize.md"
    - "../../adr/ui-tech-selection-poc.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-28-ui-poc.md"
    - "../../designs/issue-28-ui-poc.md"
    - "../../adr/ui-tech-selection-poc.md"
    - "poc-ui.md"
    - "poc-render-backends.md"
    - "poc-message-boundary.md"
    - "poc-native-surface.md"
    - "poc-frontend-and-dist.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../specs/issue-28-ui-poc.md"
    - "../../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/issue-28-ui-poc.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/34"
    - "https://github.com/tauri-apps/wry/pull/1762"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "../exp/ac7-adr-finalize.jsonl"
    rejected: []
---

# PoC 採否 ADR 確定（エビデンス集約と confirm/revise）

## Summary

T6（AC7・AC8 最終確認）を実施し、採否 ADR `ADR-034Jc9Cguud7nZ2Tt88Ptu` を `proposed` → `accepted` へ遷移させた。コード変更はないドキュメント工程であり、「実装」はエビデンス集約と各条項の confirm/revise 判定そのもの。

- `poc/ui/RESULTS.md` の AC → 実験 → エビデンス対応表を完成させた（AC7 行を T6 作業へ紐付け、AC7 セクションに条項別 verdict 表と監査再現コマンドを追加。全行が env・再現コマンド・証跡を持つことを確認）
- 採否 ADR の全 7 条項をエビデンスリンク付きで確定: **confirm 5 条項**（WebView 実装・メッセージ境界・フロントエンド FW・配布・採否構造）、**revise 2 条項**（高頻度描画・ネイティブサーフェス合成）
- spec を `approved` → `implemented`（AC1–AC8 全チェック）、plan を `completed`、全 task（T1–T6）を `done` に同期。impl 記録が全 task に紐付いていること・exp ログが揃っていることを監査
- wgpu MSRV 1.87 と宣言 `rust-version = "1.85"` の衝突を Issue #34 への入力として ADR Follow-ups・連携事項に記録（引き上げ自体は決めない）

## Context

- 実施タスク: [../../tasks/poc-adr-finalize.md](../../tasks/poc-adr-finalize.md)（`TASK-034JfHz5u1uXcPkG8d8t4B`、plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T6）
- 上流: [plan](../../plans/issue-28-ui-poc.md)（T6 分解）・[design](../../designs/issue-28-ui-poc.md)・[spec](../../specs/issue-28-ui-poc.md)（AC7・AC8 の正本）・[test-spec](../../test-specs/issue-28-ui-poc.md)（confirm 条件・文書評価ラベルの判定規則）・[採否 ADR](../../adr/ui-tech-selection-poc.md)（confirm/revise 対象）
- 前提: T1–T5 の impl 記録（[poc-ui.md](poc-ui.md)・[poc-render-backends.md](poc-render-backends.md)・[poc-message-boundary.md](poc-message-boundary.md)・[poc-native-surface.md](poc-native-surface.md)・[poc-frontend-and-dist.md](poc-frontend-and-dist.md)）と全実験ログ（`docs/impl/exp/ac1–ac6-*.jsonl`）が揃っていること。`poc/ui` 独立ワークスペースの隔離維持（AC8）・コミットしない

## Implementation

### Changes

- **`poc/ui/RESULTS.md`**: 対応表の AC7 行を `docs/tasks/poc-adr-finalize.md`（T6 本工程）へ紐付けて完成。AC7 セクションを新設し、7 採否条項の verdict 表（confirm/revise + 根拠エビデンス）と監査再現コマンドを記録。併せて対応表の AC6/AC8 アンカーと全 `#acN` 見出しアンカーを正しいスラグに修正（`--experiment` のハイフン保持で `ac1----experiment-host` 形式が正）
- **`docs/adr/ui-tech-selection-poc.md`**: 7 条項すべての `要 PoC 確証` を confirm/revise + エビデンスリンクに置換し status を `accepted` へ。Considered Options の `SetWindowRgn` 回避策・WebGPU 可用性の記述に PoC 実測の注記を反映。Verification 全 8 項目をチェック。Consequences を確定後の実態（2 条項 revise）に更新し Follow-ups（MSRV 連携・WebGPU 有効化経路・PR #1762 追跡・macOS/Linux 実機化・wxp 差分精査・Linux 配布方式）を明記
- **status 同期**: spec `approved` → `implemented`（AC1–AC8 チェック）・plan `completed`・task `done`・各 `changes` 記録を実施結果に同期・README 索引 4 件更新

### Decisions During Implementation

- **confirm/revise の判定基準**: 下書きの採否方向が実測で裏付けられたが条項の事実記述に修正が要るものを revise（描画: WebGPU 非可用 / 合成: z-order 既定・SetWindowRgn 不成立）、方向と記述がそのまま実測で裏付けられたものを confirm とした。revise でも採否自体が覆った条項はない — 隣接構成と特性別併用は成立を維持
- **メッセージ境界は confirm（追記あり）**: 3 経路併用の実現性は覆らなかったが、実測で明らかになった coalesce・コールドパス・タイマ量子の制約を「採用条件としての設計含意」として条項に追記した（test-spec の「黙って confirm せず」規則に照らし、条項文言の修正部分は注記として明示）
- **文書評価ラベルの維持**: macOS / Linux 実挙動・PR #1762 依存構成・WebGPU 有効化経路は実機未検証のまま `文書評価` / 後続調査として明示。実測と文書評価の混在は採否誤認リスクのため厳格に区別
- **`implemented-by` の向き**: task → 本 IR（`../../impl/ir/poc-adr-finalize.md`）を紐付け、採否 ADR 側にも `implemented-by` を追加して確定作業との対応を graph に記録した

## Related Experiments

### Adopted

- [../exp/ac7-adr-finalize.jsonl](../exp/ac7-adr-finalize.jsonl) — AC7 エビデンス集約と各条項 confirm/revise verdict の逐条記録

### Rejected

なし。

## Validation

- `node .agents/skills/doc-status/scripts/audit_docs.js --type adr --json` → `findings: []`
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` → `findings: []`
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir --json` → `findings: []`
- `node .agents/skills/impl-doc/scripts/audit_experiment_log.js --dir docs/impl/exp --json` → `findings: []`
- `git status --porcelain -- Cargo.toml Cargo.lock crates mise.toml` → 空（AC8 最終確認）
- `grep -rn "要 PoC 確証" docs/adr/ui-tech-selection-poc.md` → 0 件（全条項解消）
- Manual: `RESULTS.md` 対応表の全行に環境（env 表）・再現コマンド・証跡（`out/*.jsonl` / `docs/impl/exp/*.jsonl`）があり、文書評価ラベルが必要な行（AC6 macOS/Linux 行・リスク定量化表）すべてにラベル付き

## Risks

- **RESULTS.md アンカーと外部リンク**: `poc/ui` は doc-status 監査の走査範囲外のため、ADR 側はファイルリンクで cite（見出し名はリンクテキストで明示）。RESULTS.md 内部アンカーは GitHub スラグ規則に合わせ修正済み
- **revise 条項の後続影響**: in-WebView WebGPU 非可用・`SetWindowRgn` 回避策不成立は将来の `ruviform-ui` / `ruviform-ui-webview` 設計の前提制約になる。実装 Issue 起票時に ADR 条項を再参照する運用が前提
- **文書評価項目の残存リスク**: macOS / Linux の合成・配布挙動は実機未検証。対象 OS 確定時の実機検証まで採否の一部は文書根拠に依存する

## Follow-ups

- **MSRV 連携（Issue #34）**: `wgpu` MSRV 1.87 がルート宣言 `rust-version = "1.85"` と衝突する事実を、MSRV 見直し専用タスクへの入力として記録（ADR Follow-ups (a)・連携事項）。本 PoC は独立 WS + pin 1.99.0 のため実害なし。wgpu 採用条項はこの衝突解消を条件とする
- **WebView2 内 WebGPU 有効化経路**: SDK 側 opt-in API の有無など、flags 以外の有効化経路の調査（再び採用候補にする場合の前提）
- **wry PR #1762 追跡**: 上流 merge 状況の監視 / fork 保守採否は DOM 内合成が必要になった時点で判断
- **macOS / Linux 実機検証**: `.app`+署名/公証・`libwebkit2gtk-4.1`+GTK3・deb/AppImage ・layer 合成の実機確認は配布対象 OS 確定時に実施
- **wxp パッチ差分精査**: プラグイン GUI Issue への引き渡し材料として整理済み（採用判断の差分レビュー自体は別 Issue）
