---
id: "DESIGN-034QdHuPP1Uvvlkvbh5awX"
type: "design"
status: "approved"
title: "Issue #219 Cross-Repo CI Gate 設計・credential 一次設定・証拠記録 runbook"
created: "2026-11-02"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
  implements:
    - "../specs/issue-219-consumer-compat-contract.md"
    - "../adr/cross-repo-ci-revision.md"
  implemented-by: []
  depends-on:
    - "../specs/issue-219-consumer-compat-contract.md"
    - "../adr/cross-repo-ci-revision.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-219-consumer-cross-repo-ci.md"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-219-consumer-compat-contract.md"
    - "../adr/cross-repo-ci-revision.md"
    - "../discovery/issue-219-consumer-cross-repo-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/guides/ruviform-private-access-ci.md"
  defers: []
  deferred-by: []
---

# Issue #219 Cross-Repo CI Gate 設計・credential 一次設定・証拠記録 runbook

## Design Overview

`.github/workflows/consumer-verify.yml` として、`workflow_dispatch` 手動起動の Cross-Repo 検証 workflow を Ruviform 側に新設する。Ruviform 候補 revision（既定: dispatch 対象 ref の SHA）に対し、Synvelis・AudioLens それぞれ独立した job で、Consumer scratch checkout 内の ruviform Git 依存を候補 revision に切り替えて `cargo fetch`/`check`/`test`/`smoke` を実行し、job summary に Consumer 別の証拠表を出力する。required checks（12 件）には含めず、片 Consumer の失敗・未検証が他 Consumer や Ruviform merge を止めない情報 lane とする。credential が未設定でも `not run` を明示して終了し、`unknown` を `pass` に見せない。

本設計は ADR `cross-repo-ci-revision.md` の決定（Option A: read-only Deploy Key / Option 1: manifest rev 書換 + lockfile 再解決 / Option i: 独立 workflow + dispatch）を実装形に落としたものである。

## Context

- 3 repo（ruviform・audio-lens・synvelis）とも private。Ruviform Actions から Consumer checkout と Consumer 内の ruviform Git 依存 fetch の 2 段の private アクセスが必要。
- AudioLens: 11-crate workspace、14 個の ruviform git-dep が同一 `rev` pin（`080c228…`、main 比 57 commits 遅れ）。`.cargo/config.toml` に `net.git-fetch-with-cli = true`・`GIT_LFS_SKIP_SMUDGE = "1"`。toolchain pin: Rust 1.99.0 等。CI（`workflow_dispatch`・3 OS matrix）は設計済みだが credential 未設定で実走 0 件。
- Synvelis: Cargo workspace 未存在（設計段階）。mise.toml は基盤ツールのみで Rust toolchain 宣言なし。Consumer lane は `not run` を正しく報告する側の設計対象。
- Ruviform: `ci.yml` は pull_request/push/workflow_dispatch trigger で required checks 12 件稼働中（AGENTS.md の記述は stale で本 Issue で修正対象）。secrets・Deploy Key は現在未設定。
- ローカル実測済み: audio-lens scratch clone で全 ruviform `rev` を候補 SHA `41772a7…` に書換 → `cargo fetch` で全 15 crate が単一 SHA に再解決 → `cargo check --workspace --all-targets` exit 0（25.78s）。

## Components

### `.github/workflows/consumer-verify.yml`

`workflow_dispatch` inputs（`inputs.*.default` には式を書けないため、候補 revision は各 job の `env` で解決する）:

| input | 既定 | 用途 |
|---|---|---|
| `ruviform_rev` | 空 | 検証対象の Ruviform commit SHA。job env で `env: RUVIFORM_REV: ${{ inputs.ruviform_rev \|\| github.sha }}` として解決し、preflight が 40 桁 hex を検証 |
| `audiolens_ref` | `main` | AudioLens の checkout ref（**branch/tag 名のみ**。depth 1 clone 可能な ref name を preflight で形式検証。任意 commit SHA は対象外） |
| `synvelis_ref` | `main` | Synvelis の checkout ref（同様に branch/tag 名のみ） |

