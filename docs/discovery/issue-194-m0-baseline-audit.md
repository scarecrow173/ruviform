---
id: "DISC-034Q6prdW5pMq5zW9uTb3z"
type: "discovery"
status: "active"
title: "Issue #194 M0 互換性・依存・検証 Baseline 実測監査"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/194"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/configurable-ppqn-resolution-independent-musical-time.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/adr/select-audio-device-backend.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #194 M0 互換性・依存・検証 Baseline 実測監査

## Summary

固定コミット `0cd895b86d8dcff966e0236e8b6bb02f605f4719`（= 記録時点の `origin/main`）で M0-D 実測を実施し、静的監査（Issue 本文 M0-A〜E）と実測の整合を確定した。依存 DAG・テスト・feature lane・MSRV・実機列挙は全て緑。**workspace lint は HEAD で赤（`ruviform-numeric` 2 error）** だが、G0-③ の監査作業（実行・成功/失敗/対象外の分離）は実施済みであり **G0 監査条件は全項合格** と判定。lint 緑化（#199）は「M0 監査の完了条件」ではなく **M1 着手 Gate** として #194/#193 で分離管理する。詳細 Issue 12 件の Ready/Blocked は各 Issue コメントに反映済み。#194 の最終 Close は未観測項目の扱いと共に Owner 判定。本稿は M0 報告の唯一の正本であり、外部証跡ファイルへの依存はない（レビュー指摘で `m0-evidence/` は廃止・内容を本稿へ統合）。

## Exploration Goal

Issue #194 が定義する M0 の問い: 「M1〜M7 の実装者が既存互換・実行/検証手段・API/ADR 制約を推測せず判断できる不変の Baseline を作れるか」。静的監査（GitHub tree/API 経由・実行環境なし）で記録された仮説値を、実行可能な環境での実測に置き換え、未検証項目を「必要だが未検証」として明示することが本記録の目的。コード移行・新 API 公開は範囲外。

## Key Issues and Assumptions

実測で確定した Baseline 事実（静的監査の再検証結果）:

- 28 workspace crate・通常 path 依存 67 edges / 循環 0（normal+build・normal+dev 双方）を `cargo metadata`/`cargo tree` で実測確認。default feature で resolver が活性化したのは 65 edge（`resolve.nodes[].deps[].dep_kinds` 実測。manifest 宣言 67 のうち `host` feature ゲートの optional 2 edge が default では未活性。all-features resolve では 67 で宣言と一致）。dev path edges は 5 件で `plugin-host→ruviform-audio` を含む。**通常依存循環を前提にした再設計は不要**（#193 の前提を再確認）。
- `plugin-clap`/`plugin-vst3`→`plugin-host` も dev-only。`[target.'cfg(...)'.dependencies]` 配下の path 依存は 0 件。
- `media-core/src/music.rs` は `Ppqn(960)` 固定。Variable PPQN 後継 ADR は Accepted・実装移行前。
- `io-symphonia` の `make_encoder`/`make_muxer` は Unsupported を維持（成功偽装なし）。`mp4`/`aac` feature は `cargo check` で緑だが canonical mise task が未定義（Gap）。
- 現行 `OfflineDriver` は stateful 連続 render の意味論を保証しない（M0-B 確定のまま）。
- CI: `workflow_dispatch` のみ・HEAD での run 0 件・`icc`/`ui`/`l1` lane は定義後 CI 未実行・main は branch protection 未設定（404 確定）・rulesets `[]`。
- `rt_` name filter は部分文字列一致（matched 145 中、真の `rt_` 前置は 109、衝突 36）。「0 件一致で exit 0」の構造的欠陥は残存。

## Alternatives and Comparison

