---
id: "DESIGN-NS1HOEsCLKLDYdKMhuZOSf"
type: "design"
status: "approved"
title: "Issue #411 — cargo_metadata 依存系列選定設計（MSRV 1.89 収束）"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/411"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-ui-test/src/driver_boundary.rs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-390-driver-contract-lab.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-411-msrv-cargo-metadata-pin.md"
  derived-by:
    - "docs/plans/issue-411-msrv-cargo-metadata-pin.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-411-msrv-cargo-metadata-pin.md"
  references:
    - "https://docs.rs/cargo_metadata/0.19.2/cargo_metadata/"
    - "https://index.crates.io/ca/rg/cargo-platform"
    - "https://index.crates.io/ca/rg/cargo_metadata"
  defers: []
  deferred-by: []
---

# Issue #411 — cargo_metadata 依存系列選定設計

## 診断

- 観測症状: Synvelis `mise run test-msrv`（rustc 1.89.0）で
  `cargo-platform@0.3.3 requires rustc 1.91`（exit 101）。
  RF worktree で `mise exec rust@1.89.0 -- cargo test
  -p ruviform-ui-test` を実行し同一症状を再現確認済み
  （他 MSRV 超過 package なし — cargo-platform 0.3.3 のみ指摘）。
- 原因経路: `ruviform-ui-test` 通常 dep → `cargo_metadata 0.23.x`
  → `cargo-platform ^0.3.0` → 通常解決で最新の 0.3.3
  （rust-version 1.91）。
- lockfile だけを platform 0.3.2（rust-version 1.88）へ下げると
  即時は通るが、次回 `cargo update` 相当の再解決で 0.3.3 に
  戻り再発する — 恒久修復でない（Issue #411 同見解）。

## 候補評価（crates.io index 実測値）

| cargo_metadata | 自身の rust-version | cargo-platform 要求 | platform 解決先 | MSRV 1.89 |
| --- | --- | --- | --- | --- |
| 0.19.2 | 1.78 | `^0.1.2` | 0.1.9（rv 1.78） | 適合 |
| 0.20.0 | 1.82 | `^0.2.0` | 0.2.0（rv 1.81） | 適合 |
| 0.21.0 | 1.86 | `^0.2.0` | 0.2.0（rv 1.81） | 適合 |
| 0.22.0+ | 1.86 | `^0.3.0` | 0.3.3（rv 1.91） | 不適合 |
| 0.23.1 | 1.86 | `^0.3.0` | 0.3.3（rv 1.91） | 不適合 |

## 決定: `cargo_metadata = "0.19"`（解決先 0.19.2）

- **選定理由**: 通常解決の最大値が MSRV 適合系列に閉じる
  （platform `^0.1.2` → 0.1.x のみ。0.2/0.3 へ到達不能）ことが
  Issue の受入条件「通常の Consumer 再解決でも対応 MSRV 範囲へ
  収束」を最も直接的に満たす。0.19.2 の推移 closure は
  camino/cargo-platform/semver/serde/serde_json/thiserror のみで、
  全て workspace 既存の共有 dep — 追加の通常 dep・新規
  transitive tree を持ち込まない（Issue の候補評価と一致）。
- **0.20/0.21 を取らない理由**: platform 0.2 系で MSRV 上は
  適合だが、`cargo-util-schemas`（+ toml/url/schemars/
  serde-untagged/serde-value/unicode-xid）の open-ended な
  transitive tree を新たに引く — 将来の最大版 bump で MSRV
  超過へ再発するリスク面が増える。0.19.2 の方が周辺 closure が
  小さく恒久性が高い。
- **API 互換**: guard が使う cargo_metadata API surface
  （`MetadataCommand::{new,manifest_path,no_deps,other_options,exec}`、
  `Metadata::{packages,workspace_members,resolve}`、`Resolve.nodes`、
  `Node::{id,deps}`、`NodeDep::{pkg,dep_kinds}`、`DepKindInfo.kind`、
  `Package::{id,name,dependencies,features,manifest_path,targets}`、
  `Dependency::{name,rename,kind,optional,features,
  uses_default_features,path}`、`DependencyKind` 全 variant、
  `Target::{kind,src_path}`、`TargetKind::Lib`、`PackageId` の
  Hash/Eq/Display、`cargo_metadata::camino` re-export）は
  docs.rs 上 0.19.2 に全て存在。唯一の型差は name 系が
  `PackageName`（0.21+）ではなく `String`（0.19.2）になる点で、
  呼び出しは `.as_str()`/`.clone()`/`{name}` Display で吸収可能。
  実測: 0.19.2 で lib/bin/test target が compile 成功、
  `*.name.to_string()` 12 箇所のみ clippy `implicit_clone` 抵触
  → `*.name.clone()` へ機械置換（String 生成結果は同一）。
- **却下した代替**: `cargo_metadata = "=0.23.1"` exact pin は
  transitive 範囲を変えず再発。`cargo-platform` への直接 dep
  追加による constraint 固定は「余分な transitive 直接依存を
  置くだけ」の構造的回避で Issue の指示と責務境界に反し却下。
- **保持される意味**: ruleset・dep-graph/signature/allowlist の
  検査、`--offline` 実行条件、fixture 正負 semantic test、
  finding code/witness path の型は一切変更しない。

## 検証入口（本 Issue で新設）

`mise.toml` に `[tasks.test-ui-test-msrv]` を `test-ofx-msrv` と
同型で追加（`mise exec rust@1.89.0 -- cargo test
-p ruviform-ui-test`、`CARGO_TARGET_DIR=target/msrv-ui-test`）。
cargo 直叩きを避ける MSRV の正規入口として維持する。

## Links

- Spec: [issue-411-msrv-cargo-metadata-pin](../specs/issue-411-msrv-cargo-metadata-pin.md)
- Plan: [issue-411-msrv-cargo-metadata-pin](../plans/issue-411-msrv-cargo-metadata-pin.md)
- Test Spec: [issue-411-msrv-cargo-metadata-pin](../test-specs/issue-411-msrv-cargo-metadata-pin.md)
- Task: [issue-411-msrv-cargo-metadata-pin](../tasks/issue-411-msrv-cargo-metadata-pin.md)