jobs（Consumer 同士は互いに独立 — `needs` は全 job が `preflight` のみを参照し、verify 間の依存は無い）:

1. **`preflight`**: `CROSS_REPO_DEPLOY_KEY` の有無を step 内 env 経由で判定し output `credential_configured` を返す（`secrets.*` コンテキストは `jobs.<id>.if` / `steps.if` では使えないため、`env: KEY: ${{ secrets.CROSS_REPO_DEPLOY_KEY }}` → `[ -n "$KEY" ]` → `echo "credential_configured=true" >> $GITHUB_OUTPUT` の step-level パターンを使う。秘密そのものは出力しない）。`RUVIFORM_REV` の形式（40 桁 hex）と `audiolens_ref`/`synvelis_ref`（branch/tag name 形式）を検証し、不正入力は **preflight 自体を fail** にする。
2. **`verify-audiolens`**（`needs: [preflight]`、`if: always()`）: 先頭 gating step が `needs.preflight.result` を検査する — `failure`/`cancelled` なら `fail (preflight failed)` を summary に書き exit 1（入力検証失敗を credential 未設定に偽装しない）。credential 未設定なら `not run (credential not configured)` を summary に書き exit 0。設定済みなら以下を実行。
   1. `RUNNER_TEMP` に Deploy Key を配置（mode 600）。`known_hosts` は GitHub 公表の SSH host key fingerprint（workflow 内ハードコード定数: `SHA256:uNiVztksCsDhcc0u9e8BujQXVUpKZIDTMczCvj3tD2s`(RSA)・`SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU`(Ed25519)・`SHA256:p2QAMXNIC1TJYWeIOttrVc98/R1BUFWuNC/Ldsl8C8I`(ECDSA)）と照合して pin する（`/meta` 応答の fingerprint と鍵を同時に信頼するのは circular なため、定数照合で真の pin にする。rotation で不一致になったら定数更新を促すエラーを出す）。
   2. `GIT_SSH_COMMAND` に `ssh -i <key> -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes` を設定し、audio-lens を SSH clone（`git@github.com:scarecrow173/audio-lens.git`、depth 1、`audiolens_ref`、`GIT_LFS_SKIP_SMUDGE=1`）。
   3. `git config --global url."git@github.com:scarecrow173/ruviform.git".insteadOf "https://github.com/scarecrow173/ruviform.git"` で cargo の HTTPS fetch を SSH へ誘導（audio-lens CI と**同一**の `.git` suffix 付き prefix — suffix 無しだと `ruviform.git.git` に書換わり fetch が壊れる）。`git ls-remote` で probe。ruviform dep fetch 側は `GIT_LFS_SKIP_SMUDGE=1` を維持（ruviform の test fixture は非メディア形式のため安全）。
   4. `[workspace.dependencies]` 内の全 `ruviform-*` `rev = "…"` を `RUVIFORM_REV` に一括書換（`git diff` で manifest diff を可視化）。`--locked` は付けない — rev 書換後は lockfile の再解決が目的であり、`cargo fetch`（非 locked）が Cargo.lock を更新する（ローカル実測済み。audio-lens 本体 CI の `fetch --locked` は pin 不変の通常検証用であり意図的に外す）。
   5. `cargo fetch` → 単一 SHA guard: `Cargo.lock` の `source = "git+…scarecrow173/ruviform.git…"` 行を抽出し、(a) 1 行以上存在、(b) 全行が `?rev=<RUVIFORM_REV>` かつ `#<RUVIFORM_REV>` 末尾を含むことを assert。**crate 名ではなく repo URL を照合キーにする**（crate rename・transitive `ruviform-rt`・`rev`→`branch` 形態変更でも誤判定しない）。不変条件違反・0 行（書換 no-op）は fail。
   6. `mise install` → `mise run check-ci` → `mise run check-cli-headless`（smoke 相当）→ `mise run test-ci`（Consumer 自身の CI と同一の mise task・`--profile ci`・toolchain pin = audio-lens `mise.toml`）。test lane の直前に `git lfs pull` で `crates/*/tests/fixtures/`・`samples/` の LFS 実体を materialize — `al-media` の mp4 fixture テストは実体必須（check/headless lane は LFS 不要）。
   7. `git diff`（manifest+lockfile）・resolved SHA・各 lane 結果を `GITHUB_STEP_SUMMARY` に Consumer 証拠表として出力（この step は `if: always()` — 失敗 lane こそ証拠を出す）。job `outputs` に `status`（`pass`/`fail`/`not_run`）を書き出し、lane 実行状況を `result` だけでなく status で後続に伝える。