監査そのものではなく、G0-H が実装者へ枠付けした PoC 候補を記録する（独断変更防止のため候補/方法/合否/不採用時を併記）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| #191 WAV backend: 自前 PCM writer | 依存最小・契約を自前で厳密制御 | 実装コスト・format 拡張性 | PoC 比較対象 |
| #191 WAV backend: `hound` 系 crate | 実績のある PCM 処理 | 外部依存の保守・feature 制御 | PoC 比較対象 |
| #191 WAV backend: symphonia encoder 拡張 | 既存 backend との一体感 | 現行 `Unsupported` からの改修量 | PoC 比較対象 |
| #195 plan ownership: 単一 RT owner | 所有権の単純化 | 既存 swap 契約からの移行量 | PoC 比較対象（assert_no_alloc 窓で比較） |
| #195 plan ownership: plan-owned / stable-slot | 現行 `Mutex<Graph>`/PlanSwap との近接性 | overflow 時 retained 不変量の維持設計が必要 | PoC 比較対象 |
| #198 stateful offline: 連続意味論の導入 | E2E-L2 の前提 | 現行保証なし仕様との互換設計 | 合否は sequential PCM deterministic golden |

不採用時の行動: #191 は `Unsupported` 継続、#195 は現行 `Mutex/RwLock` 維持、#198 は `Unsupported` 明記 — いずれも既存動作を Baseline として保護し regress なし。

## Tentative Conclusions and Hypotheses

**Confirmed（実測確定）:**

- manifest 宣言依存 DAG は静的監査と完全一致（宣言 67/循環 0・dev 5・target 条件 0）。resolver 実測では default 活性 65・all-features 活性 67・両feature 集合とも循環 0 で、宣言と resolved の差分は `host` ゲート optional 2 edge のみ。通常/dev の混同なし。
- 既存回帰: fmt・check・test 1784 pass・rt_ 109 実在・全 feature lane（icc 219・ui-webview・L1 E2E 6 件実走・ofx・plugin-core・bench-smoke・symphonia check）・MSRV 1.89・実機 WASAPI 列挙 5 デバイスは緑。
- **lint は HEAD 実在の赤**（`resample.rs:73` doc_markdown・`:340` unfulfilled_lint_expectations、10-31 投入・最終 CI run 以降）。`--locked`・canonical entry 差ではない。Baseline の記録対象であり、緑化は #199（コード変更のため M0 範囲外）。
- CI の現行 SHA 検証状態は「GitHub Actions では未検証」。本稿のローカル実測が唯一の証跡。main は未保護（404 確定）。
- Synvelis #23 の F0（revision 固定・公開 API/MSRV 実用条件）は本実測で証拠化可能。

**Hypothesis（判定保留）:**

- G0-③「既存回帰テストの成功/失敗/対象外分離」: 実行・分離の**監査作業は実施済み＝合格**と再判定（条件文は「実行し分離する」であり実施を要求）。lint 赤は既存 baseline の測定結果として記録し、緑化は #199 に切り出し「M1 着手 Gate」として別管理。
- lint 赤の混入は「CI 自動ゲート不在 + 最終 run 以降の積上げ」が構造的原因と推定（403 commits 未検証区間）。branch protection/自動化の整備は M1 着手前の推奨事項として #194/#193 側へ記録。

## Open Questions

- M1 着手 Gate（#199 lint 緑化）: `#[expect]` の扱いは設計判断を含む — cast が truncation か widening かの確認が必要。監査側の ③ は実施済みとして閉じたが、着手 Gate の解除条件は #199 の緑化。
- 他 OS（Linux/macOS）での同一 lane 実測。pipewire-backend は Linux 限定。
- HEAD `0cd895b` での CI 全 lane run（現状 0 件）と新規 lane（icc/ui/l1）の CI 実績。
- 実機 `play_sine`・input/duplex capture・実外部規格 plugin・窓付き webview。
- `audio-device` realtime/pipewire・io-symphonia mp4/aac の canonical lane 整備（現在 task 未定義）。
- Golden 比較基盤（Graph/PPQN/Plugin/Media）未整備 — #191/#195〜#198 の受入で新設が必要。
- Organization 側 ruleset の有無（repo スコープ API では観測外）。

