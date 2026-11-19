---
id: "DESIGN-mcItTpQPSlr7FZ7BBqChaW"
type: "design"
status: "approved"
title: "Issue #329 sccache 設計 — step 構成・env 解決・cache 正しさ・監査・無効化"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "docs/specs/issue-329-sccache-compile-cache.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-329-sccache-compile-cache.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-329-sccache-compile-cache.md"
    - "docs/adr/issue-329-sccache-gha-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-329-sccache-compile-cache.md"
    - "docs/adr/issue-329-sccache-gha-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/mozilla/sccache/blob/main/docs/GHA.md"
    - "https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments#vars-context"
    - "https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows"
  defers: []
  deferred-by: []
---

# Issue #329 sccache 設計 — step 構成・env 解決・cache 正しさ・監査・無効化

## Overview

`verify`(3 OS)と `ui-verify`(3 OS)に sccache を導入する。構成は ADR(Option A1+B1+C1+D1+E1)の通り:

- `mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba`(# v0.0.11、`with: version: "v0.19.0"`)
- `actions/github-script@3a2844b7e9c422d3c10d287c895573f7108da1b3`(# v9.0.0)で credential 系 env を `GITHUB_ENV` へ export
- `vars.CI_SCCACHE_DISABLE != '1'` で全 sccache step を gate(repo variable kill switch)
- 環境確認 step + 統計出力 step を job log に残す

## Step 配置(verify job。cargo cache restore の直後、prek の前)

```yaml
      - name: Set up sccache
        if: vars.CI_SCCACHE_DISABLE != '1'
        uses: mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba # v0.0.11
        with:
          version: "v0.19.0"

      - name: Export sccache GHA backend env
        if: vars.CI_SCCACHE_DISABLE != '1'
        uses: actions/github-script@3a2844b7e9c422d3c10d287c895573f7108da1b3 # v9.0.0
        with:
          script: |
            const token = process.env.ACTIONS_RUNTIME_TOKEN || '';
            const url = process.env.ACTIONS_RESULTS_URL || '';
            const gha = token !== '' && url !== '';
            core.exportVariable('ACTIONS_RUNTIME_TOKEN', token);
            core.exportVariable('ACTIONS_RESULTS_URL', url);
            core.exportVariable('SCCACHE_GHA_ENABLED', gha ? 'true' : 'false');
            core.exportVariable('SCCACHE_GHA_VERSION', `ruviform-v1-${process.env.RUNNER_OS || 'unknown'}`);
            core.exportVariable('RUSTC_WRAPPER', 'sccache');
            core.info(`sccache GHA backend: ${gha ? 'enabled' : 'disabled (cache credentials absent)'}`);

      - name: Verify sccache environment
        if: vars.CI_SCCACHE_DISABLE != '1'
        shell: bash
        run: |
          set -euo pipefail
          command -v sccache
          sccache --version
          echo "RUSTC_WRAPPER=$RUSTC_WRAPPER"
          echo "SCCACHE_GHA_ENABLED=$SCCACHE_GHA_ENABLED"
          echo "SCCACHE_GHA_VERSION=$SCCACHE_GHA_VERSION"
          if [ -n "${ACTIONS_RESULTS_URL:-}" ]; then echo "ACTIONS_RESULTS_URL: set"; else echo "::warning::ACTIONS_RESULTS_URL missing — sccache GHA backend disabled (local disk fallback)"; fi
          if [ -n "${ACTIONS_RUNTIME_TOKEN:-}" ]; then echo "ACTIONS_RUNTIME_TOKEN: set"; else echo "::warning::ACTIONS_RUNTIME_TOKEN missing — sccache GHA backend disabled (local disk fallback)"; fi
```

`test-ci` の後(`Measure disk usage (after build)` の前)に統計 step:

```yaml
      - name: sccache statistics
        if: vars.CI_SCCACHE_DISABLE != '1'
        shell: bash
        run: sccache --show-stats || true
```

`ui-verify` job には同一 3 step を cargo cache restore 直後に、`Upload UI browser evidence` の前に統計 step を置く。

## env 解決表

| event / var | setup step | export step | RUSTC_WRAPPER | SCCACHE_GHA_ENABLED | backend |
| --- | --- | --- | --- | --- | --- |
| var 未設定/ != '1'(既定)・token あり | run | run | `sccache` | `true` | GHA cache(read/write) |
| var 未設定/ != '1'・token なし | run | run | `sccache` | `false` | local disk fallback |
| `CI_SCCACHE_DISABLE=1` | skip | skip | 未設定(空ではなく存在しない) | 未設定 | なし(素の rustc) |

- `RUSTC_WRAPPER` を job env に置かず `GITHUB_ENV` 経由にしたのは kill switch の安全性のため(D 項): step skip → env 未生成 → cargo は wrapper なし。空文字 export という中途半端な状態を作らない。
- token 不在時(`gha=false`)でも `RUSTC_WRAPPER=sccache` は残す — sccache local disk backend は常に起動可能で job 内の重複 compile を吸収する。GHA backend 未設定時の server 起動失敗リスクを消すのが `gha` 条件化の目的。

## Cache 正しさ(誤共有の構造的排除)

sccache のキャッシュキーは **compiler 実行ファイルのハッシュ + 完全なコマンドライン引数 + プリプロセス済みソース + 関連 env** のハッシュ。よって以下は全て key 相違となり誤共有しない:

| 差異 | key への反映 |
| --- | --- |
| OS/arch(`ubuntu`/`windows`/`macos`) | rustc executable hash + target triple が異なる |
| toolchain 版(MSRV 1.89 vs 1.99) | rustc executable hash が異なる |
| profile(dev vs `ci`) | opt-level/debug 等の flags が異なる |
| feature(verify=default vs ui-verify=webview) | `--cfg feature=…` が異なる |
| RUSTFLAGS | 引数に含まれる |
| Cargo.lock(依存 SHA 更新) | 変更 crate は preprocessed source hash が変わる・未変更 crate は hit 継続(部分 hit) |

MSRV lane は ruviform `ci.yml` に存在しない(consumer 側 lane)。将来追加されても toolchain hash 差で共有されない。sccache のキー設計が compile 正しさの authoritative 実装であり、本設計はそれを再利用する(DRY)。なお hit した成果物は sccache が検証してから返し、dep-info は v0.19.0 以降ローカル path で再生成される(#2894)ため checkout path 差による破損参照も起きない。

## GitHub Actions cache backend 監査(実装 4)

| 項目 | 実測/仕様 |
| --- | --- |
| 権限 | `ACTIONS_RUNTIME_TOKEN` は runner が JS action に注入する per-job token(workflow permission 無関係・`contents: read` のまま動作)。`run` step には渡されないため github-script で再 export |
| ref scope | cache は branch scope。`pull_request` run は PR merge ref(`refs/pull/N/merge`)scope で read/write。**base/main へは書けない**。同一 PR の後続 run は自身の scope を read/write |
| fork PR | fork PR run でも cache API token は付与され、merge ref scope で read/write 可能(base repo の main scope からは復元方向のみ限定。他人 PR の scope には触れない) |
| 容量 | repo あたり 10GB。超過時は **least-recently-used から eviction**。sccache は entry ごと独立した cache key(content hash)で書き込むため eviction は古い未使用 entry から部分的に起きる |
| 失効 | 7 日間アクセスのない cache は削除(GitHub 既定) |
| 転送 | cache service への fetch/store は REST。sccache は必要 entry のみ fetch(transfer = hit 分)。rate limit 到達時は sccache 公式「build は継続・store が行われない可能性」の fail-open |
| 既存 cargo cache との分離 | `actions/cache` step(`~/.cargo/registry`)は lockfile hash key・job 単位。sccache GHA backend は cache service API を直接利用・entry 単位。両者は key 空間も用途も独立で、本変更では cargo cache 側を一切変更しない |
| `gh cache list` 観測 | key prefix = `SCCACHE_GHA_VERSION` 由来の `ruviform-v1-<RUNNER_OS>` で OS 帰属を識別可能 |

## 失敗モードと無効化手順

| 失敗モード | 検知 | 影響 | 無効化/復旧 |
| --- | --- | --- | --- |
| credential 不在(環境変化・self-hosted 等) | export step で `gha=false` → `SCCACHE_GHA_ENABLED=false`。env check step が `::warning::` annotation を出す | なし — sccache は local disk backend に縮退し、検証能力は cache インフラ非依存(fail-open)で維持。縮退は annotation で可視 | 恒久なら `CI_SCCACHE_DISABLE=1` |
| GHA cache service 障害/rate limit | stats の storage error 欄・post-run annotations | store 欠落(ビルドは続行) | 一過性なら放置。継続なら `CI_SCCACHE_DISABLE=1` |
| sccache server 起動失敗 | env check `sccache --version`・compile log | sccache client は server 不可時に compiler 直接実行へ fallback | `CI_SCCACHE_DISABLE=1` または `SCCACHE_GHA_VERSION` bump |
| 破損 entry | 内容アドレスで一致しない限り参照されない・v0.19.0 は storage error を可視化 | miss 扱い→再コンパイル | `gh cache delete` / version bump |
| 緊急無効化 | — | — | Settings → Variables `CI_SCCACHE_DISABLE=1`(merge 不要・次 run から効く) |
| 完全 rollback | — | — | `git revert` で step 除去。永続形式・依存追加なし(生成 cache は自然失効) |

## Consumer 展開の前提(AL #148 / SV #147)

- 両 repo は `actions/cache` + mise を使う同系構成(synvelis は `msrv-verify` lane で rust@1.89.0 併用 — toolchain hash 差で誤共有しないことは構造上同じ主張が立つ)。
- 展開前提: 各 repo が自前で (a) action 2 本の SHA pin 採否、(b) `CI_SCCACHE_DISABLE` var、(c) `SCCACHE_GHA_VERSION` namespace(repo 名入り推奨・cache scope は repo 単位で共有されないため衝突はないが識別性のため)、(d) MSRV/feature lane への適用範囲を決める。ruviform 側の変数・命名を consumer が参照する契約は作らない — 各 repo の workflow が独立して同パターンを所有する(依存 SHA・cache 責務を ruviform に移さない、Issue #329 実装 6)。
- 本 PR では ruviform 側 verify/ui-verify の cold/warm 実測のみを証拠として残し、consumer 適否は各 Issue の検証に委譲する。

## Edge cases 検証表

| case | 静的評価 |
| --- | --- |
| `vars.CI_SCCACHE_DISABLE` 未定義 | `'' != '1'` → true → 有効(既定 ON) |
| `pull_request` / `push` / `workflow_dispatch` | var 条件は event 非依存 — 全 trigger で同一動作 |
| `if:` on `uses:` step | step 実行前に評価・skip で action 未実行。`vars` context は step `if` で使用可 |
| `github-script` の `process.env` | JS action には runner 注入 env(ACTIONS_*, RUNNER_*)が見える — runner 公式挙動 |
| token を export してよいか | 値は log に出さず `GITHUB_ENV` へ。GITHUB_ENV 書込みは runner が mask 扱い。`--show-stats` や env check は presence のみ出力 |
| `sccache --show-stats || true` | server 未起動/統計失敗でも job を落とさない(観測 step) |
| sccache-action の post 処理 | `Post Run sccache-cache` が自動追加され `--show-stats` を実行(明示 step と二重だが一方が落ちても観測は残る) |