3. **`verify-synvelis`**（同様に `needs: [preflight]`、`if: always()`、同一 gating 構造）: credential 未設定 → `not run`。checkout 後に workspace root `Cargo.toml` が存在しなければ `not run (no buildable workspace)` を summary に出力し exit 0。存在する場合は lane を実行する（v1 は presence check のみだったが、synvelis main が buildable workspace 化したため 2026-11-03 追記のとおり実 lane へ拡張済み — Synvelis 未 buildable を fail にしない方針は presence 判定の `not run` で維持）。job `outputs.status` も同じく出力。
4. **`summary`**（`if: always()`、`needs: [preflight, verify-audiolens, verify-synvelis]`）: 各 job の `outputs.status` と `result` を集約して workflow レベルの証拠表を出力。真偽表: いずれか `fail` または `cancelled` → workflow **失敗**（明示 `exit 1`）。**全 lane が `not run` の場合も失敗** — 証拠ゼロの run を緑にしない（ADR「成功扱いにしない」の実装。credential 未設定期の初回 dispatch は赤で「未検証」を正直に示す）。1 lane 以上が実行済み（pass）かつ fail 無し → 成功（表に `not run` 行も明記）。

### `.github/PULL_REQUEST_TEMPLATE.md`

全 PR に求める Consumer 影響欄:

```markdown
## Consumer 影響分類
| Consumer | impacted / no impact / unknown | 根拠・対応 Issue/PR リンク |
| Synvelis | | |
| AudioLens | | |

## 検証証拠
| 対象 | lane/test | OS | feature | ruviform revision | 結果(pass/fail/not run) | 許容例外 | Rollback |
```

- `unknown` の場合は未検証理由と解消先（起票 Issue または `consumer-verify` dispatch 予定）を必須記述とし、結果語彙は `pass`/`fail`/`not run`（`not run (blocked: <共有契約理由>)` の注記形を許容）。
- **`unknown` の merge 時点ゲートは human gate とする**: required check 化は scope 外のため、レビュアーは merge 前に分類欄の記入と `unknown` 解消先リンクを確認する運用を AGENTS/運用文書の責務として明記する（テンプレートは強制力を持たないため、運用責務を設計で明示する）。

### `.github/ISSUE_TEMPLATE/`

- `consumer-impact.yml`（form）: 変更種別（API/feature/behavior/dependency/CI 基盤）、両 Consumer の分類、根拠、Ruviform↔Consumer 相互リンク欄。
- `consumer-followup.yml`（form）: Consumer 追従作業の起票用。対象 revision・追従 lane・blocker/例外・Rollback 欄。
- `config.yml`: Consumer 側 Issue への contact link（synvelis/audio-lens の Issue 一覧）。

## Interactions and Flows

