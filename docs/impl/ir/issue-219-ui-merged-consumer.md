---
id: "IMPL-034QVA0eue5vkEk1WKOcRA"
type: "impl"
status: "completed"
title: "Issue #219 UI 横断ゲート追記反映・merged 変更の Consumer 影響分類・証拠状況記録"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-219-ui-merged-consumer.md"
    modified:
      - type: "doc"
        path: "docs/specs/issue-219-consumer-compat-contract.md"
      - type: "doc"
        path: "docs/discovery/issue-219-consumer-cross-repo-ci.md"
      - type: "doc"
        path: "docs/impl/ir/issue-265-correlationplan-prefers-direct-mixed-radix-direct.md"
      - type: "doc"
        path: "docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "../../specs/issue-219-consumer-compat-contract.md"
    - "../../discovery/issue-219-consumer-cross-repo-ci.md"
    - "../../designs/issue-219-cross-repo-ci-gate-credential-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../adr/cross-repo-ci-revision.md"
    - "issue-219-consumer-verify-gate.md"
    - "issue-265-correlationplan-prefers-direct-mixed-radix-direct.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../specs/issue-219-consumer-compat-contract.md"
    - "../../discovery/issue-219-consumer-cross-repo-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/issues/47"
    - "https://github.com/scarecrow173/synvelis/issues/48"
    - "https://github.com/scarecrow173/audio-lens/issues/131"
    - "https://github.com/scarecrow173/audio-lens/issues/132"
    - "https://github.com/scarecrow173/audio-lens/pull/128"
    - "https://github.com/scarecrow173/audio-lens/pull/134"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #219 UI 横断ゲート追記反映・merged 変更の Consumer 影響分類・証拠状況記録

## Summary

Issue #219 本体の 2026-11-03 追記（UI Component System 横断ゲート）を canonical 文書に反映し、追記後に merged された直近変更（`ui/*` #258〜#263・numeric FFT #255/#265・reader EOF 修正 #249・#219 基盤 #224）の両 Consumer 影響分類を discovery `## Evidence` に dated snapshot として記録した。`consumer-verify.yml` は情報 lane のまま維持（required 昇格なし・credential 未設定で実走 0 件であることを正直に記録）。併せて issue-265 IR の `references` に混入していた prose 参照を GitHub URL に修正し、artifact-graph の `broken-relation` blocker を解消した。

## Context

- 上流: `docs/specs/issue-219-consumer-compat-contract.md`（R10/AC11 を今回追加）・`docs/discovery/issue-219-consumer-cross-repo-ci.md`（Evidence dated snapshot 追記運用・design の Operational Tracking に従う）・`docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`（approved）・`docs/adr/cross-repo-ci-revision.md`（Option A+1+i）
- 先行実装: `docs/impl/ir/issue-219-consumer-verify-gate.md`（workflow・テンプレート・AGENTS.md drift 修正、completed）
- Issue 追記（2026-11-03）の要求: `ui/*` 共通基盤の変更でも両 Consumer を独立評価（Synvelis: 制御/Transport/RT 非阻害、AudioLens: f64・NaN 欠測・Validity/Provenance・bounded Dataset 可視化）、情報レーンは required 化しない、API/behavior/schema/renderer/性能差分 PR に分類記録・未検証は追跡 Issue リンク。
- 実態: merged PR #256/#266/#267〜#271/#254 の PR 本文に Consumer 影響分類欄の記入がなく、PR 編集（GitHub 側書込み）は本 dispatch の範囲外のため in-repo の canonical 記録で補完した。

## Implementation

### Changes

- `docs/specs/issue-219-consumer-compat-contract.md` — R10（UI 横断ゲート: Consumer 固有評価軸・追跡 Issue リンク・情報レーン非昇格）と AC11（merged 変更の分類記録）を追加。In Scope に追記反映・分類記録を追記。references に #257・Synvelis #47/#48・AudioLens #131/#132 を追加。status は `proposed` のまま（Issue の Done 基準 = 初回両 Consumer CI 実走・追従更新・起票〜解消ループの検証であり、まだ完了していないため）。
- `docs/discovery/issue-219-consumer-cross-repo-ci.md` — `## Evidence` に `### 2026-11-03` dated snapshot を append（append-only 運用）。synvelis workspace 化（`f7aadd0`・main 追従依存・lock `322a661`）の棚卸し差分、9 件の merged 変更の分類表（変更種別・両 Consumer 分類・根拠・追跡先）、追従更新 PR ループ実績（audio-lens #128/#134）、verified / informational / not-yet-required / not verified の正直な区分、差分後の M0〜M7 受入証拠表を記録。
- `docs/impl/ir/issue-265-correlationplan-prefers-direct-mixed-radix-direct.md` — `relations.references` の prose 文字列 2 件（`broken-relation` blocker 起因）を GitHub URL 4 件（ruviform#255・#256・audio-lens#133・PR#134）に修正し、`source` に Issue #265・`updated` を補正。本文記述は不変。
- `docs/impl/ir/README.md` — 手動 curated index に本 IR の行を追記（generated marker は付いていない手更新ファイル）。`docs/specs/README.md`・`docs/discovery/README.md` も index 再生成ツールで確認したが、対象文書の title/status 不変のため差分なし。

### Decisions During Implementation

