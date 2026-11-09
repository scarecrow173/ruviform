---
id: "DESIGN-CTGZSygUhyfXwQSdShQuap"
type: "design"
status: "approved"
title: "Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 設計"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/tasks/issue-306-a1-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf306/report.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
  refines:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 設計

## Context

- 上流: [spec](../specs/issue-306-io-msrv-dag-rt-audit.md)
  （approved）・Issue #306 本文（受入条件の正本）。
- 前提: `test-ofx-msrv`（`mise.toml`）と `test-ui-test-msrv`
  （Issue #411）が MSRV lane の先例。`cargo metadata`/`cargo
  tree` は workspace DAG の機械確認手段として確立済み。
- 層構造の正本: `docs/designs/ruviform-architecture-design-
  baseline.md`（approved）・`crates/README.md` 索引。

## Decisions

### D1: MSRV lane は単一 task `test-io-msrv`、2 コマンド構成

`mise.toml` の MSRV task 末尾群（`test-ofx-msrv`・
`test-ui-test-msrv` の直後）に配置する。`run` は 2 要素の
配列:

1. `mise exec rust@1.89.0 -- cargo test -p ruviform-io-file
   -p ruviform-io-archive -p ruviform-io-structured
   -p ruviform-io-compression -p ruviform-io-wav
   -p ruviform-io-e2e`
2. `mise exec rust@1.89.0 -- cargo test -p
   ruviform-io-compression --features deflate`

`env = { CARGO_TARGET_DIR = "target/msrv-io" }` で dev profile
の `target/` と分離する（先例と同型）。

根拠:

- **feature unify による網羅**: `ruviform-io-e2e` の
  dev-dependencies が io-structured に
  `json,yaml,toml,csv,xml,file`、io-compression に
  `gzip,zstd` を要求する。同一 `cargo test` 呼出で `-p`
  集合に io-e2e を含めると resolver v2 の feature unify に
  よりこれら feature が有効化され、E2E が実際に消費する
  組合せが MSRV toolchain でそのまま検証される。個別
  `--features` 指定を各 crate に書くより、消費側の実使用
  feature を正本として写す方が drift しない。
- **deflate の補完**: io-e2e は deflate を消費しないため
  unify 対象外。`io-compression` の feature matrix 正規 lane
  （`test-io-compression`）が deflate を含むことに対応し、
  MSRV lane でも `--features deflate` 単独実行で補完する。
  2 コマンド目は初回 build 済み依存を `target/msrv-io` で
  再利用するため増分コストは小さい。
- **対象 crate の限定**: `io-avi`・`io-symphonia` は Issue
  #306 の対象外（IO-01〜05 = file/structured/compression/
  archive/wav + e2e 合成）。`io-e2e` 自体が io-symphonia を
  dev-dep するため symphonia の推移依存は MSRV build に
  含まれるが、symphonia 自身の test suite は本 lane の
  対象にしない。

却下案:

- 各 crate の feature matrix を MSRV で全網羅（`test-io-
  compression`/`test-io-structured` と同じ `&&` 連結）:
  実行時間が増え、MSRV リスクは依存 crate 側（serde・
  flate2・zstd-safe 等）に集中するため、E2E 消費 feature +
  deflate 補完で十分と判断。matrix 全網羅が必要になれば
  別 Issue で拡張する。
- `cargo +1.89.0` 直接指定: 検証入口規約（`mise.toml` タスク
  経由・`mise exec` で toolchain 固定）に反するため却下。

### D2: DAG/RT audit は `cargo metadata` + `cargo tree` の併用

機械確認は 2 系統で行い、両方の実コマンドと結果を証跡に残す:

- `cargo tree -i ruviform-io-e2e` — 逆依存が自身のみ =
  leaf 判定。`cargo tree -i` は dev-dependency 辺も既定で
  含むため、「本番・dev いずれの辺も無い」ことを一意に示す。
  補助として全 `crates/*/Cargo.toml` の `ruviform-io-e2e`
  参照 grep でも 0 件を確認する。
- `cargo metadata --format-version 1` を `python` で parse
  し、(a) io-e2e の reverse dep 0 件、(b) media-io の
  dependencies 名集合 == Domain-Core3、(c) 3 core が
  media-io/io-* へ上向き依存しない、(d) ruviform-rt の
  dependencies に `ruviform-io-*`・blocking 系・workspace
  path 依存が無い、を assert する。`cargo tree -p
  ruviform-media-io --depth 1`・`cargo tree -p ruviform-rt`
  の出力も証跡に転記する。

「Domain-Core3」は `Cargo.toml` workspace コメントと
`crates/README.md` の層構造で確立した media-io の許可依存
`ruviform-media-core`・`ruviform-audio-core`・
`ruviform-video-core` を指す。

### D3: 証跡は `evidence/rf306/report.md` へ slice 節を追記

新規 YAML/CI framework を作らない Issue 制約に従い、既存
の issue 級 report `evidence/rf306/report.md` に「slice 3」
節を追記する。節内に当該 slice の worktree・branch・base
SHA・環境・実行日時を独立して記し、slice 1 の header 情報と
混同しない。`fault-fuzz-report.md` は slice 2 専用のため
対象外。

### D4: License は現行値の転記のみ

`[workspace.package] license = "MIT"` は AGENTS.md 記載の
仮置き（license 未決定・`LICENSE` ファイル非存在）。証跡
にはこの事実関係のみを記録し、ライセンス決定・承認を
主張しない。

## Risks / Trade-offs

- MSRV toolchain でのみ発火する lint/コンパイル差異:
  `test-io-msrv` は test 実行のみで clippy を含まない
  （`test-ofx-msrv`/`test-ui-test-msrv` と同じ範囲）。
  MSRV clippy の扱いは `docs/development-guidelines.md`
  （Issue #223）の情報 lane 方針に従う。
- feature unify の暗黙性: io-e2e の dev-dep feature 変更で
  MSRV lane の検証範囲が追従する（意図どおり）。逆に
  io-e2e が deflate を消費し始めた場合は 2 コマンド目が
  冗長になるが、害はない。
- audit は実行時点の snapshot。依存追加時は再実行が必要
  （証跡に実行日時を記録）。
