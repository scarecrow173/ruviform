---
id: "ADR-SG8Lf0sCLiCM8ziAFqCNZ7"
type: "adr"
status: "accepted"
title: "Issue #329 sccache 導入方式 — sccache-action + GHA cache backend + repo-variable kill switch"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-329-sccache-compile-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-329-sccache-compile-cache.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/mozilla-actions/sccache-action"
    - "https://github.com/mozilla/sccache/blob/main/docs/GHA.md"
    - "https://github.com/actions/github-script"
    - "https://github.com/Swatinem/rust-cache"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #329 sccache 導入方式 — sccache-action + GHA cache backend + repo-variable kill switch

## Context and Problem Statement

`ci.yml` の `verify`/`ui-verify` 3 OS に Rust コンパイル成果物キャッシュを導入したい。制約は (a) 新規外部サービス・secret なしで成立すること、(b) 誤共有が構造的に起きないこと、(c) 失敗時に commit なしで無効化できること、(d) job 名・required checks を変えないこと、(e) 追加 action は維持されている公式実装を不変 SHA で pin すること。

## Decision Drivers

- sccache の cache backend 選定が権限・容量・フォールバック設計を決める。
- `ACTIONS_RESULTS_URL`/`ACTIONS_RUNTIME_TOKEN` は runner が JS action にのみ注入し `run` step には渡さない(runner 公式仕様)ため、export には JS action が必須。
- GitHub Actions cache は key 単位 immutable・ref scope 制御で、PR run は merge ref scope にのみ書き込める。
- repo 規約は GitHub 公式 action を version tag pin(`actions/checkout@v7.0.1` 等)。Issue #329 は採用 action に「公式バージョン + 可能な限り不変 commit SHA」を要求 — 新規 action 2 本は tag 名をコメントに残した full SHA pin とする。
- 無効化手段は「再 push なし・merge なしで即時」が望ましい(CI 障害時の MTTR)。

## Considered Options

### A. 導入 action

