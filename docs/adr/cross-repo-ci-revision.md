---
id: "ADR-034QdDEaffrclLniNBLhQe"
type: "adr"
status: "proposed"
title: "Cross-Repo CI アクセス認証と候補revision検証方式"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
  changes:
    added:
      - type: "workflow"
        path: ".github/workflows/consumer-verify.yml"
    modified:
      - type: "doc"
        path: "AGENTS.md"
      - type: "template"
        path: ".github/PULL_REQUEST_TEMPLATE.md"
      - type: "template"
        path: ".github/ISSUE_TEMPLATE/"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-219-consumer-compat-contract.md"
  implemented-by: []
  depends-on:
    - "../specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-219-consumer-compat-contract.md"
    - "../discovery/issue-219-consumer-cross-repo-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/199"
    - "https://github.com/scarecrow173/ruviform/issues/201"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/guides/ruviform-private-access-ci.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Cross-Repo CI アクセス認証と候補revision検証方式

## Context and Problem Statement

Ruviform 候補 revision に対して Synvelis・AudioLens 両 Consumer の互換性を検証する Cross-Repo CI を実装する。3 repo とも private であり、Ruviform の GitHub Actions から (a) Consumer リポジトリを checkout し、(b) Consumer workspace 内で private ruviform Git 依存を取得する、という 2 段のアクセスを最小権限で実現する必要がある。同時に、Consumer の依存 manifest（`git + rev` 固定）を候補 revision に切り替える機構と、workflow の配置・trigger 方針を決める。現状、ruviform repo には secrets も Deploy Key も存在せず、audio-lens 側の `RUVIFORM_DEPLOY_KEY` も未設定で同 repo の CI 実走は 0 件（棚卸し確定）。

## Decision Drivers

* private credential は最小権限・取り消し容易・秘密をログ/成果物に出さない（Issue #219 チェックリスト）。
* `unknown` を互換確認済みとみなさない: credential 未設定・Consumer 未 buildable は `not run` として明示し、job 失敗とは分離する。
* Consumer 固有の非依存タスクを他 Consumer の未完で一律停止しない（ルール6）。Cross-Repo lane を required check にしない。
* 自動 trigger は初回実測後に判断（Issue 明示・G0 でも同方針）。
* 検証経路は Consumer の追従更新 PR と同一形（manifest `rev` 切替 + lockfile 再解決）で、実運用経路をそのまま検証する。
* audio-lens 側の既存設計（Deploy Key + url.insteadOf + pinned host key、third-party action 不使用）と整合させる。

## Considered Options

### アクセス認証

* **Option A: 専用 read-only Deploy Key（1 鍵ペアを 3 repo へ登録、秘密鍵は ruviform Actions secret）**
* **Option B: fine-grained PAT（Contents: Read on 3 repos）**
* **Option C: GitHub App + `actions/create-github-app-token`**
* **Option D: `cargo vendor` / submodule による依存ソース複製**

### 候補 revision への切替機構（Consumer scratch checkout 内）

* **Option 1: manifest `rev` 一括書換 + `cargo fetch` による lockfile 再解決**
* **Option 2: `[patch]` によるローカル checkout 切替**
* **Option 3: `branch = "main"` への依存形態変更 + `cargo update`**

### workflow 配置と trigger

* **Option i: 独立 workflow（`consumer-verify.yml`）+ `workflow_dispatch` のみ**
* **Option ii: `ci.yml` に job 追加**
* **Option iii: `repository_dispatch` / `schedule` による自動起動**

## Decision Outcome

Chosen options: **A + 1 + i**。