1. 開発者が Ruviform の変更後（または任意タイミングで）Actions タブから `consumer-verify` を `workflow_dispatch` 起動（`ruviform_rev` 省略時は dispatch ref の SHA）。
2. `preflight` が credential 有無と入力 SHA を検証。
3. 各 Consumer job が独立に実行され、scratch checkout 内で候補 revision に切替 → fetch → check/test を実行し証拠表を job summary に出力。
4. `summary` job が全体の証拠表を集約。結果は dispatch 実行者が PR/Issue にリンクする。
5. `impacted` が検出された場合: 共通契約の問題は Ruviform Issue、Consumer Adapter 追従は Consumer Issue（Synvelis #28 / AudioLens #127 系）に起票・相互リンク（spec R2）。
6. 失敗時の Rollback: Consumer 側は manifest/lockfile diff で pin を旧 SHA に戻す（追従 PR の revert と同じ経路）。Ruviform 側は該当 change の revert/修正 Issue を起票。本 lane は情報 lane のため merge 遮断を伴わず、Rollback は証拠リンクで追跡する。

## Key Decisions

- **credential = read-only Deploy Key + SSH**: `CROSS_REPO_DEPLOY_KEY` secret → `RUNNER_TEMP` 鍵ファイル → `GIT_SSH_COMMAND`。HTTPS clone に PAT を使わない（URL 埋め込み token の漏洩面を避ける）。known_hosts は `/meta` の fingerprint 照合で pin。
- **scratch checkout 内のみ manifest を編集**: Consumer repo に一切の恒久的変更を残さない。diff は証拠として summary に出力。
- **fetch → check → test の段階実行**: fetch/lockfile 再解決の失敗（依存割れ・認証・存在しない SHA）は build 失敗と分離して報告する。
- **`not run` の first-class 化**: credential 未設定・workspace 未存在は job 成功かつ `not run` 表記（unknown≠pass）。実行されなかった lane も表に `not run` で列挙する。
- **v1 スコープ**: ubuntu-latest 単独。3 OS matrix・golden/baseline lane・自動 trigger は Deferred（spec 参照）。（当初の「synvelis は presence check」は synvelis workspace 化を受けて 2026-11-03 追記で実 lane へ拡張）

## Security and Privacy

- 秘密の取り扱い: `secrets.CROSS_REPO_DEPLOY_KEY` を環境変数経由でのみ `RUNNER_TEMP` のファイルに書き出し、echo・artifact・`git config` への残留を禁止する。step 終了時に鍵ファイルと `url.insteadOf` 設定を削除する cleanup step（`if: always()`）を置く。
- third-party action への秘密委譲は行わない（checkout も `actions/checkout` ではなく git CLI clone で鍵を直接制御）。
- host key pin: GitHub 公表の SSH host key fingerprint を workflow 内定数として保持し、`/meta` 応答の鍵と照合する（同一応答を fingerprint 照合に再利用しない — circular 信頼を避ける）。
- cleanup の順序: cleanup step は job 末尾で `if: always()` に置き、`url.insteadOf` 除去 → 鍵ファイル削除の順で、cleanup 後の git/ssh 操作は設計上禁止する（`GIT_SSH_COMMAND` が消した鍵を指して失敗するため、cleanup 以後に step を追加しない運用ルールを明記）。Deploy Key の rotation は手動 — 四半期ごとの再発行または漏洩疑い時の即時失効を runbook に併記する。
- LFS: ruviform dep fetch には `GIT_LFS_SKIP_SMUDGE=1` を維持（ruviform fixture は非メディア）。Consumer clone は smudge skip で浅く取り、test lane 前に `git lfs pull` で必要 fixture のみ materialize（audio-lens の CI も `lfs: true` で fixture を復元している）。
- 権限: workflow の `permissions` は `contents: read` + `actions: none` 相当の最小設定。Consumer repo 側の Deploy Key は read-only。

## Testing Strategy

