---
id: "TSPEC-7h5x0wTVw48oHC6pAm1VId"
type: "test-spec"
status: "approved"
title: "Issue #306 IO E2E slice 3 テスト仕様"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
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
    - "docs/test-specs/issue-306-io-composition-e2e.md"
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 3 テスト仕様

## Purpose

Issue #306 受入条件の「MSRV1.89/Feature/License/DAG/RT 監査」
行を、headless で誠実に実行可能な範囲で固定する。本 spec は
MSRV lane と audit 証跡が保証すべき契約を定め、実装方法は
定めない。fixture・公開 API の追加は行わない。

## Feature

(a) IO 系 6 crate が MSRV 1.89 toolchain で build・test でき、
E2E が実際に消費する feature 組合せを含む。(b) 依存 DAG が
io-e2e leaf・media-io → Domain-Core3 一方向・rt 非 io/blocking
の 3 判定を機械確認で満たす。(c) License の現行状態（仮置き・
未決定）が証跡に正直に記録される。

## Rules

- R1（MSRV lane の存在と入口）: `mise.toml` に
  `test-io-msrv` task が存在し、`mise exec rust@1.89.0` で
  rustc 1.89.0 を固定して `cargo test` を実行する。
  `CARGO_TARGET_DIR` は `target/msrv-io` に分離する。
- R2（対象範囲）: lane は `ruviform-io-file`・
  `ruviform-io-archive`・`ruviform-io-structured`・
  `ruviform-io-compression`・`ruviform-io-wav`・
  `ruviform-io-e2e` の 6 crate を対象とする。
- R3（feature 網羅の根拠）: io-e2e を同一 `-p` 集合に含め、
  dev-deps 経由の feature unify で io-structured の
  json,yaml,toml,csv,xml,file と io-compression の gzip,zstd
  が MSRV build に含まれる。io-compression の `deflate` は
  io-e2e 非消費のため別コマンドで `--features deflate` を
  実行する。
- R4（green の意味）: lane の exit 0 は rustc 1.89.0 上での
  test 全 pass を意味し、失敗・ignored の増加を伴わない
  （`os_publish.rs` の gate spawned `#[ignore]` helper は
  既存の意図的 ignore であり対象外）。
- R5（io-e2e leaf）: `cargo tree -i ruviform-io-e2e` の逆依存
  が自身のみであり、全 `crates/*/Cargo.toml` に
  `ruviform-io-e2e` への依存宣言が存在しない（dev 辺を含む）。
- R6（media-io 依存方向）: `ruviform-media-io` の dependencies
  が `ruviform-media-core`・`ruviform-audio-core`・
  `ruviform-video-core` の Domain-Core3 のみであり、3 core
  のいずれも media-io・`ruviform-io-*` へ依存しない。
- R7（rt 非 io/blocking）: `ruviform-rt` の dependencies
  （kind 不問の宣言面）に `ruviform-io-*`・blocking I/O 系
  crate・workspace path 依存が存在しない。dev-deps の
  criterion bench harness は非 production 依存として区別
  して記録する。
- R8（証跡の完全性）: `evidence/rf306/report.md` の slice 3
  節に、実行した各コマンド・実測結果（test 件数・失敗数・
  判定）・License 現行状態・NotRun/Unsupported が記録される。
- R9（偽装禁止）: `Indeterminate`・DirSync 後失敗・
  `TempState::Unknown` 等、公開 API から trigger 不能な fault
  class の fixture・mock 迂回・公開 API 変更を追加しない。
- R10（索引整合）: 本 doc チェーン追加後、managed docs index
  が `doc-maintenance` の apply で再生成され、`doc-status`
  audit が新規 blocking finding を出さない。

## Examples

- `mise run test-io-msrv` → exit 0。`cargo test -p` 6 crate
  一括で io-structured が yaml/toml/xml 含む 117 tests、
  io-compression が gzip+zstd unify で 96 tests、io-e2e が
  77 tests を 1.89.0 上で実行。第 2 コマンドで deflate
  feature の compression tests が pass。
- `cargo tree -i ruviform-io-e2e` → `ruviform-io-e2e v0.1.0`
  のみ出力（逆依存なし）。
- `cargo metadata` parse → `media-io.dependencies ==
  {media-core, audio-core, video-core}`、`rt.dependencies`
  に io/blocking/path 依存なし、を機械 assert。
- License 記録: 「`license = "MIT"` は配布方針確定までの
  仮置きであり license は未決定」とそのまま転記。

## Guarantees

- MSRV lane の pass は rustc 1.89.0 での実測のみを主張し、
  他 version・他 OS へ一般化しない。
- audit の判定は実行時点の `cargo metadata`/`cargo tree`
  出力に基づき、依存追加時の陳腐化を証跡の実行日時で
  明示する。
- 未実施項目（実 OS・consumer・4 GiB・Issue close）は
  NotRun/Unsupported として区別し、Pass に見せない。

## Non-goals

- MSRV clippy lane（情報 lane 方針は `docs/development-
  guidelines.md` Issue #223 に従い、本 slice では追加しない）。
- feature matrix 全網羅の MSRV 化・fuzz・性能計測。
- Consumer repo での実行・cross-repo gate 自体。

## Risk

- feature unify への暗黙依存: io-e2e の dev-deps 変更で lane
  の検証範囲が追従する。範囲縮小は io-e2e 側の失敗として
  検出されるため silent にはならない。
- audit snapshot の陳腐化: 依存 DAG 変更後に証跡を再実行
  しないまま残るリスク → 実行日時と commit SHA を記録する。
