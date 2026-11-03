---
id: "DISC-034QdBDFXQ7CpGCCYpN94k"
type: "discovery"
status: "draft"
title: "Issue #219 両 Consumer 依存形態・検証基盤棚卸しと Cross-Repo CI 前提実測"
created: "2026-11-02"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
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
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/designs/ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/guides/ruviform-private-access-ci.md"
  defers: []
  deferred-by: []
---

# Issue #219 両 Consumer 依存形態・検証基盤棚卸しと Cross-Repo CI 前提実測

## Summary

両 Consumer の依存形態・toolchain・CI・private repo アクセスを棚卸しした。AudioLens は `git + rev` 固定の 14 crate 依存を持つ実働 Rust Consumer で、Deploy Key 方式の private CI 設計は完成済みだが **credential 未プロビジョニング（Deploy Key 未登録・Actions secret 未設定）で CI 実走 0 件**。Synvelis は設計段階で Rust workspace が存在せず、ビルド系 Cross-Repo 検証は現状「実行不能（not run）」として扱う。Consumer manifest の `rev` を候補 SHA へ書き換え `cargo fetch` で再解決する追従検証機構はローカルで実測済み（全 ruviform crate が単一候補 SHA に再解決）。

## Exploration Goal

Issue #219 の問い: 「Ruviform を Synvelis・AudioLens の対等な共通基盤として並行開発する際、両 Consumer の互換性を継続的に判定し、M0〜M7 の Consumer 別受入証拠を追跡できる運用と CI の実装可能範囲をどう構築するか」。本 discovery は (1) 両 Consumer の依存形態・required features・runner/toolchain・private repo アクセスの棚卸し、(2) Cross-Repo CI の前提条件の実測、(3) M0〜M7 Consumer 別証拠の初期状態を記録し、spec / ADR / design の確定材料とする。

## Key Issues and Assumptions

実測済みの棚卸し事実（各値の取得コマンドは `## Evidence`）:

### AudioLens（`scarecrow173/audio-lens`、HEAD `1a43410`・2026-11-01）

- Rust workspace 実働 Consumer。member 11 crate（`al-core`/`al-job`/`al-cli`/`al-media`/`al-dataset`/`al-locate`/`al-analysis`/`al-service`/`al-gui`/`al-testkit`/`al-observability`）、`edition = "2024"`・`publish = false`・`[profile.ci]` あり。
- **依存形態**: `[workspace.dependencies]` に `git = "https://github.com/scarecrow173/ruviform.git"` + **全 14 entry 同一 `rev`**（現行 `080c228d197ebc645b8f81cd0082be442abaf10c`＝2026-10-31 PR #180 merge、ruviform main `41772a7` から 57 commit 遅れ）。member は `*.workspace = true` で消費し feature/SHA は crate 間で分岐しない。Cargo.lock は 15 crate（14 直接 + transitive `ruviform-rt`）が**単一 resolved rev**。
- **required features**: `ruviform-io-symphonia` の `features = ["mp4","aac"]` のみ非 default。他 crate は default feature。
- **private repo アクセス**: `.cargo/config.toml` で `net.git-fetch-with-cli = true`（cargo の fetch を system git へ委譲。ローカルは Git Credential Manager、CI は Deploy Key）+ `GIT_LFS_SKIP_SMUDGE = "1"`（ruviform の LFS fixture を引かない）。設計・手順の正本は audio-lens 側 `docs/designs/ruviform-dependency-ci.md` / `docs/guides/ruviform-private-access-ci.md`。
- **runner/toolchain**: mise が正本。`rust = 1.99.0`（components rustfmt,clippy・ruviform と同 pin）、`node = 26.12.1`、`uv = 0.13.5`、`github:microsoft/apm = 0.33.0`、`pipx:serena-agent = 1.7.0`、`prek = 0.5.7`、`biome = 2.6.2`、`git-lfs = 3.8.0`、`actionlint = 1.7.12`。検証 task: `fmt`/`check`/`lint`/`test`/`build`、`*-ci` 系（`--profile ci`）、`verify-ci`、`verify-toolchain`、`check-cli-headless`、`check/lint/test-ui-webview`、`baseline`。
- **CI 現状**: `.github/workflows/ci.yml` は `workflow_dispatch` 専用・3 OS matrix。private 依存に `RUVIFORM_DEPLOY_KEY` secret（ruviform 側 read-only Deploy Key）を要求するが、**`gh secret list`（audio-lens）= 空・`gh api repos/ruviform/keys` = 空で credential 未設定のまま run 実績 0 件**。main は branch protection 未設定。
- **Golden/回帰資産**: `baseline/`・`tools/baseline/capture.mjs`（Issue #98 pre-migration baseline）、`dictionaries/`、`samples/`、LFS 管理の `assets/`（`*.mp4` 等。CI checkout は `lfs: true`）。
- **追従意思**: audio-lens #127 が「rev pin の恒久固定をやめ既定 branch 追従 + lockfile 更新 PR」への移行方針を宣言済み。現行 rev-pin 形式は移行期の実装であり、CI 側は rev-pin 形式を**現状の正**として扱う。

### Synvelis（`scarecrow173/synvelis`、HEAD `041f9b7`）

- **設計段階**: `Cargo.toml`・`.rs`・`.github/` が**存在しない**。Rust toolchain も mise.toml に未 pin（`uv`/`prek`/`git-lfs`/`apm`/`serena-agent` のみ）。検証 task は `setup` のみ。
- **依存形態**: ruviform への依存は設計参照のみ（Target Architecture v0.7.1 正本 `docs/designs/synvelis-driven-ruviform-target-architecture-v07.md`）。buildable な依存解決対象がないため、Cross-Repo のビルド検証は**現状「not run（workspace 未存在）」**と記録する。推測で `pass` 扱いしない。
- main は branch protection 未設定。GitHub remote `scarecrow173/synvelis`（private）。
- Consumer 側 Gate 定義: Synvelis #23 が F0/E2E-L/R/X の受入を所有し、#28 が Ruviform 追従の横断作業を所有（いずれも OPEN・未実装）。

