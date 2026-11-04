---
id: "IMPL-034RTVShL0YrAoiEgYJyNu"
type: "impl"
status: "completed"
title: "Issue 297 ruviform-io-structured — Serde 型付き JSON/YAML/TOML"
created: "2026-11-04"
updated: "2026-11-04"
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
      - type: "file"
        path: "docs/specs/issue-297-io-structured-json-yaml-toml.md"
      - type: "file"
        path: "docs/designs/issue-297-io-structured-json-yaml-toml.md"
      - type: "file"
        path: "docs/plans/issue-297-io-structured-json-yaml-toml.md"
      - type: "file"
        path: "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
      - type: "file"
        path: "docs/tasks/issue-297-t1-crate-and-contract.md"
      - type: "file"
        path: "docs/tasks/issue-297-t2-yaml-toml.md"
      - type: "file"
        path: "docs/tasks/issue-297-t3-file-feature.md"
      - type: "file"
        path: "docs/impl/ir/issue-297-ruviform-io-structured-serde-json-yaml-toml.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
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
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "docs/tasks/issue-297-t1-crate-and-contract.md"
    - "docs/tasks/issue-297-t2-yaml-toml.md"
    - "docs/tasks/issue-297-t3-file-feature.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/designs/issue-297-io-structured-json-yaml-toml.md"
    - "docs/plans/issue-297-io-structured-json-yaml-toml.md"
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
    - "https://github.com/scarecrow173/ruviform/issues/284"
    - "https://github.com/scarecrow173/ruviform/issues/288"
    - "https://github.com/scarecrow173/ruviform/issues/298"
    - "https://github.com/scarecrow173/ruviform/issues/299"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-297-io-structured-json-yaml-toml.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 297 ruviform-io-structured — Serde 型付き JSON/YAML/TOML

## Summary

IO-IMP-03（Issue #297）として、構造化ドキュメント I/O 基盤
`ruviform-io-structured` を新設した。IO-02（Issue #284）設計どおり、
`json` / `yaml` / `toml` の各 feature ゲート module が共通形の
`from_reader<T>` / `to_writer<T>` を提供し、単一文書 Strict EOF・
`InputLimits` byte budget・正規化 `DocumentError`・`file` feature の
`StagedFile` publish 合成を実装した。公開 API に出る外部型は
`serde::{Serialize, DeserializeOwned}` のみ — backend
（serde_json 1.0.154・serde-saphyr 2.0.0・toml 1.1.8）の型・エラー・
feature 名は一切露出せず、診断は `Error::source()` の `dyn Error`
経由のみ。後続の IO-IMP-04 CSV（#298）・IO-IMP-05 XML（#299）が載る
scaffold として、format 横断の `DocumentError`/`DocumentFormat`/
`DocumentPosition`/`DocumentLimit`/`InputLimits` を汎用に確立した。

## Context

- 実施タスク: [T1](../../tasks/issue-297-t1-crate-and-contract.md) /
  [T2](../../tasks/issue-297-t2-yaml-toml.md) /
  [T3](../../tasks/issue-297-t3-file-feature.md)
- 上流: [plan](../../plans/issue-297-io-structured-json-yaml-toml.md)、
  [design](../../designs/issue-297-io-structured-json-yaml-toml.md)、
  [spec](../../specs/issue-297-io-structured-json-yaml-toml.md)、
  [test-spec](../../test-specs/issue-297-io-structured-json-yaml-toml.md)、
  親設計 Issue #284（IO-02）・統合 #282・共通 gate #288・cross-repo #219
- 前提契約: IO-IMP-01/02（#295/#296）の `StagedFile`/`SealedFile`/
  `PublishOptions`/`PublishFailure` — `file` feature が compose する。
- 境界: consumer schema の所有・意味・移行・byte-canonicalization は
  製品側責務（本クレートは形態を一切決めない）。

## Implementation

### Changes

- `src/error.rs` — `DocumentError`（kind + format + `position` +
  `limit` + type-erased `source`）、`DocumentErrorKind`（`Io`/
  `InvalidEncoding`/`Syntax`/`DataModel`/`TrailingData`/`LimitExceeded`/
  `Unsupported`）、`DocumentFormat`（Json/Yaml/Toml — Csv/Xml は
  後続 Issue で拡張）、`DocumentPosition`、`DocumentLimit`。
  `Display` は backend レンダリングや入力 snippet を一切埋め込まない。
- `src/limits.rs` — `InputLimits{max_bytes}`（既定 16 MiB）。
- `src/input.rs` — `BoundedRead`（budget 消尽後の 1 byte probe で
  「丁度 budget の文書は受理・1 byte 超過は拒否」を実現）、
  `read_to_string`（TOML 用有限 buffer + UTF-8 検証 →
  `InvalidEncoding`）。