- **認証は Option A**: 専用 ed25519 鍵ペアを生成し、公開鍵を `scarecrow173/ruviform`・`scarecrow173/audio-lens`・`scarecrow173/synvelis` の 3 repo に read-only Deploy Key として登録、秘密鍵を ruviform の Actions secret `CROSS_REPO_DEPLOY_KEY` に設定する。audio-lens 側が設計上採用している方式と同型（設計は監査済みだが CI 実走は 0 件 — 初回実走は本 lane が初となる）で、個人 token に紐付かず失効が容易。Consumer checkout（SSH clone）と Consumer 内 ruviform Git 依存 fetch（`url.insteadOf` 書換 + `GIT_SSH_COMMAND`）の両方を同一 credential で賄える。
- **切替は Option 1**: Consumer scratch checkout 内で `[workspace.dependencies]` の全 ruviform `rev` を候補 SHA に一括書換し、`cargo fetch` で lockfile を再解決する。Consumer の追従更新 PR と同一経路であり、manifest diff・lockfile diff・resolved SHA の記録がそのまま証拠になる。ローカル実測で全 ruviform crate が単一候補 SHA に再解決されることを確認済み（discovery の Evidence）。
- **配置は Option i**: `consumer-verify.yml` 独立 workflow + `workflow_dispatch`。required checks（12 件）に混ぜない。Consumer 別 job は互いに独立し、片側の失敗・未設定が他 Consumer の実行や ruviform の merge を止めない。credential 未設定・Consumer 未 buildable は `not run` を summary に出力して成功扱いにしない（`unknown` ≠ `pass`）。

### Consequences

* Good, because: Deploy Key は repo 単位の最小権限で、失効・監査が明瞭。SSH 方式は audio-lens の実装済み設計と同型のため運用知見を再利用できる。manifest 書換は追従 PR と同一経路を CI で再現できる。独立 workflow は Issue ルール6（一律停止禁止）と両立する。
* Bad, because: Deploy Key の一次設定（3 repo への鍵登録 + secret 投入）は owner/admin 作業として残り、初回実走前のブロッカーになる。workflow_dispatch 手動起動の間は実走忘れで `unknown` が滞留し得る。manifest 書換は scratch checkout 内のファイル編集を伴い、Consumer manifest 形式の変更（例: `rev` → `branch`、crate 追加）に追従更新が必要。

### Confirmation

- `.github/workflows/consumer-verify.yml` の actionlint 通過と、workflow が `workflow_dispatch` で起動可能であること。
- credential 未設定の状態で実行した場合、各 Consumer job が「`not run`（credential 未設定）」を summary に出力して失敗扱いにしないこと。
- audio-lens job が rev 書換 → fetch → check の経路を通し、lockfile が単一候補 SHA に再解決されること（ローカル実測の再現）。
- 一次設定手順（Deploy Key 登録・secret 投入）が design/runbook で再現可能に記述されていること。

## Implementation Plan

* Affected paths: `.github/workflows/consumer-verify.yml`（新設）、`.github/PULL_REQUEST_TEMPLATE.md`・`.github/ISSUE_TEMPLATE/`（影響判定）、`AGENTS.md`（stale な `ci.yml` 記述の修正）、`docs/`（本 ADR・spec・design・plan・task・IR）、必要なら `ci/` 配下の補助スクリプト
* Patterns to follow: audio-lens `.github/workflows/ci.yml` の private dep 認証 step（`RUNNER_TEMP` 鍵ファイル・`/meta` の pinned host key・`GIT_SSH_COMMAND`・`url.insteadOf`・`git ls-remote` probe）、ruviform `ci.yml` の mise-action + `mise install --locked` + cache 構成
* Patterns to avoid: third-party action への秘密委譲、`secrets` 値の echo/artifact 化、Consumer job の required check 化、`unknown` の `pass` 扱い、Consumer manifest への恒久的変更（scratch checkout のみ）
* Dependencies or configuration: ruviform Actions secret `CROSS_REPO_DEPLOY_KEY`（owner 一次設定）、Consumer repo 側 checkout 用の同鍵 Deploy Key 登録、audio-lens の `.cargo/config.toml`（`net.git-fetch-with-cli`・`GIT_LFS_SKIP_SMUDGE`。既存のまま利用）

## Verification

