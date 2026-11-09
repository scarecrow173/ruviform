---
id: "IMPL-euXEIcDb4J5tm1vO1GDkHH"
type: "impl"
status: "completed"
title: "Issue #306 IO 合成 E2E slice-3 — IO MSRV lane・DAG/RT 境界 audit 証跡"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
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
    - "docs/tasks/issue-306-a1-msrv-dag-rt-audit.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
    - "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
    - "evidence/rf306/report.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
  verified-by: []
  references: []
  defers:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #306 IO 合成 E2E slice-3 — IO MSRV lane・DAG/RT 境界 audit 証跡

## Summary

Issue #306 受入条件の「MSRV1.89/Feature/License/DAG/RT 監査」
行のうち headless で誠実に実行可能な部分を閉じた。IO 系 6
crate を対象とする `test-io-msrv` task を `mise.toml` に追加
し rust@1.89.0 で実走（680 tests 全 pass）、`cargo
metadata`/`cargo tree` で io-e2e leaf・media-io
Domain-Core3 方向・rt 非 io/blocking の 3 判定を機械確認し、
License 現行状態（仮置き・未決定）と共に
`evidence/rf306/report.md` へ slice 3 節として追記した。
crate の公開 API・実装・feature・依存は無変更。

## Context

- 上流: [spec](../../specs/issue-306-io-msrv-dag-rt-audit.md)・
  [design](../../designs/issue-306-io-msrv-dag-rt-audit.md)・
  [plan](../../plans/issue-306-io-msrv-dag-rt-audit.md)・
  [test-spec](../../test-specs/issue-306-io-msrv-dag-rt-audit.md)
- 前段 slice: [IR slice-2](issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md)
  （77 tests を含む io-e2e・証跡 report 2 本は main 統合済み）。
- 証拠: [evidence/rf306/report.md](../../../evidence/rf306/report.md)
  の「Slice 3 追記」節。

## Implementation

### Changes

- `mise.toml`: `test-ui-test-msrv` 直後に `[tasks.test-io-msrv]`
  を追加。`run` は 2 コマンド — (1) `mise exec rust@1.89.0 --
  cargo test -p` で io-file/io-archive/io-structured/
  io-compression/io-wav/io-e2e の 6 crate 一括、(2) `mise exec
  rust@1.89.0 -- cargo test -p ruviform-io-compression
  --features deflate`。`env = { CARGO_TARGET_DIR =
  "target/msrv-io" }` で dev `target/` と分離。
- `evidence/rf306/report.md`: slice 3 節を追記 — MSRV lane
  実測値（crate 別 test 数・ignored 内訳・feature unify の
  実測確認）、DAG/RT audit の実コマンドと判定、License 状態、
  NotRun/Unsupported 一覧。
- `crates/ruviform-io-e2e/README.md`: 検証節に `test-io-msrv`
  を追記。
- `docs/development-environment.md`: opt-in task 一覧に
  `test-io-msrv` を追記。
- 本 doc チェーン 6 本 + managed index 再生成
  （`docs/{specs,designs,test-specs,plans,tasks}/README.md`）。

### Decisions During Implementation

- **feature unify を lane 設計の根拠に採用**: io-e2e を同一
  `-p` 集合に含めると resolver v2 の feature unify で
  io-structured 全 format（json,yaml,toml,csv,xml,file）と
  io-compression gzip,zstd が MSRV build に載る。実測で
  io-structured 117 tests・gzip_format 20・zstd_format 34・
  deflate_format 0（cmd1）/ deflate_format 11・gzip/zstd 0
  （cmd2）を確認し、E2E 消費 feature と deflate 補完の分離を
  確認した。
- **deflate は別コマンドで補完**: io-e2e が deflate を消費
  しないため unify 対象外。`test-io-compression` 正規 lane
  の feature matrix に対応する形で `--features deflate`
  単独実行を第 2 コマンドとした。
- **audit は `cargo tree -i` + `cargo metadata` parse の併用**:
  `cargo tree -i` は dev 辺を含む逆依存を示し leaf を一意に
  判定。`cargo metadata` の機械 parse で dependencies 名
  集合の assert（io-e2e reverse dep 0・media-io ==
  Domain-Core3・rt 非 io/blocking/path）を行い、目視確認に
  留めない。
- **証跡は `evidence/rf306/report.md` へ slice 節追記**:
  Issue 級 report に slice 別 worktree/branch/base/環境を
  独立記載し、slice 1 header との混同を避けた。新規 YAML/
  CI framework は作らない（Issue 制約）。
- **ID 形式の訂正**: doc chain 初版の artifact ID が
  `PREFIX-<22-char Base62>` 規約（`audit_docs.js` の
  idPattern）に合わず `invalid-id-prefix` となったため、
  22 文字 Base62 suffix へ再生成した（seed のある推測では
  なく audit rule の実規約に合わせた修正）。

## Related Experiments

なし（lane 追加と audit は plan 確定済み。ID 形式の修正経緯は
本節 Decisions に記録）。

## Validation

全て `mise` タスクで実走（exit 0）:

- `mise exec rust@1.89.0 -- rustc --version` →
  `rustc 1.89.0 (29483883e 2025-08-04)`
- `mise run test-io-msrv` — rust@1.89.0 で **680 passed /
  0 failed / 1 ignored**（ignored は io-file `os_publish.rs`
  の既存 gate spawned `#[ignore]` helper）。
- `mise run fmt-check`（`cargo fmt --all -- --check`）
- `mise run check`（`cargo check --workspace --all-targets`、
  CARGO_BUILD_JOBS=2）
- `mise run lint`（`cargo clippy --workspace --all-targets --
  -D warnings`、CARGO_BUILD_JOBS=2）
- `mise run test`（`cargo test --workspace`、CARGO_BUILD_JOBS=2）
  — **3580 passed / 0 failed / 7 ignored**
- DAG/RT audit: `cargo tree -i ruviform-io-e2e`（自身のみ）・
  `cargo tree -p ruviform-media-io --depth 1`（Domain-Core3 の
  み）・`cargo tree -p ruviform-rt`（arc-swap/rtrb +
  criterion dev のみ）・`cargo metadata` python parse assert
  — 全 Pass。
- `doc-maintenance` plan/apply で managed index 再生成、
  `doc-status` audit で本 chain の blocking finding なし。

## Risks

- MSRV lane は test 実行のみで clippy を含まない
  （`test-ofx-msrv`/`test-ui-test-msrv` と同範囲）。MSRV
  clippy は Issue #223 の情報 lane 方針に従い本 slice では
  追加していない。
- audit 証跡は実行時点の snapshot。依存 DAG 変更時は
  `evidence/rf306/report.md` slice 3 節の再実行が必要。
- feature unify は io-e2e dev-deps に依存するため、将来
  io-e2e の feature 要求が変われば lane の検証範囲も追従
  する（意図どおり。deflate のような非消費 feature は
  個別補完が必要）。

## Follow-ups

- macOS/Linux 実機・Network/Cloud FS・consumer 実測
  （AudioLens #139/#143・Synvelis #82/#84）・実 4 GiB・
  Issue #306 close — headless 不可のため NotRun/Blocked。
  #219/#294 cross-repo gate の責務（slice 2 report の
  引き渡し節を参照）。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の
  統合レベル証拠が必要になった場合は `ruviform-io-file`
  mock 注入点の公開 fixture 化を別 Issue で扱う
  （slice 2 IR Follow-ups と同じ）。