- `src/json.rs` — `serde_json::Deserializer` の `end()` で strict
  EOF を検査（`TrailingCharacters` → `TrailingData`）。
  `serde_json::error::Category` 分類（Io/Syntax+Eof→Syntax/Data→
  DataModel）。depth は backend 固定上限に従い `Syntax` とする —
  `LimitExceeded` への虚偽分類はしない旨を API 文書に明記。
- `src/yaml.rs` — `serde_saphyr::from_reader_with_options` +
  `Budget`（`max_reader_input_bytes`/`max_depth`/`max_aliases` を
  `YamlReadOptions` から配線、その他 budget は backend 有限既定値）。
  `with_snippet=false` で backend 診断に入力 snippet を混ぜない。
  `MultipleDocuments` → `TrailingData`、`Budget` → `LimitExceeded`
  （breach から limit 名と実効上限を `DocumentError::limit()` へ写像）、
  alias replay 系エラーも `LimitExceeded`、parser 由来は
  `ExternalMessage`/`Eof`/構造系 → `Syntax`、serde 由来
  （Serde*・InvalidScalar・NonFiniteFloat 等）→ `DataModel`、
  `UnsupportedTag`/policy 系 → `Unsupported`。未知 variant は
  non_exhaustive 前提で `Syntax` へ保守 fallback。
- `src/toml.rs` — 有限 buffer（`read_to_string`）→
  `toml::from_str::<Table>`（`Syntax`）→ `try_into::<T>`
  （`DataModel`）の 2 相分類。TOML 文法には「文書後の余剰」と
  「文法違反」の区別がないため、trailing data は `Syntax` とする旨を
  文書化（虚偽の `TrailingData` 分類をしない）。backend `Display` は
  入力行を snippet として含むため、`source()` には
  `BackendMessage`（`message()` テキストのみ）を載せる。
- `src/file.rs` — `DocumentFileReadError{Open,Decode}` /
  `DocumentFileWriteError{Create,Encode,Publish}`。
  `write_document` は `StagedFile::create` → encode → `seal()` →
  `publish(PublishOptions)` を合成し、`Publish` には生の
  `PublishFailure`（effect・phase・durability 証跡・RecoveryInfo・
  RecoveryHandle）を丸ごと保持する。
- `src/{json,yaml,toml}.rs` の `read_file`/`write_file`
  （`feature="file"` 時のみ）— `open_document`/`write_document`
  helper を呼ぶ薄い convenience。
- `tests/structured.rs` — 36 件の契約試験（format 毎に cfg ゲート、
  feature 無しでも空でコンパイル）。round-trip（struct/enum/入れ子/
  Unicode/非 ASCII key・pretty 両系）、strict EOF・trailing data、
  不正/途中切れ syntax、型不一致 DataModel、over-budget
  LimitExceeded、丁度 budget 境界の受理/拒否、YAML depth/alias
  budget の実効、TOML 非 UTF-8、file: Open/Decode/Encode/Publish
  分離と `PublishFailure` 状態保存・dest 非破壊。
- `Cargo.toml`（workspace members + 層コメント）・`crates/README.md`
  （新セクション「構造化ドキュメント I/O」+ 索引行）・crate README
  （責務/feature matrix/format 制約/RT 契約）。

### Decisions During Implementation

- backend 選択: serde_json `=1.0.154`・serde-saphyr `=2.0.0`・
  toml `=1.1.8` に `=` pin — serde-saphyr 2.1.0（2026-11-03）と
  toml 1.2.0（2026-11-04）は 7 日経過ポリシーに抵触するため。
  toml の version 指定から build metadata は除去（semver 比較で
  無視される旨 cargo warning）。
- `DocumentError` の `Display` は kind/format/position/limit のみ —
  backend `source()` 診断に入力 snippet が混じり得る format では
  backend 側でも抑止（YAML `with_snippet=false`、TOML は
  `BackendMessage` wrapper で message のみ保持）。
- `BoundedRead` は budget 消尽後に 1 byte probe で真の EOF かを
  判別 —「丁度 budget」「budget+1 byte（空白でも）」を正確に分離。
  trip 検出は adapter 側 flag（`was_limit_exceeded`）を正本として
  backend が wrap した io error を `LimitExceeded` に再分類する。
- `YamlReadOptions` に `max_depth`/`max_aliases` を露出 — backend の
  実 budget 機構にマップできるため正直な保証として提供。
  anchor/event/node/doc/scalar byte/merge key/replay 各 budget は
  backend 既定値を維持し、公開 knob 化は行わない（YAGNI）。
