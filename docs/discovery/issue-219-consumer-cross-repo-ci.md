---
id: "DISC-034QdBDFXQ7CpGCCYpN94k"
type: "discovery"
status: "draft"
title: "Issue #219 両 Consumer 依存形態・検証基盤棚卸しと Cross-Repo CI 前提実測"
created: "2026-11-02"
updated: "2026-11-02"
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
