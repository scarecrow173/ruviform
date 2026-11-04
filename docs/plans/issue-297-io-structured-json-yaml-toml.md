---
id: "PLAN-034RTeQAV1efnZZIhQamnc"
type: "plan"
status: "approved"
title: "Issue #297 ruviform-io-structured 実装計画 (IO-IMP-03)"
created: "2026-11-04"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/297"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-structured/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-structured/README.md"
      - type: "file"
        path: "crates/ruviform-io-structured/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/error.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/limits.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/input.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/json.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/yaml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/toml.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/src/file.rs"
      - type: "file"
        path: "crates/ruviform-io-structured/tests/structured.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-295-io-file-staged-publish.md"
    - "docs/plans/issue-296-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #297 ruviform-io-structured 実装計画 (IO-IMP-03) Implementation Plan

## Goal

`ruviform-io-structured` を新設し、JSON/YAML/TOML の Serde 型付き
`from_reader`/`to_writer`（Strict EOF・入力 Byte Budget）と `file`
feature の StagedFile Convenience を、共通 `DocumentError` 契約の上に
実装する。IO-IMP-04/05 が乗る基底 scaffold でもある。

## Scope

単一実装ストリーム。共通契約 → Format 別 3 実装 → file feature →
workspace 配線の順で進め、テストは各 Format と並行して積む。
CSV/XML・複数文書 API・Consumer Schema 移管は含めない。

## Upstream Documents

- `docs/specs/issue-297-io-structured-json-yaml-toml.md`
- `docs/designs/issue-297-io-structured-json-yaml-toml.md`
- `docs/test-specs/issue-297-io-structured-json-yaml-toml.md`

## Design Inputs

- `docs/designs/issue-297-io-structured-json-yaml-toml.md`（approved）
- Issue #284 合意追補 v0.1/v0.2・`ruviform-io-file` 公開 API
  （`StagedFile`/`SealedFile`/`PublishReceipt`/`PublishFailure`）

## File Map

- `crates/ruviform-io-structured/Cargo.toml` — feature matrix・依存
- `crates/ruviform-io-structured/README.md` — 責務・feature・制限表
- `crates/ruviform-io-structured/src/lib.rs` — crate doc・mod/re-export
- `crates/ruviform-io-structured/src/error.rs` — DocumentError 族
- `crates/ruviform-io-structured/src/limits.rs` — InputLimits
- `crates/ruviform-io-structured/src/input.rs` — private BoundedRead
- `crates/ruviform-io-structured/src/json.rs` — JSON Format API
- `crates/ruviform-io-structured/src/yaml.rs` — YAML Format API
- `crates/ruviform-io-structured/src/toml.rs` — TOML Format API
- `crates/ruviform-io-structured/src/file.rs` — file feature Error 族 +
  共有 helper
- `crates/ruviform-io-structured/tests/structured.rs` — 契約・否定・
  feature 試験
- `Cargo.toml` — `members` 追加
- `crates/README.md` — クレート台帳追記
- `Cargo.lock` — 依存解決で自動更新

## Implementation Sequence

- [ ] クレート scaffold（Cargo.toml feature matrix + src 骨格 + workspace 登録 + crates/README.md）と `cargo check -p` で feature matrix が成立することを確認
- [ ] `error.rs` + `limits.rs` + `input.rs`（共通契約と BoundedRead）
- [ ] `json.rs`（from_reader/to_writer + Options + Backend 分類）+ JSON 試験
- [ ] `yaml.rs` + YAML 試験（Budget 写像・MultipleDocuments・alias 制限）
- [ ] `toml.rs` + TOML 試験（buffered + 2 相分類 + pretty）
- [ ] `file.rs` + Format module の `read_file`/`write_file` + file 試験
- [ ] 統合試験・crate README・docs 更新・`mise run fmt`/`check`/`lint`/`test`

## Dependencies and Risks

- `serde-saphyr` 2.0.0（MSRV 1.89 ちょうど）・`toml` 1.1.8・
  `serde_json` 1.0.154 — いずれも公開 ≥7 日・MIT OR Apache-2.0。
- 並行 worker（ruv-300/ruv-302）が `Cargo.toml` members・
  `crates/README.md`・`Cargo.lock` を同時編集し得る → merge 時に
  自分の追加分を保持して衝突解消する。
- serde-saphyr `Error` non_exhaustive: 未知 variant の wildcard は
  `Syntax` 既定。upgrade 時は kind 分類レビュー必須（リスク記録済）。

## Task Breakdown

- `issue-297-t1-crate-and-contract` — scaffold + 共通契約 + json
- `issue-297-t2-yaml-toml` — yaml + toml Format module と試験
- `issue-297-t3-file-feature` — file feature + workspace 台帳 + 総合検証

## Verification

- [ ] `mise run fmt` / `mise run check` / `mise run lint` / `mise run test` 全緑
- [ ] `cargo check -p ruviform-io-structured` の feature matrix
      （`--no-default-features`・`--features json`・`yaml`・`toml`・
      `file`・`--all-features`）— 補助証拠（canonical = CI でない）
- [ ] 否定試験（trailing/invalid/over-budget/複数文書/Unicode/
      型不一致）・`file` の Encode/Publish 分類が全緑
- [ ] `git merge origin/main` 後の `Cargo.lock` 再解決 + 最終検証

## Open Questions / Gaps

- JSON depth は serde_json 固定値（128）に依存し公開制限は持たない
  — `Syntax` として分類されることを試験で pin（LimitExceeded 偽装なし）。
- `read_file` の Open は `std::io::Error`（設計どおり io-file 経由
  しない）。`FileError` 化は設計案外のため採用しない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が直接実装）
- Independent work streams: なし（単一ストリーム）
- User approval required before dispatch: no（受入確定済み実装 Issue）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 焦点: Backend 非露出の API 監査、BoundedRead の EOF/limit 境界、
  serde-saphyr Error 分類表の妥当性、file feature の
  PublishFailure 情報保全。