- **Option A1(採用)**: `mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba`(# v0.0.11)+ `with: version: "v0.19.0"`。Mozilla 維持・marketplace 公開・node24 JS action。sccache バイナリ install + PATH 配置 + post-run `--show-stats` を担う。sccache v0.19.0(2026-10-24)は dep-info をローカル生成に修正(#2894・別 checkout 由来の dep-info が存在しない path を指す問題)と remote storage error の可視化(#2928)を含む correctness/robustness リリースで、CI cache 用途に直結する修正。
- **Option A2**: `Swatinem/rust-cache`。**却下** — `target/` 全体を tar 永続化する方式で、保存/復元の転送コスト・key 設計(workspace 全体の差分管理)・restore 時間が重い。sccache は compile 単位の content-addressed 共有で OS 間・job 間でも真の hit が得られ、Issue が指名する方式でもある。
- **Option A3**: sccache を mise/cargo install で自前導入。**却下** — `mise.toml` は CI 最小ツール集合(`MISE_ENABLE_TOOLS=rust,prek`)に閉じる設計で、sccache を repo 管理ツールに加えるのはバージョン管理・CI 注入点の責務を広げる。action の方が境界が狭い。`cargo install sccache` は毎 run コンパイルで本末転倒。

### B. cache backend

- **Option B1(採用)**: GitHub Actions cache service(`SCCACHE_GHA_ENABLED=true` + `ACTIONS_RESULTS_URL` + `ACTIONS_RUNTIME_TOKEN`)。追加 secret・外部サービス・permission 変更なし。`contents: read` のまま動く(runtime token は runner 注入)。`SCCACHE_GHA_VERSION` で namespace/purge 可能。
- **Option B2**: 外部 S3/R2/WebDAV backend。**却下** — secret 管理(`AWS_*`/endpoint)とコスト・運用責務が新規発生。public repo では token を runner に渡す設計上の露出面も増える。
- **Option B3**: sccache local disk backend + `actions/cache` で `~/.cache/sccache` を保存/復元。**却下(主経路として)** — 既存 cargo cache と同居して容量・失効を競合させ、復元転送が全体 cache 容量に比例して悪化する。credential 不在時の fallback としてのみ sccache 既定(local disk)に倒す設計は採用。

### C. credential export

- **Option C1(採用)**: `actions/github-script@3a2844b7e9c422d3c10d287c895573f7108da1b3`(# v9.0.0)で `core.exportVariable` により `ACTIONS_RESULTS_URL`/`ACTIONS_RUNTIME_TOKEN`/`SCCACHE_*`/`RUSTC_WRAPPER` を `GITHUB_ENV` へ書き出す。sccache 公式 GHA.md の canonical 手順。token 有無を判定して `SCCACHE_GHA_ENABLED` を条件化できる(credential 不在 → `false` → local disk fallback)のもこの方式の利点。
- **Option C2**: `crazy-max/ghaction-github-runtime`。**却下** — 同等機能だが github-script は actions/ 公式で依存評価コストが低く、env export 以外の分岐(GHA_ENABLED 条件化)も同一 step に書ける。
- **Option C3**: 自前 composite/action。**却下** — 車輪の再発明。

### D. 無効化(kill switch)

- **Option D1(採用)**: repository variable `CI_SCCACHE_DISABLE`。2 step(setup・export)を `if: vars.CI_SCCACHE_DISABLE != '1'` で囲み、`export` step 内でのみ `RUSTC_WRAPPER`/`SCCACHE_GHA_*` を `GITHUB_ENV` に出す。var=1 で両 step が skip → env 未設定 → cargo は素の rustc を呼ぶ完全無効状態。値未設定(既定)は有効。**commit・merge 不要**で Settings → Variables から即時無効化できる。
- **Option D2**: `git revert` のみ。**却下(単独では)** — 必須ロールバック経路として文書化するが、merge queue + 全 CI 完走を要し即時性がない。
- **Option D3**: env の条件式(`RUSTC_WRAPPER: ${{ vars.X && 'sccache' || '' }}`)。**却下** — 空文字の `RUSTC_WRAPPER` を cargo がどう扱うかの挙動依存を持ち込む。「env を一切出さない」構造の方が静的に確実。

### E. namespace

- **Option E1(採用)**: `SCCACHE_GHA_VERSION=ruviform-v1-<RUNNER_OS>`。OS 別 prefix で `gh cache list` の key から生成元 OS を識別でき、OS 単位の purge/bump が可能。誤共有防止は sccache の content key が担うため、version は運用・観測目的の名前空間。
- **Option E2**: 単一 `ruviform-v1`。**却下** — key の prefix が OS 共通になり、監査時の帰属が分かりにくい。機能差はないため観測性で分ける。

## Decision Outcome

Chosen options: **A1 + B1 + C1 + D1 + E1**。

各対象 job(`verify`・`ui-verify`)に 3 step を追加:

1. `mozilla-actions/sccache-action@<SHA>`(`version: "v0.19.0"`)
2. `actions/github-script@<SHA>` で runtime token 系 + `SCCACHE_GHA_ENABLED`/`SCCACHE_GHA_VERSION`/`RUSTC_WRAPPER` を export
3. 環境確認 step(`command -v`・`--version`・env 表示・credential presence の `set/MISSING` 出力 — token 値自体は出力しない)

さらに主要 cargo step 群の後に `sccache --show-stats` 出力 step を置く(action の post-run show-stats とは別に job log で即時参照可能にする)。全 step は `if: vars.CI_SCCACHE_DISABLE != '1'`。

### Consequences

- Good: 外部 secret/サービス/permission 追加なしで 3 OS 共有の content-addressed compile cache が立つ。cache 障害・rate limit・credential 不在はすべて fallback/縮退方向(ビルドは継続)。repo variable で即時無効化、`SCCACHE_GHA_VERSION` bump で全 purge、git revert で完全除去。
- Bad: `vars.CI_SCCACHE_DISABLE` は repo 設定依存のため worktree 内で完全検証不能(var 未設定 = 有効既定は静的に確認可、実無効化は repo admin 権限で run 単位検証)。GHA cache service の rate limit で書込みが落ち得る(sccache 公式: 継続するが保存されない可能性)。sccache が rustc 実行 path に挟まるため、sccache 自体の障害時は直接実行 fallback 前提 — fallback が効かない致命的モードが残る可能性は受容し kill switch で緩和。
- Neutral: GHA backend は key ごと immutable なため同一キーの再書込みは起きない(内容が同じなら不要)。fork PR は merge ref scope で read/write 可能で、base/main への書込みはできない — 監査は design 書に記録。github-script は JS action として `github.token` を既定で受けるが `script` は固定式で token を出力しない。
