---
id: "PLAN-oa5MzrXRL7vzdL68cwsGhf"
type: "plan"
status: "approved"
title: "Issue #300 ruviform-io-compression Gzip・Raw Deflate 実装計画 (IO-IMP-06)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/300"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-compression/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-compression/README.md"
      - type: "file"
        path: "crates/ruviform-io-compression/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/options.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/receipt.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/stream.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/gzip.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/src/deflate.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/encoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/decoder_contract.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/format_cases.rs"
      - type: "file"
        path: "crates/ruviform-io-compression/tests/composition.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "ci/impact/map.mjs"
      - type: "file"
        path: "ci/impact/tests/classify.test.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/designs/issue-333-impact-based-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-300-io-compression-gzip-deflate.md"
    - "docs/designs/issue-300-io-compression-gzip-deflate.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-300-io-compression-gzip-deflate.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #300 ruviform-io-compression Gzip・Raw Deflate 実装計画 (IO-IMP-06) Implementation Plan

## Goal

`ruviform-io-compression` クレートを新設し、IO-03 v0.1/v0.2 の Gzip /
Raw Deflate 借用 Stream 契約（`finish` 完了・VerifiedEnd・累積 Budget・
Backend 隠蔽）を実装し、feature 専用 CI lane まで導入する。

## Scope

単一実装ストリーム・1 plan・3 task。変更は
`crates/ruviform-io-compression/` 新設 + workspace 索引/CI 接続に限定。
既存クレートの API・動作は一切変更しない。

## Upstream Documents

- `docs/specs/issue-300-io-compression-gzip-deflate.md`（approved）
- Issue #300 本文（受入正本）・Issue #285 IO-03 v0.1/v0.2・
  Issue #282 統合設計・Issue #288 受入契約

## Design Inputs

- `docs/designs/issue-300-io-compression-gzip-deflate.md`（approved —
  framing 所有・借用型・状態機械・error mapping・CI lane 型）

## File Map

- `crates/ruviform-io-compression/Cargo.toml` — `default=[]`・
  `gzip`/`deflate` features・optional `flate2` dep（1.1.10・
  rust_backend・依存理由コメント）。
- `crates/ruviform-io-compression/README.md` — 責務・層/依存・
  feature 表・finish/Drop 契約・保証範囲。
- `crates/ruviform-io-compression/src/lib.rs` — crate-level doc + pub
  re-export + feature cfg。
- `src/error.rs` — `CompressionError`/`Kind` + `io::Error` 変換。
- `src/options.rs` — `DecodeScope`/`EncodeLimits`/`DecodeLimits`。
- `src/receipt.rs` — `EncodeReceipt`/`DecodeReceipt`/`IntegritySummary`。
- `src/stream.rs` — private deflate pump/byte 計測/状態 enum/共有
  compress loop。
- `src/gzip.rs` — header/trailer codec + `Encoder`/`Decoder` + options。
- `src/deflate.rs` — 単一 stream `Encoder`/`Decoder` + options。
- `tests/encoder_contract.rs` / `decoder_contract.rs` /
  `format_cases.rs` / `composition.rs` — AC1-AC6 試験群。
- `Cargo.toml` — `members` へ追加（`ruviform-io-file` の直後・
  コメントで層位置を記録）。
- `crates/README.md` — 「汎用ファイル I/O」表に行追加。
- `mise.toml` — `check-io-compression`/`lint-io-compression`/
  `test-io-compression` 追加。
- `ci/impact/map.mjs`・`ci/impact/tests/classify.test.mjs`・
  `.github/workflows/ci.yml` — `compression-verify` lane 接続。
- `docs/designs/issue-333-impact-based-ci.md` — lane 表へ
  `compression-verify` 行追加（正本との整合）。
- `Cargo.lock` — `mise run check` の再解決で更新（手編集しない）。

## Implementation Sequence

- [ ] T1: クレート scaffold + `error`/`options`/`receipt`/`stream` +
      `gzip`/`deflate` 両 module + 索引/members/依存接続
      （`mise run check`・feature matrix check が通る状態まで）。
- [ ] T2: 4 試験ファイルで AC1-AC6 を実装（encoder/decoder contract・
      format golden/negative・TAR 様合成）+ rustdoc 充実 + crate README。
- [ ] T3: CI lane `compression-verify`（mise task 3 本・map.mjs・
      ci.yml・classify.test.mjs・issue-333 design doc 整合）+
      impl record + 最終検証。

## Dependencies and Risks

- `flate2 = { version = "1.1.10", default-features = false,
  features = ["rust_backend"] }` — miniz_oxide + crc32fast の pure Rust
  backend。`Cargo.lock` は `mise run check` で更新。
- `git merge origin/main` の衝突（Cargo.toml members・crates/README.md・
  Cargo.lock・map.mjs・ci.yml・mise.toml）は同時期他 worker が触る共有
  面であり、**本タスクの最終検証前に必ず merge して解消する**。
- Windows ローカルで `cargo test` を feature matrix 全部回す。OS 固有の
  追加保証は無い（`std::io` のみ）。

## Task Breakdown

- `docs/tasks/issue-300-t1-crate-core.md` — crate + shared types + 両
  format module + 索引接続。
- `docs/tasks/issue-300-t2-tests.md` — AC1-AC6 試験・rustdoc・crate
  README。
- `docs/tasks/issue-300-t3-ci-lane.md` — CI lane + classifier + design
  doc + impl record + 最終検証。

## Verification

- [ ] `mise run fmt` → `mise run check` → `mise run lint` → `mise run test`
      （dev profile・Windows ローカル）。
- [ ] `cargo check -p ruviform-io-compression`（default）/
      `--features gzip` / `--features deflate` / `--all-features`。
- [ ] `cargo test -p ruviform-io-compression --all-features`。
- [ ] `node --test ci/impact/tests/`（classifier 変更の自己試験）。
- [ ] `mise run verify-ci` 相当（最終 merge 後）。
- [ ] 未検証 OS（Linux/macOS）は CI matrix へ委譲と明記。

## Open Questions / Gaps

- `flate2` backend が `rust_backend` 固定なので zlib-rs runtime
  detection 経路は含まない — backend 差し替え余地は private stream.rs に
  閉じる。
- `NotExhausted`/`TrailingData` の kind 追加は IO-03 案からの拡張 —
  Zstd 担当（#301）と同じ語彙を使えるよう設計書で明文化済み。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（worker 自身が実装）。
- Independent work streams: none（単一ストリーム・T1→T2→T3 順次）。
- User approval required before dispatch: yes — spec/design/plan は
  review-protocol で blocker なしとして approved 済み。

## Review Handoff

- 重点: `Decompress` の `total_in` delta と `consume` の対応、
  AtOutputLimit probe の boundedness、`finish()` 非駆動設計
  （VerifiedEnd 照会のみ）の一貫性、CI lane wiring（workflow.test.mjs
  契約）。