- `mise exec -- actionlint .github/workflows/consumer-verify.yml` で構文検証。
- credential 未設定での dispatch（初回実走時）: 各 Consumer job が `not run` を出力し、summary が「全 lane `not run` = 証拠ゼロ」で workflow を**失敗**にすることを確認（`unknown` ≠ `pass` の実装確認）。
- 不正 `ruviform_rev` での dispatch: preflight が fail し、各 Consumer job が `fail (preflight failed)` を報告することを確認（credential 未設定への偽装を防止）。
- credential 設定後の初回実走: audio-lens job が単一候補 SHA に lockfile 再解決し、check-ci / check-cli-headless / test-ci の証拠表（lane・OS・feature・resolved SHA・pass/fail/not run・許容例外・Rollback）が出ることを確認（ローカル実測の再現）。
- synvelis job が workspace 非存在 ref に対して `not run (no buildable workspace)` を正しく出力し、buildable ref では manifest rev 注入 → fetch → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test` の証拠表が出ることを確認（2026-11-03 追記 lane・ローカル scratch 実測済み）。
- 秘密漏洩の確認: run ログに鍵・token 文字列が出力されないこと、`git config --list` 差分に token 系 URL が残らないこと。
- 単一 SHA guard の検証: `Cargo.lock` の `scarecrow173/ruviform.git` source 行が 0 行または複数 SHA の場合に fail することを確認。

## Operational Tracking

- **M0〜M7 Consumer 別証拠 tracker の正本**: discovery `issue-219-consumer-cross-repo-ci.md` の `## Evidence` 内 M0〜M7 表。Evidence は append-only のため、運用中の更新は「dated snapshot の追記」で行い、既存表は書き換えない。
- **Consumer 依存追従方針（spec R3 の正本）**: AudioLens の追従先既定ブランチは `main`、更新頻度は「M-gate 受入・契約変更・月次のいずれか早い契機」で追従 PR を起票する。manifest の `rev` pin は再現性のための記録であり更新停止策ではない — 更新 PR が `rev` と `Cargo.lock` resolved SHA を同時に進める運用を維持する（恒久 SHA 固定は禁止）。
- **AGENTS.md drift 修正**: `.github/workflows/ci.yml` の記述（「`workflow_dispatch` 専用の手動検証ワークフロー」）は #199/#201 後の実態（pull_request/push trigger + required checks 12 件）と drift しており、本 Issue の実装タスクで現行 trigger・job 構成・required check 列挙に合わせて修正する。

## References

