---
id: "TSPEC-8FPRKgybhQfr9n7cWGNJoz"
type: "test-spec"
status: "approved"
title: "Issue #329 テスト仕様 — sccache 導入の不変条件・ケース・証拠"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-329-sccache-compile-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-329-sccache-compile-cache.md"
    - "docs/specs/issue-329-sccache-compile-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-329-sccache-compile-cache.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-329-sccache-compile-cache.md"
  verified-by: []
  references:
    - "https://github.com/mozilla/sccache/blob/main/docs/GHA.md"
  defers: []
  deferred-by: []
---

# Issue #329 テスト仕様 — sccache 導入の不変条件・ケース・証拠

## Scope

`.github/workflows/ci.yml` の `verify`・`ui-verify` job に追加する sccache 関連 step/setup/env の意味論と実測効果を対象とする。job 名・step 名(既存)・check 名・検証範囲は変更対象外(非変更であることを検証する側)。

## Invariants

- INV-1(検証非破壊): 12 required checks の job 名・job 数・trigger・permissions・既存 step 名は不変。diff は sccache 関連 step 追加 + コメントのみ。
- INV-2(誤共有なし): コンパイル成果物の共有キーは sccache が content-address(compiler hash + 完全引数 + preprocessed source + 関連 env)で構成するため、OS/arch/toolchain/profile/feature/RUSTFLAGS/target/依存 SHA の違いで成果物が再利用されない。MSRV lane との共有も構造上不可能(異なる rustc = 異なる hash)。
- INV-3(cache 非依存): cache 無効(`CI_SCCACHE_DISABLE=1`)・credential 不在・cache miss・破損のいずれでもビルドは成功する。sccache は server 接続失敗時に compiler 直接実行へ fallback し、GHA backend 不在時は local disk backend に縮退する。
- INV-4(役割分離): 既存 `actions/cache`(`~/.cargo/registry`)は不変。依存取得物 = actions/cache、コンパイル成果物 = sccache GHA backend で key 空間・責務を共有しない。
- INV-5(秘密の非露出): `ACTIONS_RUNTIME_TOKEN` の値は log に出力しない(env check は `set`/`MISSING` の presence のみ)。GITHUB_ENV への export は runner が mask する。
- INV-6(観測可能): 各対象 job の log に `sccache --version`・env 状態・`--show-stats`(hit/miss/compile 時間)が残る。

## Cases

| Case | 検証 | 実施者 |
| --- | --- | --- |
| C1: 静的妥当性 | `mise exec -- actionlint .github/workflows/ci.yml` が exit 0。`check-yaml`(prek)green | worker |
| C2: env 分岐の静的評価 | design の env 解決表に対し var 未設定/=`1`・token 有無の 3 状態が表通り(setup/export skip 有無・RUSTC_WRAPPER 有無・backend 種別) | worker |
| C3: Cold run 実測 | 本 PR の初回 CI run で 3 OS `verify`/`ui-verify` が成功し、sccache stats が記録される(cold = miss 支配)。run URL を記録 | worker(gh CLI) |
| C4: Warm run 実測 | 同一 PR の 2 連続 push で warm run が hit を示し、wall time / runner total を cold・baseline(#327 exp)と比較して記録 | worker(gh CLI) |
| C5: 無効化 path | repo variable `CI_SCCACHE_DISABLE=1` を設定して job re-run → sccache step が skip され build 成功。終了後 var を削除 | worker(権限があれば)。不可なら理由を明記し NotRun |
| C6: Cargo.lock 更新 | ローカルで `cargo update -p <dep>` → rebuild で更新 crate のみ miss・他は hit(部分 hit)を `sccache --show-stats` で記録。lockfile は戻す | worker(ローカル) |
| C7: feature 切替 | ローカルで default feature vs `webview` feature の `cargo check` が独立した key(= miss)を持つことを記録。CI 上では verify(default)と ui-verify(webview)の hit 分離としても観測 | worker(ローカル + CI log) |
| C8: 破損 entry | ローカル sccache cache の entry を破損させて rebuild → miss/再コンパイルで成功することを記録 | worker(ローカル) |
| C9: 非破壊 diff | `git diff` で sccache step + コメント追加以外の変更がない | worker |
| C10: rollback | `git revert` で step 群を除去 → 従来構成へ即時復帰(差分が追加のみのため自明)。文書化のみ | 記録 |

## Evidence

- worker: actionlint/prek 出力・`git diff` 要約・cold/warm run の URL と `sccache --show-stats` 抜粋・ローカル実験の stats・repo var 無効化 run の job log を IR/exp に記録。
- `collect-ci-metrics.mjs` で対象 run の run/job/step レコードを収集し `docs/impl/exp/data/issue-329-sccache-runs.jsonl` に保存。Before は #327 baseline exp 値。
- NotRun とした検証は理由と後続実施先を IR に明記する(Pass に擬装しない)。