- `toml::from_str::<Table>` → `try_into::<T>` の 2 相 — 文法違反と
  型不一致を位相で分離。`toml::de::Error` の span は byte offset と
  して `position.byte` へ写像。
- `file` feature の error 設計: `Open` は `std::io::Error`、
  `Create` は `ruviform-io-file::FileError`、両者で別 variant — plan
  記録どおり、`read_file` は io-file 契約外（std::fs 直接）である旨を
  維持。
- `#[non_exhaustive]` 外部構築不能のため options は `with_*`
  builder を提供（`with_limits`/`with_pretty`/`with_max_depth`/
  `with_max_aliases`）。

## Related Experiments

### Adopted

- serde-saphyr 2.0.0（YAML backend）— `serde_yaml` 系の archive 状態を
  避け、budget 機構・MSRV 1.89・serde trait 境界を満たす唯一の
  実用選択肢。`default-features=false` + `serialize`/`deserialize` で
  最小導入。
- `serde_json::Deserializer::end()` — strict EOF の実証手段。
  `TrailingCharacters` が確定的に検出可能な唯一の API で、
  自前の tail-scan を不要にした。

### Rejected

- serde_yaml 0.9.x — unmaintained（archive）のため採用見送り。
- unsafe-libyaml — `unsafe` 原則禁止ポリシーに反するため採用見送り。
- JSON depth の `InputLimits` 公開 — serde_json が固定内部上限しか
  持たず、公開 knob 化は虚偽の保証になるため `Syntax` 分類 + 文書化
  で対応。
- YAML `max_documents=1` 設定 — `MultipleDocuments` を `TrailingData`
  として検出するため backend 既定（1024）を維持し、単一文書 API の
  責務として複数文書拒否を分類した。

## Validation

実施済み（Windows 11 x64 / mise 管理 toolchain rustc 1.99.0）:

- `mise run fmt` — `cargo fmt --all` 適用、全ソース正規化済み。
- `mise run check` — `cargo check --workspace --all-targets` 全緑。
- `mise run lint` — `cargo clippy --workspace --all-targets --
  -D warnings` 全緑（`pedantic` + `exhaustive_enums` を含む）。
- `mise run test` — `cargo test --workspace` 全緑（34 crates）。
- `cargo test -p ruviform-io-structured --all-features` — 36 契約試験
  + 9 doctest 全緑。
- `cargo test -p ruviform-io-structured --no-default-features
  --features <f>` — `json`/`yaml`/`toml`/`json,file`/`yaml,file`/
  `toml,file` 各組合せで contract test 全緑。
- `cargo check -p ruviform-io-structured --no-default-features
  [--features <f>]` — 無 feature・`file` 単体を含む全組合せで警告 0。
- `cargo +1.89 check -p ruviform-io-structured --all-features
  --all-targets` — MSRV 1.89 toolchain で全 feature コンパイル通過。

未実施・補足:

- `cargo test` の feature matrix 実行は workspace `test` task
  （default features のみ）では補助証拠として実施した。
- macOS/Linux の実 OS/実 FS 検証は CI lane に委譲。Windows 実 FS での
  `file` feature 試験は tests/structured.rs で実施済み。
- Manifest/Project byte-canonicalization の golden test 等価性は
  consumer 側の責務であり本クレートは検証対象外。

## Risks

- serde-saphyr は若い crate で、alias/tag の細かい semantics は
  backend 版間で揺れ得る。`=` pin で固定し、version bump は
  7 日経過ポリシーで評価する。
- YAML の unsupported tag は backend policy 上「tolerate または
  拒否」の両立動作 — 本クレートは `UnsupportedTag` を `Unsupported`
  kind として honest に報告するが、tag 使用入力の受理可否は backend
  の policy（`reject_unsupported_tags` 既定 false）に従う。
- `Budget` 以外の内部構造制約（granit-parser の block/flow nesting
  limit 等）は `BudgetBreach` の `Syntax` fallback 側に回る — 実効的
  に parser の内部保護として働く。

## Follow-ups

- IO-IMP-04 CSV（Issue #298）・IO-IMP-05 XML（Issue #299）— 本
  scaffold の format module として追加（`DocumentFormat` variant・
  `DocumentError` 語彙・`InputLimits`・`file` feature composition は
  そのまま再利用可能）。
- Issue #288（IO-06 共通 gate）・#219（cross-repo）での consumer 受入
  統合検証。
- `YamlWriteOptions`/`TomlWriteOptions` が空に近いため、実 consumer
  で必要になった時点で emitter knob を追加検討（YAGNI で今回は最小）。