- Spec: [issue-219-consumer-compat-contract.md](../specs/issue-219-consumer-compat-contract.md)（R1–R10・AC1–AC11）
- ADR: [cross-repo-ci-revision.md](../adr/cross-repo-ci-revision.md)（Option A/1/i の根拠と却下案）
- Discovery: [issue-219-consumer-cross-repo-ci.md](../discovery/issue-219-consumer-cross-repo-ci.md)（棚卸し・M0–M7 初期証拠表・ローカル実測 Evidence）
- audio-lens 側同型設計: `docs/designs/ruviform-dependency-ci.md`・`docs/guides/ruviform-private-access-ci.md`（Deploy Key 手順の正本）
- Issue: [ruviform#219](https://github.com/scarecrow173/ruviform/issues/219)

## Appendix: credential 一次設定 runbook（owner/admin 作業）

1. `ssh-keygen -t ed25519 -C "ruviform-cross-repo-ci" -f cross-repo-ci-key` で専用鍵ペアを生成（パスフレーズなし、生成物は repo 外・一時領域に置く）。
2. 公開鍵 `cross-repo-ci-key.pub` を 3 repo（`scarecrow173/ruviform`・`scarecrow173/audio-lens`・`scarecrow173/synvelis`）の Settings → Deploy keys に **read-only（Allow write access オフ）** で登録。
3. 秘密鍵 `cross-repo-ci-key` の内容を `scarecrow173/ruviform` の Settings → Secrets and variables → Actions に `CROSS_REPO_DEPLOY_KEY` として登録。
4. 生成した鍵ファイルをローカルから削除（clip 等に残さない）。
5. `consumer-verify` を dispatch して初回実走。`not run` → `pass/fail` への遷移を証拠リンクとして Issue #219 に記録。
6. 失効手順: 3 repo の Deploy Key 登録を削除 → secret を削除/差替。

## 2026-11-03 追記: gate 本体実装（synvelis 実 lane・PR 分類機械検査）

synvelis main `f7aadd0` の buildable workspace 化（5 member・ruviform git dep ×10・lock `322a661` 単一解決）を受け、設計 §Components 3 の presence check stub を実 lane へ拡張した。変更点:

- **`verify-synvelis` 実 lane**: workspace `Cargo.toml` 存在時に audio-lens lane と同じ流れ（private dep probe → manifest rev 正規化 → `cargo fetch` → 単一 SHA guard → `mise run check` → `git lfs pull` → `mise run test`）を実行する。`Cargo.toml` 不在 ref は従来どおり `not run (no buildable workspace)`（後方互換ガードとして presence 判定を残す）。
- **依存形態差異の吸収**: synvelis は ruviform 依存を `rev` 未指定の既定ブランチ追従で宣言する（synvelis AGENTS.md「SHA の恒久的ハードコード禁止」規約）。そのため manifest 書換を「`rev` pin 置換・`branch` 追従除去・`rev` 未指定への注入」の 3 形態を同一ルールで `rev = "$RUVIFORM_REV"` へ正規化する sed に両 lane で統一した（audio-lens 側の `rev`→`branch` 移行が起きても同じ lane で動く）。scratch checkout 内のみの変更で consumer repo の追従形態は変えない。
- **`.cargo/config.toml` 非存在の補完**: synvelis には audio-lens の `net.git-fetch-with-cli` / `GIT_LFS_SKIP_SMUDGE` 宣言が無いため、job env で `CARGO_NET_GIT_FETCH_WITH_CLI=true`・`GIT_LFS_SKIP_SMUDGE=1` を供給する（同 env は `1` ではなく `true`/`false` 文字列を要求する点に注意）。
- **toolchain 復元の scope**: synvelis lane の `mise install` は lane が使う `rust` と `git-lfs` のみに絞る（`--locked`）。
- **`test` lane は `--locked` 付き**: synvelis の `mise run test` = `cargo test --workspace --locked` であり、fetch 後の lockfile 整合を追加で検証する。
- **audio-lens smoke を spec R5 に整合**: `cargo build -p al-cli --profile ci`（audio-lens 側 ci.yml と同一入口）を `check-cli-headless` に先行する smoke step として追加。
- **`.github/workflows/consumer-impact-check.yml` 新設**: `pull_request`（`opened`/`edited`/`synchronize`/`reopened`/`ready_for_review`・main 向け・draft 除外）で PR 本文の `## Consumer 影響分類` 表を機械検査する lane。検査は「分類欄の記入有無（impacted/no impact/unknown 語彙）」と「`unknown` 使用時の `### unknown の解消先` 記入有無」のみで、分類内容の正しさは機械判定しない。HTML コメントは記入とみなさない（テンプレートの説明文を素通しさせないため除去してから検査）。未記入・語彙外・解消先なしは job fail。required checks には含めず、分類の最終確認は引き続きレビュアー human gate。
- **ローカル実測（credential 非依存区間）**: synvelis `f7aadd0` の scratch clone で候補 `a014b3f9`（ruviform main 当時 HEAD）に対し、manifest 正規化 → `cargo fetch` で 15 crate が `?rev=a014b3f9#a014b3f9` に単一解決（guard bad=0）→ `mise run check` exit 0（10.66s）→ `git lfs pull` → `mise run test` exit 0（23 suite・0 failed）を確認済み。SSH/credential 経路と実 dispatch は引き続き未実走（Appendix runbook の一次設定待ち）。