### Ruviform 側（本 repo）

- `.github/workflows/ci.yml` は push/pull_request/`workflow_dispatch` で稼働（#199/#201 closed 済み・main 最新 run 成功・**main に 12 件の required status checks が設定済み**。AGENTS.md の「`workflow_dispatch` 専用」記述は stale で drift 修正が必要）。
- `gh secret list`（ruviform）= 空、`gh api repos/ruviform/keys` = 空 → Cross-Repo 用 credential は真っ新な新設。
- `.github/ISSUE_TEMPLATE`・`PULL_REQUEST_TEMPLATE` は**未存在** → 影響判定テンプレートは新設。

## Alternatives and Comparison

Cross-Repo アクセス credential 候補（ADR で確定。audio-lens 側設計の採否を継承する形で評価）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 専用 read-only Deploy Key（全 3 repo へ同一公開鍵登録、ruviform secrets に秘密鍵） | 最小権限・失効容易・audio-lens の監査済み手順と同型・期限管理なし | 各 repo admin の一次設定が必要・鍵ローテーションは手動 | yes |
| fine-grained PAT（Contents: Read on 3 repos） | 1 credential で checkout/依存 fetch を完結・HTTPS で SSH 不要 | 個人紐付け・有効期限管理・URL 埋め込み時の漏洩面 | no（代替手順として記録） |
| GitHub App + `actions/create-github-app-token` | 短命 token・最高の監査性 | App 作成/インストールの初期コスト・third-party action の信頼辺追加 | later（監査要件次第） |
| `cargo vendor`/submodule で依存ソース複製 | ビルド時認証不要 | 二重管理・license 追跡・同期コスト（audio-lens 設計と同じ理由で不採用） | no |

候補 ruviform revision への切替機構（Consumer scratch checkout 内で検証）:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| manifest `rev` 一括書換 + `cargo fetch`/`update` で lockfile 再解決 | Consumer 追従更新 PR と同一経路をそのまま検証（manifest diff + lockfile diff が出力になる）・実測済み | scratch checkout 内のファイル編集が必要 | yes |
| `[patch]` でローカル checkout へ切替 | manifest 非改変・push 前の ruviform 候補でも評価可能 | git-dep の fetch/lockfile 経路を検証できない（実運用経路と異なる） | 補助用途のみ |
| `branch = "main"` 化 + 追従 | 追従が lockfile 更新のみで完結 | Consumer の依存契約を変える（AL#127 の判断領域） | no（Consumer 側判断） |

Cross-Repo 検証の配置:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 独立 workflow（`consumer-verify.yml`・`workflow_dispatch`） | required checks を汚染しない・credential 未設定期に全 PR を壊さない・lane 別責務の既存 convention（`l1-real-verify` 等）と同型 | run 起動が手動 | yes |
| `ci.yml` に job 追加 | 既存 workflow に集約 | required check 化で Consumer 不具合/未設定が ruviform merge を止める＝Issue ルール6（非依存タスクの一律停止禁止）に抵触 | no |
| repository_dispatch/schedule で自動化 | 変更即時検出 | 実測前の自動化禁止（Issue 明示） | deferred（実測後判断） |

## Tentative Conclusions and Hypotheses

**Confirmed（実測確定）:**

- Consumer manifest の `rev` を候補 SHA へ一括書換 → `cargo fetch` で全 ruviform crate が**単一候補 SHA**（`41772a7af35e42a2380c45dade841b80c0d00d70`）に再解決され、lockfile が `git+...?rev=<sha>#<sha>` で記録される。private HTTPS fetch は system git + Git Credential Manager 経由で成功（`net.git-fetch-with-cli`・`GIT_LFS_SKIP_SMUDGE` が clone 済み `.cargo/config.toml` から効く）。機構は `## Evidence` のコマンドで再現可能。
- ruviform 側 credential（secrets/deploy keys）は現状ゼロ。Cross-Repo CI の初回実走には owner による一次設定（Deploy Key 登録 + secret 設定）が前提となり、**未設定時は CI 側が `not run` を明確に返す**設計が要る（`unknown` を `pass` と混同しないルールと整合）。
- Synvelis は buildable な対象を持たないため、Cross-Repo Gate は「Consumer 別の証拠形式」を持つ必要がある（build 系 lane と contract/設計参照系 lane で記録形式が異なる）。

**Hypothesis（確定は設計時）:**

- 影響判定（`impacted`/`no impact`/`unknown`）は PR テンプレート必須欄 + Consumer Impact Issue テンプレートで運用定着できる。`unknown` は判定が要件を満たさない限り消えない欄として残す。
- Cross-Repo CI v1 は ubuntu-latest 単発の `workflow_dispatch` が実装・実測に十分で、3 OS matrix・自動 trigger・required check 化は初回実測後に判断する（Issue の指示どおり）。

## Open Questions