## Promotion Candidates

本監査の反映先は既存の詳細 Issue（spec/ADR の上流）であり、Issue コメントへの反映で済んだため新規 spec/ADR は作らない。

- [ ] spec-doc needed: 各詳細 Issue（#184〜#191・#195〜#198）が設計 doc 化の Owner。Ready/Blocked・初手・判定テスト・依存・Rollback を各 Issue コメントに反映済み。
- [ ] adr-doc needed: #195 plan ownership の採否（現行 Accepted `audio-engine-snapshot-swap` ADR からの変更は後継 ADR 判断）・#191 backend 選定。
- [x] issue needed: lint 緑化 → [#199](https://github.com/scarecrow173/ruviform/issues/199)（**M1 着手 Gate** — G0-③ 監査作業は実施済み合格）。io-symphonia mp4/aac の canonical lane 欠落は別途小さな整備 Issue 候補。

## Sources and Relations

- 主 Issue: [ruviform#194](https://github.com/scarecrow173/ruviform/issues/194)（M0・本記録が実測証拠）。上位: [#193](https://github.com/scarecrow173/ruviform/issues/193)・[#192](https://github.com/scarecrow173/ruviform/issues/192)・[synvelis#23](https://github.com/scarecrow173/synvelis/issues/23)。
- Ready/Blocked 反映先: ruviform #184〜#191・#195〜#198 の各 Issue コメント（2026-11-01 投稿済み）。
- 証跡の扱い: 本稿が M0 報告の正本（自己完結）。実測時の全量ログ・metadata JSON 等は公開しない（ローカルユーザー名パス・ホスト名・デバイス GUID 等の識別情報を含むため）。**公開 MD だけでは当時の生ログを第三者が独立検証することはできない** — 再検証は下記 `#### 再現・照合手順` に基づく同一条件での再実行で行う。実測 worker: Orca dispatched `task_8ab47e4aaf96`/`ctx_3b599b336ff9`。

## Evidence

### 2026-11-01: M0-D 実測記録（Windows 11）

**固定コミット:** `0cd895b86d8dcff966e0236e8b6bb02f605f4719`（作業中 HEAD 不動・ソースツリー無変更）。**環境:** Windows 11 `10.0.26200` / Git Bash x86_64 / mise 2026.10.1 / rustc+cargo 1.99.0（mise.toml pin）/ MSRV 用 rust 1.89.0。全コマンドは `mise run`/`mise exec --` 経由。

#### コマンド・exit code 一覧

| # | コマンド | exit | 結果要点 |
| --- | --- | --- | --- |
| a | `cargo metadata --locked --format-version 1` | 0 | 231 pkg（集計は `#### 再現・照合手順` の方法で再算出可能） |
| b | `cargo metadata --locked --all-features --format-version 1` | 0 | 456 pkg（+225） |
| c | `cargo tree --locked --workspace --edges normal,build` | 0 | 通常 path 依存は次節の表どおり |
| d | `cargo tree --locked --workspace --edges dev` | 0 | dev path edges 5（`plugin-host→audio`・`plugin-{clap,vst3}→plugin-host` 含む） |
| e | `cargo fmt --all -- --check` | 0 | 清浄 |
| f | `cargo check --workspace --all-targets --locked` | 0 | warning/error 0 |
| g | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **101** | `ruviform-numeric` 2 error（#199） |
| h | `cargo test --workspace --locked` | 0 | **1784 pass / 0 fail / 0 ignored**（87 binaries） |
| i | `cargo test --workspace --locked rt_ -- --list` | 0 | **145 matched**（真 rt_ 109 + 衝突 36） |
| j | `cargo test --workspace --locked rt_` | 0 | 145 pass / 0 fail |
| k | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` | 0/0/**101**/0/0 | canonical entry と同一（lint は `--locked` 起因ではない） |
| lane | `test-video-proc-icc` / `lint-video-proc-icc` | 0/0 | 219 pass + lint 清浄 |
| lane | `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` | 0/0/0 | 7 pass・窓付きは未観測 |
| lane | `test-l1-real-plugin` / `lint-l1-real-plugin` | 0/0 | E2E 6 件実走（CLAP3+VST3 3・zero-match ではない） |
| lane | `test-ofx`/`test-ofx-bridge`/`lint-ofx`/`test-plugin-core`/`lint-plugin-core` | 全 0 | 51+1+141 pass・lint 清浄 |
| lane | `bench-smoke` | 0 | math/rt/viz 各 1 回実行 |
| lane | `cargo check -p ruviform-io-symphonia --locked --features mp4,aac` | 0 | canonical task 未定義 → 代替 check で緑 |
| lane | `test-ofx-msrv` | 0 | rust 1.89 で 192 pass |
| lane | `mise exec rust@1.89 -- cargo check --workspace --locked` | 0 | 全 28 crate lib/bin check 緑（test/bench は範囲外） |
| lane | `cargo run -p ruviform-audio-device --example list_devices` | 0 | WASAPI 5 デバイス列挙（実機） |
| env | `mise install --locked` | **1** | global mise config ツールが lockfile 未収録で resolve 失敗（repo 管理 7 ツールは installed。環境起因） |

#### CI・保護・実機証跡

- `gh run list --commit 0cd895b`: **0 件**。最終 run 39364871013（2026-10-20 @ `6839101`）は verify×3・rt・bench の旧 5 job のみで `icc-verify`/`ui-verify`/`l1-real-verify` は CI 未実行。
- `gh api repos/.../branches/main/protection`: **404 "Branch not protected"**（確定値）。`rulesets`: `[]`（確定的空）。Organization 側は観測外。
- `list_devices` example: NVIDIA HDMI×2・Audient iD4×2・S/PDIF の 5 WASAPI 出力列挙、各 DeviceId/AudioFormat/supported formats（8k〜384k・F32・Stereo）取得。

#### 静的監査との Discrepancy

1. lint ゲートが HEAD で赤（上記・#199 で緑化対応）。
2. AGENTS.md の bench-smoke 記述は 2 本だが実 task は math/rt/viz 3 本（doc drift・mise.toml が正本）。
3. `rt_` filter は部分文字列一致（145 中 36 衝突）。lane 名の精度は約 75%。
4. io-symphonia `mp4`/`aac` に canonical task なし（代替 check で緑）。
5. `mise install --locked` が global config 環境で失敗（環境起因・貢献者環境で再現し得る）。
6. branch protection「403 で不明」→ 404 未保護に確定。
7. 静的監査「audio-device example は実機依存で CI 対象外」→ 本環境で実機列挙成功。
8. default `cargo test` の ignored は 0（`required-features` 分離が機能している証拠）。
9. `--all-features` で +225 pkg — feature lane 必要性の定量裏付け。

#### G0 判定（7 項）

① manifest 実測・dev 混同なし:**合格** / ② `cargo metadata --locked` 記録:**合格** / ③ 回帰実行・分離:**合格**（監査作業=実行・分離は実施済み。lint 赤は測定結果として記録し #199 へ分離）/ ④ API/ADR/Consumer 影響マップ判定可能:**合格** / ⑤ 検証済み・未検証の分離:**合格** / ⑥ Unsupported の成功偽装なし:**合格** / ⑦ 権限不足を Blocked 記録:**合格**。**G0 監査条件は全項合格**。なお **M1 着手 Gate（#199 lint 緑化・CI 自動ゲート整備）は G0 とは別に定義**し、他 OS/CI lane/実機走行等の未観測項目が残るため **#194 の Close 判定は本稿では行わない**（Owner 判定事項）。

#### G0-H 反映（12 Issue・コメント投稿済み）

Ready: #184/#185/#186/#188/#189/#190/#191/#197/#198。Blocked（条件付き）: #187（実機 duplex/capture 走行ログ）・#195（#184 確定）・#196（#184+#190 確定）。PoC 枠付けは各コメントと §Alternatives。**各 Issue の「Ready」は「G0 を満たした場合の設計/PoC 着手可能」を意味し、全面機能実装 Ready ではない**。共通の M1 着手 Gate: ①lint 緑化（#199）②CI 自動ゲート整備を M1 着手前に潰すことを推奨。

#### 未観測項目

Linux/macOS・HEAD の CI 全 lane・`play_sine`/input capture・実外部規格 plugin・窓付き webview・audio-device realtime/pipewire lane・`mise run bench` 全測定・MSRV の test/feature 範囲・prek/apm/LFS・Organization ruleset・Golden 比較基盤。

#### 再現・照合手順

外部ファイル・スクリプトへの依存はない。以下を基準 SHA の worktree で実行すれば各集計値を再算出できる。

```sh
git worktree add <dir> 0cd895b86d8dcff966e0236e8b6bb02f605f4719 && cd <dir>
mise install   # `mise install --locked` は global mise config に lockfile 未収録ツールが
               # ある環境で exit 1 になり得る（環境起因・実測済み）。隔離する場合は
               # MISE_GLOBAL_CONFIG_FILE=<空 toml> を指定して実行

# 検証 lane — && で連結しない（lint exit 101 で後続が停止するため独立に実行）
mise run fmt-check        # 記録値 0
mise run check            # 記録値 0
mise run lint             # 記録値 101（ruviform-numeric 2 error・#199）
mise run test             # 記録値 0（1784 pass/0 fail/0 ignored・87 blocks）
mise run test-rt          # 記録値 0（145 pass・厳密 rt_ 前置 109 + 部分一致 36）
# opt-in lane: test-l1-real-plugin / lint-l1-real-plugin /
#   check-ui-webview / lint-ui-webview / test-ui-webview / bench-smoke /
#   test-video-proc-icc / lint-video-proc-icc / test-ofx* / test-plugin-core
# MSRV: mise install rust@1.89.0 後に mise exec rust@1.89.0 -- cargo check --workspace --locked
```

**DAG 集計方法（宣言 vs 解決済みの区別）:** `cargo metadata --locked --format-version 1` の JSON に対し、`packages[].dependencies` を走査したものが **manifest 宣言 edge**（feature 非依存・宣言テキスト）、`resolve.nodes[].deps[].dep_kinds`（`kind`/`target` フィールド）を workspace メンバーの path 依存に限って走査したものが **resolved 活性 edge**（当該 metadata 実行時の feature 集合で有効化されたもの）。結果: 宣言 67 / default resolve 65 / all-features resolve 67 / 循環 0。default で未活性の 2 edge は `host` feature ゲートの `plugin-clap→plugin-export`・`plugin-vst3→plugin-export`。テスト件数は `test result:` 行の `passed/failed/ignored` を集計（87 blocks）。default↔all-features の +225 pkg は両 metadata の `packages` 件数差。

**CI/保護の再確認:** `gh run list --commit 0cd895b86d8dcff966e0236e8b6bb02f605f4719`（記録値 0 件）・`gh api repos/scarecrow173/ruviform/branches/main/protection`（記録値 404）・`gh api repos/scarecrow173/ruviform/rulesets`（記録値 `[]`）。

#### 証跡の保管と検証限界

実測時の全量実行ログ・metadata JSON・cargo tree 全量出力・gh API 応答は、ローカルユーザー名パス・ホスト名・WASAPI DeviceId(GUID) を含むため**公開成果物には含めない**（調査者ローカル環境に保管）。これら原本は本稿の集計値生成に用いた一次資料であり、**公開 MD 単体では当時の生ログの独立検証はできない** — 再検証は上記手順による同一 SHA・同等環境での再実行をもって行う（本稿の値は再測定・推測ではなく当該環境での 1 回の実測記録）。