- **分類記録の置き場所を discovery Evidence とした**: Issue ルール2 は「Issue/PR に根拠を残す」ことを要求するが、merged PR 本文の編集は GitHub 側書込みであり本 dispatch の報告経路（GitHub 非投稿）と矛盾する。design が定める M0〜M7・運用証跡の正本は discovery `## Evidence` の dated snapshot であり、post-merge の分類記録として同所に記録するのが canonical で一貫している。
- **Synvelis 分類は lockfile 実測で個別判定**: 当初「workspace 未存在」で一律 `no impact` とする草案だったが、実測で synvelis main `f7aadd0` が 5 member workspace（main 追従 git 依存・lock `322a661`）に前進していたため、lockfile crate 含有・ソース利用面で行ごとに判定した（media-reader を含む #254 は `impacted`、numeric 非含有の FFT 系は `no impact`）。`unknown` は使わず、根拠を明示した。
- **`ui/*` は両 Consumer `impacted`**: 新設パッケージは consumer 採用を目的とする共通基盤であり、追記が独立評価を明示的に要求している。変更は `ui/` TS パッケージのみで既存 pin 中 Rust crate は無変更だが、採用・受入評価面が追加されたため `impacted` + 追跡 Issue リンク（#47/#48/#131/#132）で表現し、`unknown` で誤魔化さない。
- **ADR/spec の status は据置**: ADR `cross-repo-ci-revision`（proposed）・spec（proposed）の昇格は Issue クローズ基準（実走証拠）未達のため行わない。実装済みだが「完了」とは記録しない。

## Related Experiments

### Adopted

なし（文書運用タスク。実測は `gh` API の read-only 参照のみ）

### Rejected

なし

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| doc graph blocker 解消 | `route_graph.js --current followup-triage` | issue-265 IR の `broken-relation` 2 件が消え artifact-graph blocker 解除（残 blocker は pre-existing の他ファイル由来 + focus-required） |
| workflow 構文 | `mise exec -- actionlint .github/workflows/consumer-verify.yml` | clean（workflow 本体は無改変・再確認のみ） |
| doc audit（本 Issue 関連） | `audit_docs.js --type spec` / `--type discovery` / impl audit | 本 Issue の新規・変更文書に blocking finding なし（repo 全体には pre-existing blocker 109 件が残存・本 scope 外） |
| cargo 系 | `mise run fmt-check` / `check` / `lint` / `test` | green（文書のみの変更だが規約どおり実施） |
| Issue 追記の反映確認 | spec R10/AC11 + discovery snapshot 対 Issue 追記本文の照合 | 評価軸・追跡 Issue・非昇格・分類記録の 4 要件を全て収録 |
| 追跡 Issue の実在 | `gh issue view`（synvelis#47/#48・audio-lens#131/#132/#127/#133・synvelis#28） | 全て実在（#133 CLOSED・他 OPEN） |
| audio-lens 現行 pin | `gh api repos/scarecrow173/audio-lens/contents/Cargo.toml` で `rev = "b9f2a10…"` を確認・`git rev-list --count b9f2a10..main`（ruviform 側）で遅れ計測 | #266 merge commit・ruviform main `ced77f0` 比 12 commit 遅れ（ui/* merge 5 件 #267〜#271 + 配下 feature commit） |
| synvelis 棚卸し差分 | `F:/repositry/audio-products/synvelis` の `Cargo.toml`/`Cargo.lock`/`git log` を read-only 実測 | main `f7aadd0` が 5 member workspace・`git` 追従 dep×10・lock `322a661`（ruviform HEAD 比 39 commit 遅れ・`ruviform-numeric`/`ruviform-ui` 非含有） |

## 未検証（意図的・別運用）

- `consumer-verify` の dispatch 実走: `CROSS_REPO_DEPLOY_KEY`・Deploy Key 一次設定が未実施（owner/admin 作業・design Appendix runbook）。情報 lane のまま。
- ui/* の consumer 側受入評価: Synvelis #47/#48・AudioLens #131/#132 が追跡 Issue（全 OPEN・未実施）。
- audio-lens 側 `test-ci`/`check-cli-headless`/`git lfs pull` の CI 経路、Golden/`baseline` lane の cross-repo 化、Synvelis 側の全 consumer 検証（`f7aadd0` で workspace は存在・本 dispatch では consumer repo 非変更のため未実測）。
- Issue #261（ui/binding）の IR が repo に存在しない — 本 scope 外の別ギャップとして記録。

## Risks

- post-merge 分類は in-repo 記録であり merged PR 本文には反映されていない。今後の PR はテンプレート欄の記入 + reviewer human gate（AGENTS.md 運用）でカバーする運用。
- Synvelis は main 追従依存（rev pin なし）のため lockfile re-resolve で随時 main の変更を取り込む。分類は lockfile crate 含有に基づくため、依存追加（`ruviform-numeric`・`ruviform-ui` 等）が入れば次回分類対象の結論が変わり得る — synvelis#28 で管理。
- `docs/impl/ir/README.md` は手動 curated で、本追加以前から複数の既存 IR（issue-181/-189/-191/-196/-197/-238/-242/-259/-260 等）が未収録のまま。本 scope 外の pre-existing gap として記録（別途 index 修復タスク候補）。

## Follow-ups

- owner/admin: `CROSS_REPO_DEPLOY_KEY` 一次設定（design Appendix runbook）→ 初回 dispatch → 結果を Issue #219・本 discovery Evidence に追記。
- Issue #261（ui/binding）の IR 欠落は別タスクで補完する。
- spec/ADR の status 昇格は Issue の Done 基準（初回実走・追従更新・解消ループ証拠）達成後に判断。