- credential 一次設定の実行者（owner/admin）とタイミング — Cross-Repo CI の初回実走ブロッカー。手順は design で確定し Issue 起票案として報告。
- audio-lens #127 の `rev` → `branch` 追従形への移行後、override 機構の更新（`branch` 形では `cargo update -p <crate>` が追従経路になる）。現時点は rev-pin 前提で設計。
- audio-lens `verify-toolchain`/`check-cli-headless` 等の lane を Cross-Repo gate でどこまで再利用するか（v1 は `check`/`test` + headless guard まで、Golden/`baseline` 計測は別 lane 候補）。
- M0〜M7 の Consumer 証拠マトリクスの置き場所（Issue/PR 記録 + CI summary + 追跡文書の住み分け）。

## Promotion Candidates

- [x] spec-doc needed: 互換契約（影響判定・証拠要件・Gate 受入）と Cross-Repo CI の要件・受入基準。
- [x] adr-doc needed: credential 方式・候補 revision 切替機構・workflow 配置・trigger 方針の決定記録。
- [ ] issue needed（Consumer 側・起票案のみ報告）: audio-lens の `RUVIFORM_DEPLOY_KEY` 一次設定・synvelis の将来 buildable 化時の gate 接続。

## Sources and Relations

- 主 Issue: [ruviform#219](https://github.com/scarecrow173/ruviform/issues/219)。上位: [#192](https://github.com/scarecrow173/ruviform/issues/192)・[#193](https://github.com/scarecrow173/ruviform/issues/193)（Cross-Consumer 受入 Gate 追加条件）。Consumer 追従: [synvelis#28](https://github.com/scarecrow173/synvelis/issues/28)・[audio-lens#127](https://github.com/scarecrow173/audio-lens/issues/127)。
- 棚卸し一次資料: audio-lens `Cargo.toml`/`Cargo.lock`/`.cargo/config.toml`/`mise.toml`/`.github/workflows/ci.yml`/`docs/designs/ruviform-dependency-ci.md`/`docs/guides/ruviform-private-access-ci.md`、synvelis `mise.toml`/`docs/`、`gh` API（secrets/keys/branch protection/run list）。
- 先行 baseline: [docs/discovery/issue-194-m0-baseline-audit.md](issue-194-m0-baseline-audit.md)（ruviform 単体側 M0 実測。本稿は Consumer 側の棚卸し）。

## Evidence

### 2026-11-02: 棚卸し実測記録（Windows 11）

**環境:** ruviform worktree `rf219-cross-repo-ci`（branch `scarecrow173/rf219-cross-repo-ci`）、Git Bash、gh CLI 認証済み、`mise exec` 経由の rust 1.99.0。Consumer は `F:/repositry/audio-products/{audio-lens,synvelis}` のローカル checkout と GitHub API。

#### Consumer 棚卸し

| 対象 | 観測値 |
| --- | --- |
| audio-lens HEAD | `1a434101c3952169a8643be8b210606cc57d4dce`（2026-11-01・PR #126 merge） |
| audio-lens ruviform dep | `git` + `rev = "080c228d197ebc645b8f81cd0082be442abaf10c"`×14 entry（[workspace.dependencies]、コメントで同一 SHA 規約を明記） |
| audio-lens lockfile | `git+https://github.com/scarecrow173/ruviform.git?rev=080c228…#080c228…` が 15 crate（+transitive `ruviform-rt`）で単一 source |
| audio-lens 非 default feature | `ruviform-io-symphonia` `["mp4","aac"]` |
| audio-lens `.cargo/config.toml`（tracked） | `net.git-fetch-with-cli = true` / `GIT_LFS_SKIP_SMUDGE = "1"` |
| audio-lens CI | `workflow_dispatch` 専用・3 OS・`RUVIFORM_DEPLOY_KEY` 必須 step あり |
| audio-lens mise pin | rust 1.99.0(fmt,clippy) / node 26.12.1 / uv 0.13.5 / apm 0.33.0 / prek 0.5.7 / biome 2.6.2 / git-lfs 3.8.0 / actionlint 1.7.12 |
| synvelis HEAD | `041f9b7`（docs scaffold merge） |
| synvelis Cargo/CI | `Cargo.toml` なし・`.github/` なし・Rust toolchain 未 pin |
| 遅れ | `git rev-list --count 080c228..origin/main` = **57 commits**（`080c228` は `origin/main` の ancestor・rev pin は到達可能） |

#### GitHub 側権限・保護

| API | 結果 |
| --- | --- |
| `gh secret list --repo scarecrow173/ruviform` | 空 |
| `gh secret list --repo scarecrow173/audio-lens` | 空（`RUVIFORM_DEPLOY_KEY` 未設定） |
| `gh api repos/scarecrow173/ruviform/keys` | 空（Deploy Key 0 件） |
| `gh api repos/scarecrow173/ruviform/branches/main/protection` | 設定済み・required checks 12 件（verify×3/l1-real-verify×3/ui-verify×3/rt-verify/icc-verify/bench） |
| `gh api .../audio-lens/branches/main/protection` | 404 未設定 |
| `gh api .../synvelis/branches/main/protection` | 404 未設定 |
| `gh run list --repo scarecrow173/ruviform` | main 最新 run `41343442936` success（2026-11-01） |
| `gh run list --repo scarecrow173/audio-lens` | 0 件 |

#### 候補 revision 切替の実測（追従更新 PR と同一経路）

```sh
git clone https://github.com/scarecrow173/audio-lens.git <scratch>/audio-lens   # HEAD 1a43410
# Cargo.toml の 14 entry: rev = "080c228…" → "41772a7af35e42a2380c45dade841b80c0d00d70"（ruviform main）
mise exec -- cargo fetch        # private HTTPS fetch: system git + Git Credential Manager
# → 15 ruviform crate 全て rev=41772a7a…#41772a7a… に再解決（`Adding ruviform-*` 出力・単一 SHA）
grep -o 'rev=[a-f0-9]*#[a-f0-9]*' Cargo.lock | sort -u
# → rev=41772a7af35e42a2380c45dade841b80c0d00d70#41772a7af35e42a2380c45dade841b80c0d00d70
mise exec -- cargo check --workspace --all-targets   # exit 0・25.78s
#   → 全 15 ruviform crate が rev=41772a7a で check され、audio-lens
#     全 member（al-core/al-job/al-cli/al-media/al-dataset/al-locate/
#     al-analysis/al-service/al-gui/al-testkit/al-observability）が
#     候補 SHA で型チェック通過。symphonia 0.6.1 は io-symphonia 由来の
#     単一版、al 側の symphonia 0.5.5 直接依存はこの resolve では混在なし。
```

補足: `cargo check` は型チェックのみ（test/golden は未実行）。追従更新 PR の「manifest diff + lockfile diff + resolved SHA」が検証出力として再現できることを実測確認したが、`cargo test` や `mise run check-cli-headless`/`baseline` 系の回帰 lane は本実測では未実施（Cross-Repo lane の実走領域）。

#### M0〜M7 Consumer 別受入証拠・初期状態（2026-11-02 時点）

| Gate | 主対象 | Ruviform 単体証拠 | Synvelis 証拠 | AudioLens 証拠 |
| --- | --- | --- | --- | --- |
| M0 / G0 | 28 crate baseline | 済（#194/#200・本稿が Consumer 側補完） | N/A（workspace 未存在） | 移行 baseline 資産あり（#98・`baseline/`）。rev-pin の現行依存形態を本稿で記録 |
| M1 / G1 | Typed Graph IR | 未着手 | not run | not run |
| M2 / G2 | Audio Runtime/RT 所有 | 未着手 | not run | not run |
| M3 / G3 | Time/PPQN/Transport | 未着手 | not run | not run |
| M4 / G4 | Plugin | 未着手 | not run | not run |
| M5 / G5 | Device/Reader/Prefetch | 未着手 | not run | not run |
| M6 / G6 | WAV/Offline/Export | 未着手 | not run | not run |
| M7 / G7 | Multiwindow/Cutover | 未着手 | not run | not run |

未実測の見える化: Cross-Repo CI の初回実走（credential 設定待ち）、audio-lens 側の Golden/`baseline` 系 lane、3 OS での Consumer 検証、synvelis 側の buildable 化後の全 Gate。

#### 実機証跡の保管と検証限界

生の `cargo fetch` ログ・gh API 応答はローカルユーザー名パス等を含み得るため公開成果物には含めない。本稿の集計値は当該環境での 1 回の実測記録であり、再検証は同一手順の再実行で行う。実測 worker: Orca dispatched `task_8d72b0eeb9f6`/`ctx_cc43040cc288`。

### 2026-11-03: Issue 追記（UI 横断ゲート）反映と merged 変更の Consumer 影響分類

**追記対応:** Issue #219 本体の 2026-11-03 追記（UI Component System 横断ゲート）を spec `issue-219-consumer-compat-contract.md` に R10/AC11 として反映。追記の Consumer 側受入 Issue は全て OPEN を確認: [Synvelis #47 controls](https://github.com/scarecrow173/synvelis/issues/47)・[#48 viz](https://github.com/scarecrow173/synvelis/issues/48)・[AudioLens #131 controls](https://github.com/scarecrow173/audio-lens/issues/131)・[#132 viz](https://github.com/scarecrow173/audio-lens/issues/132)。`consumer-verify` 情報レーンは追記どおり required check に昇格していない（`workflow_dispatch` 専用のまま・required checks 12 件は不変）。

**Consumer 棚卸し差分（2026-11-03 実測）:** 2026-11-02 棚卸し以後に synvelis main が workspace 化へ前進（`041f9b7` → `f7aadd09cf55ecbe4ca6f3cc73cb2d80459769e7`、2026-11-02 12:56 JST・E2E-X PR #35 merge）。分類根拠を実測値へ更新する。

| 対象 | 観測値 |
| --- | --- |
| synvelis workspace | `f7aadd0` で 5 member workspace（synvelis-project / synvelis-recording / recording-poc / synvelis-player / synvelis-render）。synvelis AGENTS.md は ruviform 依存を「既定ブランチ追従の git 依存・rev pin なし・解決 revision は Cargo.lock に記録」と規約化 |
| synvelis ruviform dep | `[workspace.dependencies]` に `git`（rev 指定なし）×10 crate: media-core / media-io / media-reader / io-symphonia / graph / audio / audio-core / rt / io-wav / exec |
| synvelis lockfile | 15 ruviform crate（直 10 + transitive dsp / math / audio-analysis / observability / video-core）が `322a661ac0177fc98f799b07ae591c98eb2ec845`（PR #235 merge・2026-11-02 11:01 JST）に単一解決 — ruviform HEAD `ced77f0` 比 **39 commit 遅れ**・`ruviform-numeric`・`ruviform-ui` 系は非含有 |
| synvelis 分類への含意 | branch 追従依存のため「次回 lockfile re-resolve で ruviform main の変更を取り込む」。「workspace 未存在」による一律 `no impact` は本日時点で成立しない — lockfile の crate 含有・ソース利用面で個別判定する |

**直近 merged 変更の Consumer 影響分類（post-merge 記録）:** PR 本文への分類記入がなかった merged 変更について、Issue ルール2/R1/R10 に基づき分類・根拠・追跡先を記録する。`unknown` 残置は互換性確認済みではない。

| PR / Issue | 変更種別 | Synvelis | AudioLens | 根拠・追跡先 |
| --- | --- | --- | --- | --- |
| #256（#255 mixed-radix FFT 変換長選択） | behavior / 性能差分 | no impact（lock に `ruviform-numeric` 非含有・ソースに numeric/corr/fft 利用なし） | **impacted** — audio-lens [#133](https://github.com/scarecrow173/audio-lens/issues/133) で e2e +10.2%・locate +17.1% 回帰を実観測（当時 pin）。ruviform#265 で修正、現行 pin `b9f2a10` に #256・#266 の双方を同梱。consumer 側再検証は PR [#134](https://github.com/scarecrow173/audio-lens/pull/134)・追従運用は [#127](https://github.com/scarecrow173/audio-lens/issues/127) |
| #266（#265 CorrelationPlan コストモデル校正） | behavior / 性能差分 | no impact（同上 — `CorrelationPlan` は `ruviform-numeric` にのみ存在） | **impacted** — #133 回帰の upstream 修正。ruviform 側ベンチで consumer baseline と一致（回帰形状 134.95ms vs consumer 実測 134.38ms・IR `issue-265-correlationplan-prefers-direct-mixed-radix-direct.md`）。consumer pin `b9f2a10`（本 PR merge commit）へ更新済み |
| #267（#258 `ui/core` headless 契約） | feature / crate 構成 | **impacted** — UI 共通基盤の新規採用面（変更は `ui/` TS パッケージのみ・pin 中 Rust crate は無変更）。制御/Transport/RT 非阻害の consumer 側評価は [#47](https://github.com/scarecrow173/synvelis/issues/47) で追跡（未実測） | **impacted** — 同上。f64・NaN 欠測・Validity/Provenance の評価は [#131](https://github.com/scarecrow173/audio-lens/issues/131) で追跡（未実測） |
| #268（#259 `ui/components`） | feature / crate 構成 | **impacted** — #47 で追跡（未実測） | **impacted** — #131 で追跡（未実測） |
| #269（#260 `ui/viz`） | feature / crate 構成 | **impacted** — viz 系受入は [#48](https://github.com/scarecrow173/synvelis/issues/48) で追跡（未実測） | **impacted** — bounded Dataset 可視化の受入は [#132](https://github.com/scarecrow173/audio-lens/issues/132) で追跡（未実測） |
| #270（#261 `ui/binding`） | feature / crate 構成 | **impacted** — #47 で追跡（未実測）。IR 未作成は別途フォローアップ | **impacted** — #131 で追跡（未実測） |
| #271（#263 `ui/showcase`） | feature / crate 構成 | **impacted** — IR `issue-263-…` が Consumer A shape `impacted` を記録済み。consumer 側評価は #47/#48 で追跡（未実測） | **impacted** — IR が Consumer B shape `impacted` を記録済み。#131/#132 で追跡（未実測） |
| #254（#249 reader End at EOF） | behavior / 契約 | **impacted（positive・対応不要）** — `synvelis-player/src/driver.rs` の manual `Prefetch` driver を IR `issue-249-prefetch-detection-ended-strands-end-marker.md` が実在利用として検証済み（現 main workspace 同梱）。lock `322a661` は #254 前のため次回 re-resolve で取込。回帰追跡は [#28](https://github.com/scarecrow173/synvelis/issues/28) | no impact — `ruviform-media-reader` 非依存（直 14 crate + transitive `ruviform-rt` のみ・issue-249 IR が local worktree で検証し #244 の `unknown` を解消）。pin `b9f2a10` に #254 は同梱されるが依存面に及ばない |
| #224（#219 本 Issue の基盤整備: workflow + テンプレート） | CI 基盤 | no impact（ruviform 側情報 lane・テンプレート追加のみ。Consumer 依存・API 不変） | no impact（同上） |

分類語彙: `impacted` = 消費中 dependency surface（consumer lockfile に含まれる crate）に変更が及ぶ、または採用・受入評価の対象となる新規契約面が追加された（後者の consumer 側実測は #47/#48/#131/#132 の未実測項目として残る）。`no impact` = 依存面にも採用評価にも及ばない。`unknown` = 未検証であり互換性確認済みとはしない。

**追従更新 PR ループの実績（Issue チェックリスト 4 項の初回実績）:** audio-lens 側で pin 追従 PR が手動運用で 2 回実走している。PR [#128](https://github.com/scarecrow173/audio-lens/pull/128)（pin → `7c430212`・consumer 回帰検証付き）と PR [#134](https://github.com/scarecrow173/audio-lens/pull/134)（pin → `258df5a` → 回帰検出で merge 保留 → ruviform#265 修正後 `b9f2a10` へ再 bump・golden 再生成して 2026-11-02 merge）。#134 は「manifest/lockfile 更新 + golden 5 run 0-diff + 回帰検出 → upstream 修正待ち → 修正後再検証」の不適合切り分け〜解消ループを 1 回完走しており、Issue ルール5/チェックリスト5項の実績として記録する。現行 pin `b9f2a10` は ruviform main `ced77f0` から **12 commit 遅れ**（`git rev-list --count b9f2a10..main` 実測 — 未取込は ui/* merge 5 件 #267〜#271 と配下 feature commit。`b9f2a10` より前に merge された #254・#256・#266 は pin に同梱済み）。

**検証状態の正直な区分（2026-11-03 時点）:**

- **verified（ruviform 側）**: `consumer-verify.yml` の actionlint 合格・構造上の gating/guard/真偽表（設計実装済み）。ui/* headless lane（`check/test-ui-*`）は required check `ui-verify` 経由で green。#265 修正は ruviform bench + consumer baseline 一致で確認。
- **verified（consumer 側・手動経路）**: audio-lens の pin 追従 PR #128/#134 が golden + perf 検証で実走済み（cross-repo workflow 経由ではない）。Synvelis は `f7aadd0` で buildable workspace 化済みだが、本 dispatch では consumer repo 非変更のため build/test 実測は未実施（cross-repo lane も未 dispatch）。
- **informational（未稼働）**: `consumer-verify.yml` は `CROSS_REPO_DEPLOY_KEY`・3 repo への read-only Deploy Key 登録が未設定のため dispatch 実走 0 件。dispatch 時は全 lane `not run` → workflow 失敗（証拠ゼロを緑にしない設計）。
- **not-yet-required（意図的に未昇格）**: consumer-verify の required check 化・自動 trigger は spec R9/Deferred と Issue 追記どおり未実施・未要求。
- **not verified（既知の欠落）**: audio-lens `test-ci`/`check-cli-headless`/`git lfs pull` の CI 経路実走、Synvelis 側の全 consumer 検証（workspace は存在・未実測）、ui/* の consumer 側受入評価（#47/#48/#131/#132）、Golden/baseline 計測 lane の cross-repo 化。

**M0〜M7 Consumer 別受入証拠（2026-11-03 更新）:** append-only 運用のため 2026-11-02 表は改変せず、本 snapshot に差分後の表を追記する。

| Gate | 主対象 | Ruviform 単体証拠 | Synvelis 証拠 | AudioLens 証拠 |
| --- | --- | --- | --- | --- |
| M0 / G0 | 28 crate baseline | 済（#194/#200・本稿が Consumer 側補完） | workspace 化済み（`f7aadd0`・lock `322a661`・main 追従依存）— consumer 側 build/test は未実施 | 移行 baseline 資産（#98・`baseline/`）に加え、pin 追従 PR [#128](https://github.com/scarecrow173/audio-lens/pull/128)/[#134](https://github.com/scarecrow173/audio-lens/pull/134) で consumer 回帰検証ループ完走（golden 5 run 0-diff・#133 回帰検出→ruviform#265 修正→`b9f2a10` 再 bump）。現行 pin `b9f2a10` |
| M1 / G1 | Typed Graph IR | 未着手 | not run（`ruviform-graph` を dep に含む・未実測） | not run |
| M2 / G2 | Audio Runtime/RT 所有 | 未着手 | not run（`ruviform-rt`/`ruviform-audio` を dep に含む・未実測） | not run |
| M3 / G3 | Time/PPQN/Transport | 未着手 | not run | not run |
| M4 / G4 | Plugin | 未着手 | not run | not run |
| M5 / G5 | Device/Reader/Prefetch | 未着手 | `synvelis-player` が `ruviform-media-reader`/`Prefetch` を実利用（IR `issue-249-…` が #254 semantics を検証・positive）— consumer 側回帰 lane 未接続 | `ruviform-media-reader` 非依存のため #254 は no impact 確定（issue-249 IR・#244 `unknown` 解消済み） |
| M6 / G6 | WAV/Offline/Export | 未着手 | `synvelis-render` が sequential offline WAV export を実装（`f7aadd0`・PR #34/#35・`ruviform-io-wav`/`ruviform-exec` 利用）— ruviform 側との互換 lane 未接続 | not run |
| M7 / G7 | Multiwindow/Cutover | ui/* merged（#258〜#263・TS パッケージ） | 採用評価 Issue 起票済み・未実測（#47/#48） | 採用評価 Issue 起票済み・未実測（#131/#132） |

### 2026-11-03（追記 2 件目）: Cross-Repo Gate 本体実装 — synvelis 実 lane・PR 分類機械検査・smoke 整合

**実施内容:** `consumer-verify.yml` の `verify-synvelis` を presence check stub から実 lane へ拡張（synvelis `f7aadd0` の workspace 化を受けた設計追記の実装）。manifest 正規化を `rev` pin・`branch` 追従・`rev` 未指定の 3 形態対応へ一般化して両 lane 統一、audio-lens smoke に `cargo build -p al-cli --profile ci` を追加（spec R5 整合）、`.github/workflows/consumer-impact-check.yml` を新設（PR 本文の分類欄を機械検査する非 required lane）。

**ローカル実測（credential 非依存区間・2026-11-03）:** synvelis `f7aadd0` の scratch clone（`git clone --depth 1`）で候補 `a014b3f941dfc21cb349504c1383b65548730381`（ruviform `origin/main`）に対し lane 手順を再現実行:

| 手順 | 結果 |
| --- | --- |
| manifest 正規化（`rev` 未指定 10 dep → `rev` 注入） | 全 10 行が `rev = "a014b3f9…"` に書換・`git diff` で可視化 |
| `CARGO_NET_GIT_FETCH_WITH_CLI=true` + `GIT_LFS_SKIP_SMUDGE=1` で `cargo fetch` | 15 ruviform crate が `?rev=a014b3f9…#a014b3f9` に単一解決 |
| 単一 SHA guard（repo URL キー） | total=15・bad=0 |
| `mise run check` | exit 0（dev profile・10.66s） |
| `git lfs pull` + `mise run test`（`--locked`） | exit 0・23 test suite・0 failed |
| audio-lens `987f236` scratch での manifest 正規化 + `cargo metadata` 再解決 | 14 dep の `rev` が候補へ置換（`features` 保持）・lock 15 source 全て `?rev=<候補>#<候補>`（total=15・bad=0） |
| consumer-impact-check 検査スクリプト | 5 case（未記入・記入済み・unknown+解消先なし・unknown+解消先あり・セクション欠落）をローカル実行し pass/fail が全て意図どおり |

**検証状態の正直な区分（本追記時点）:**

- **verified**: synvelis lane の credential 非依存区間（manifest 正規化・fetch・guard・check・test）をローカル実測。audio-lens lane の manifest 正規化 + lockfile 再解決も実測。両 workflow の actionlint 合格。
- **informational（未稼働）**: dispatch 実走は `CROSS_REPO_DEPLOY_KEY` 一次設定待ちで 0 件（SSH/known_hosts pin・`url.insteadOf`・証拠表出力の runner 経路は未実走）。consumer-impact-check は PR への初回起動が未発生（次回 PR で観測）。
- **not verified**: audio-lens 側 `check-ci`/`test-ci`/`check-cli-headless`/`git lfs pull` の lane 実行（初回 dispatch で検証）、synvelis lane の runner 上実行、全 lane not run → 失敗の真偽表の実 dispatch 観測。

**分類（本変更自身）:** Synvelis = impacted（本 lane の検証対象として実 lane 化・consumer 側変更なし・追従運用は synvelis#28）/ AudioLens = impacted（smoke lane に al-cli build 追加・同・audio-lens#127）。両者とも consumer repo への恒久的変更を含まない scratch 検証経路の拡張であり、依存 pin・API・行為規約は不変。

### 2026-11-03（追記 3 件目）: repo public 化の観測・棚卸し refresh・access mode fallback 実装と lane ローカル実走

**前提変化（visibility）:** 設計時の「3 repo とも private」前提が変化した。`gh api` 実測で `scarecrow173/ruviform`・`scarecrow173/audio-lens`・`scarecrow173/synvelis` すべて `visibility: public`、匿名 `git ls-remote`（HTTPS・credential なし）が 3 repo で成功。audio-lens `Cargo.toml` コメントは「公開方針確定前の暫定」と記すため再 private 化の可能性は残る。`gh secret list`（ruviform/audio-lens）は引き続き空、`gh api repos/ruviform/keys` = 0 件で Deploy Key 一次設定は未実施のまま。

**access mode fallback（consumer-verify.yml への実装・本 PR の中核）:** secret 未設定 + 匿名 HTTPS read 可能な repo に対し `anonymous` mode で lane を実行する fallback を追加（design 2026-11-03 追記 2 件目・task `issue-219-t5-anonymous-access.md`）。`deploy_key` mode は private 前提の canonical 経路として維持し secret 設定時は優先。anonymous mode は秘密を一切持たず（最小権限の下限）、probe は consumer repo 単位（R7 の独立性維持）、mode は証拠表 `access:` に記録。全 lane `not run` → workflow 失敗の既定は不変。

**Consumer 棚卸し差分（2026-11-03 再実測・前回 snapshot からの drift）:**

| 対象 | 観測値 |
| --- | --- |
| audio-lens HEAD | `10c4fc797e9d44b5475a61fbbc89afb2cd8dbc5b` |
| audio-lens ruviform dep | `git` + `rev = "e9e6abe514b8f3f0bfff2d976b8ca6551955ca09"` ×**16** entry（#128/#134 以後も pin は前進し現行は ruviform HEAD と一致。追加された dep: `ruviform-ui`/`ruviform-ui-webview`/`ruviform-viz`/`ruviform-numeric`/`ruviform-io-avi` — ui 採用が lockfile 上の依存面にも反映）。`rev` pin 形態は維持（#127 の branch 追従移行は未実施） |
| audio-lens lockfile | 15 ruviform crate が `?rev=e9e6abe…#e9e6abe` に単一解決（前回 `b9f2a10` から前進・12 commit 遅れは解消） |
| audio-lens 非 default feature | 不変（`ruviform-io-symphonia` `["mp4","aac"]` のみ） |
| audio-lens CI | `ci.yml` は `workflow_dispatch` 専用・3 OS のまま。`RUVIFORM_DEPLOY_KEY` 必須 step は private 前提の記述のまま残存（repo は public — consumer 側対応候補として指摘） |
| audio-lens mise tasks | `check-ci`/`test-ci`/`check-cli-headless`/`verify-toolchain`/`baseline` のほか `check-ui`/`test-ui`/`check-ui-webview` 等の ui lane が追加 |
| synvelis HEAD | `5adc4d24578d94aa845a08cc623d79b5f669fcbf` |
| synvelis workspace | 5 → **8 member**（+ `synvelis-workspace`/`synvelis-gateway`/`mcp-poc`。gw10 ゲートハーネス task 群あり） |
| synvelis ruviform dep | `git`（rev 未指定）×**13** crate（+ `ruviform-ui`/`ruviform-plugin-core`/`ruviform-plugin-host`/`ruviform-audio-plugin-bridge`） |
| synvelis lockfile | 19 ruviform crate が `258df5ae255f812f54461eb1aef30fce608e8c6d` に単一解決（branch 追従・rev param なし）。`258df5a` は audio-lens #134 で回帰を検出した pre-#265 revision — synvelis の lock が ruviform main から遅れている既知状態として記録 |
| synvelis toolchain | mise.toml に `rust = "1.99.0"` pin 追加済み（前回「未 pin」から解消）・`check`/`test`/`build`/`lint`/`fmt`/`gw10` task。`.cargo/config.toml`・`.github/` は依然非存在 |
| 両 repo LFS | `git lfs pull` は匿名 HTTPS で public repo から実体取得可能（audio-lens lane 実走で確認） |

**lane ローカル実走（anonymous 経路と同一の HTTPS・候補 `e9e6abe514b8f3f0bfff2d976b8ca6551955ca09` = ruviform main HEAD・scratch clone・Windows 11 Git Bash）:** workflow の lane 手順を両 consumer 現行 main で再現実行。

| lane | 手順 | 結果 |
| --- | --- | --- |
| synvelis | manifest 正規化（13 dep rev 注入）→ `cargo fetch` → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test` | guard total=19・bad=0。check exit 0（dev profile 18.08s）。test exit 0（test profile 1m07s・63 result line 全て 0 failed） |
| audio-lens | manifest 正規化 → `cargo fetch` → guard → `mise run check-ci` → `cargo build -p al-cli --profile ci` → `check-cli-headless` → `git lfs pull` → `mise run test-ci` | guard total=15・bad=0（pin が既に候補 SHA のため manifest diff は io-symphonia 行の rev 位置正規化のみ）。check-ci exit 0（36.98s）。smoke build exit 0（23.19s）。`check-cli-headless`: no GUI framework deps in 10 headless crates。LFS 実体取得後 test-ci exit 0（46 result line 全て 0 failed） |

ローカル再現の差分（runner 条件との明示的な違い）: audio-lens の `mise install` は runner では repo `mise.toml` のみを見るが本ローカルは global mise config に lockfile 未収録 tool があり `--locked` が失敗したため `rust git-lfs node` へ scope。lane 手順そのものは workflow と同一。

**実 dispatch 証拠（anonymous mode・PR #274 ブランチ ref・候補 `e9e6abe`）:**

- run [41604118237](https://github.com/scarecrow173/ruviform/actions/runs/41604118237): verify-synvelis 全緑（1m42s）・verify-audiolens が fetch step で `mise install --locked` 失敗（audio-lens `mise.lock` の `actionlint`/`pipx:serena-agent` が platform entry を持たず `--locked` は全 OS で復元不可 — consumer 側 lockfile gap）。証拠表は fetch=failure・下流 lane=skipped を正しく記録。audio-lens lane の install を lane 必要 tool（rust/git-lfs/node）へ scope して修正（synvelis lane と同型）。
- run [41604498852](https://github.com/scarecrow173/ruviform/actions/runs/41604498852): verify-synvelis 全緑（1m14s・anonymous HTTPS clone/probe/pin/fetch+guard/check/test 全 step）。verify-audiolens は check-ci・smoke 2 lane（`al-cli` ci build + `check-cli-headless`）を通過し `test-ci` で **3 test fail** を検出（detection は gate の意図どおり・workflow は fail を正直に報告し summary も fail）。
- **AudioLens 検出事項（起票案）**: `al-cli --test cli` の export 3 test（`full_pipeline_reaches_export`・`export_command_packages_a_job_without_export_in_its_plan`・`compressed_only_mp4_runs_the_default_pipeline_end_to_end`）が ubuntu-latest で `export file analysis/inputs/IN-*.metadata.json still embeds absolute path /tmp/…` で fail。機序: `al-job` が input path を `std::path::absolute` で manifest `source_path`/`source_path_abs` に書き込み（job.rs:60・manifest.rs:90-91）、export portability gate（al-dataset `export/package.rs`）は forbidden 側のみ `\`→`/` 正規化するため Windows では JSON 内の `\\` 区切りと一致せず blind、Linux で初めて検出可能になった潜在 portibility 欠陥。ruviform 起因ではない（pin は既に候補 SHA と一致・consumer 側テスト資産自身の問題）。consumer 側対応（package 内 metadata の source path 相対化/除去・gate の両 separator 照合）が必要 — audio-lens #127 系の追従 Issue として起票案を提示。
- **consumer-impact-check**: PR #274 で初回実トリガー・`classification` check pass（3s）— PR 本文分類欄の機械検査が実稼働することを確認。
- **runner 実測タイミング補足**: `ubuntu-latest` で synvelis lane は ~1.2-1.7 分（cargo cache 温間時）。audio-lens lane は mise install scoped 化で fetch+guard+check-ci+smoke まで ~2.5 分で到達（test-ci fail まで 2m34s・全緑想定でも数分台と見込まれる）。

**検証状態の正直な区分（本追記時点）:**

- **verified（lane 手順・anonymous 経路相当）**: 両 consumer 現行 main で clone → 正規化 → fetch → guard → check/test/smoke まで全手順をローカル完走（上表）。`consumer-verify.yml` 変更後の actionlint pass。manifest 正規化が audio-lens 現行 16 dep 形態（features 付き行を含む）でも guard まで到達することを実測。
- **verified（runner・anonymous mode）**: 実 dispatch 2 run で synvelis lane 連続 green、audio-lens lane が fetch scope 修正後に test-ci まで到達（fail は consumer 側潜在欠陥の検出であり gate の誤検出ではない）。全 lane not run → 失敗の真偽表は run 41604118237（片 lane fail → summary fail）で一部観測。証拠表の `access:` 記録・skipped/failure 区分を runner で確認。
- **informational**: `deploy_key` mode の SSH 経路（credential 未設定・構造は維持）、全 lane not run → 失敗の実 dispatch 観測（片 lane fail 済み・全 not run ケースは未観測）、audio-lens `RUVIFORM_DEPLOY_KEY` 依存の consumer 側 ci.yml（private 前提記述が残存・consumer 側の責務）。
- **not verified**: Golden/`baseline` 系 lane（v1 スコープ外・引き続き Deferred）、3 OS matrix、synvelis `gw10` lane の cross-repo 接続、audio-lens export 3 test の修正後再検証（consumer 側対応待ち）。

**分類（本変更自身）:** Synvelis = no impact・AudioLens = no impact（ruviform 側 CI 基盤のみの変更。consumer repo への変更なし・依存 pin・API・行為規約は不変。#224 の基盤整備と同じ分類根拠）。実測 worker: Orca dispatched `task_91078cf22169`/`ctx_9a2208a14504`。