* [ ] `mise exec -- actionlint .github/workflows/consumer-verify.yml` が pass する
* [ ] `workflow_dispatch` で run を起動し、credential 未設定時に `not run` が summary へ出て job 失敗にならない（or 明確な setup 誘導で終了する）
* [ ] credential 設定後の初回実走で、audio-lens の `check`/`test` lane が候補 SHA で実行され、job summary に Consumer 別証拠表（lane・OS・feature・resolved SHA・pass/fail/not run）が出力される
* [ ] synvelis job が `not run（workspace 未存在）` を正しく報告する
* [ ] secret が workflow ログ・artifact に出力されない（`git config` の差分に鍵・token が残らない）

## Pros and Cons of the Options

### Option A: 専用 read-only Deploy Key

* Good, because 最小権限（単一 repo ごとの read-only）・失効容易・audio-lens 側の監査済み手順と同型・有効期限管理が不要。
* Bad, because 3 repo への登録と秘密鍵の ruviform secret 投入に admin 作業が必要、鍵ローテーションは手動。

### Option B: fine-grained PAT

* Good, because 1 credential で Consumer checkout（HTTPS）と ruviform Git 依存 fetch を完結でき、SSH 設定が不要。
* Bad, because 個人アカウントに紐付き有効期限管理が必要、`x-access-token` を URL に埋め込む形のため .git/config・ログへの漏洩面がある。Deploy Key が使えない環境向けの代替手順として runbook に併記する程度に留める。

### Option C: GitHub App + `actions/create-github-app-token`

* Good, because 短命 token・最高の監査性・App 単位の権限管理。
* Bad, because App 作成・各 repo へのインストール・third-party action への秘密委譲の初期コストが大きい。将来の監査要件次第で移行可能（Deferred）。

### Option D: vendor/submodule

* Good, because ビルド時の認証が不要になる。
* Bad, because ruviform ソースの二重管理・license 表記の追跡・更新同期の責務が発生する（audio-lens 側設計と同じ理由で不採用）。

### Option 1: manifest `rev` 書換 + fetch 再解決

* Good, because Consumer 追従更新 PR と同一経路を検証でき、manifest/lockfile diff と resolved SHA がそのまま証拠になる。ローカル実測済み。
* Bad, because scratch checkout 内の編集であり、Consumer manifest 形式の変更に workflow の書換ロジックが追従する必要がある。

### Option 2: `[patch]` ローカル切替

* Good, because manifest を編集せず、push 前の ruviform 候補でも評価できる。
* Bad, because git-dep の fetch/lockfile 経路を検証できず、実運用経路（git dep 更新 PR）と異なる検証になる。

### Option 3: `branch = "main"` 形への変更

* Good, because 追従が lockfile 更新のみで完結する。
* Bad, because Consumer の依存契約を変更する（AL#127 の判断領域）。Ruviform 側の CI が Consumer manifest の形を強制しない。

### Option i: 独立 workflow + dispatch

* Good, because required checks を汚染せず、credential 未設定期や Consumer 障害時に ruviform merge を止めない。既存の lane 別責務 convention（`l1-real-verify`/`ui-verify` 等の独立 job）と同型。
* Bad, because 起動が手動で実走忘れのリスクがある（自動化は実測後の Deferred）。

### Option ii: `ci.yml` job 追加

* Good, because workflow が 1 ファイルに集約される。
* Bad, because required check 化した場合 Consumer 側の未設定・不具合が ruviform の全 merge を止め、ルール6に抵触する。

### Option iii: 自動 trigger

* Good, because 変更即時に検出できる。
* Bad, because 実測前の自動化は Issue が明示的に禁止し、credential 未稼働では全 trigger が `not run` を量産する。

## More Information

- 棚卸し・実測: [docs/discovery/issue-219-consumer-cross-repo-ci.md](../discovery/issue-219-consumer-cross-repo-ci.md)
- 仕様: [docs/specs/issue-219-consumer-compat-contract.md](../specs/issue-219-consumer-compat-contract.md)
- audio-lens 側 credential 設計・手順（同型の SSH Deploy Key 方式）: `audio-lens/docs/designs/ruviform-dependency-ci.md`・`audio-lens/docs/guides/ruviform-private-access-ci.md`
- CI 基盤（required checks 12 件・自動 trigger 済み）: ruviform #199/#201（closed）
