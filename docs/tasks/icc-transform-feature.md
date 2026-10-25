---
id: "TASK-034LFHl0gbFixJl7RmlzQm"
type: "task"
status: "done"
title: "IccTransform + IccIntent 実装（feature icc・moxcms 経路）"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/srgb.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/display_p3.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/gray_gamma22.icc"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".gitattributes"
      - type: "file"
        path: "docs/test-specs/icc-transform-contract-tests.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by:
    - "docs/impl/ir/issue-99-icc-transform-task-4-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# IccTransform + IccIntent 実装（feature icc・moxcms 経路）

## Work

approved plan `implement-issue-99-color-conversion-kernels.md` の **actions 9–11 を 1 task の ordered substeps として実行**。optional dependency `moxcms 0.9` + feature `icc` の配線 → `icc.rs` の `IccIntent`/`IccTransform`（moxcms 閉じ込め）→ feature 専用 verification lane（mise task + CI ジョブ）。

**先行確認**: `moxcms 0.9` の実効 API 形状（`create_transform_f32`・`Layout::Rgba`・`TransformOptions` フィールド・`Intent` 値域）を docs.rs/`cargo doc` で確認 — design 記載と相違なら blocker を記録し設計判断に戻す（fake success なし・代替実装の独自発明禁止）。

### A9 — Cargo feature 配線

- `crates/ruviform-video-proc/Cargo.toml`:
  - `moxcms = { version = "0.9", optional = true, default-features = false, features = ["lut", "extended_range"] }` — `in_place`/`options`/`any_to_any`/SIMD 系は有効化しない（design Non-Functional Constraints どおり）
  - `[features]` `icc = ["dep:moxcms"]` — `default = []` 維持（feature-off 時は外部依存ゼロ）
  - `cargo build` で `Cargo.lock` 更新（追跡継続）
- 依存追加の理由（保守状況・ライセンス BSD-3-Clause OR Apache-2.0・依存木 3 crate・MSRV 適合）は design 記録を根拠として commit/PR に記録 — 暫定規約「依存追加」

### A10 — `icc.rs`（feature-gated）

- `crates/ruviform-video-proc/src/icc.rs` 新設 — ファイル全体を `#[cfg(feature = "icc")]` 配下に配置（`lib.rs` の `#[cfg(feature = "icc")] mod icc`/`pub use`）
- `IccIntent`（`#[non_exhaustive]` 公開 enum）— `Perceptual`/`RelativeColorimetric` 等 design の intent 集合を moxcms `Intent` へ内部写像（moxcms 型を公開しない）
- `IccTransform` — `new(src_profile: &[u8], dst_profile: &[u8], intent: IccIntent) -> Result<Self>`:
  - ctor で profile parse（失敗 → `InvalidValue("icc profile")`）+ executor 焼付け（transform 構築失敗 → `InvalidValue("icc transform")`）— 非 RT by convention・`process` は alloc-free
  - `process(src, dst)` — src/dst format ∈ {`Rgba8`, `RgbaF32`} 端点組合せ（同一可）・同一寸法・排他所有 — 既存エラー文字列 `"src format"`/`"dst format"`/`"dimensions"`/`"dst plane"` 再利用
  - per-pixel 変換（`moxcms::Transform::transform` 等を 1 要素スライスで呼出 — 行バッファ確保なし・alloc-free）・alpha は straight パススルー（premultiply 回避）
  - `dst.color` = `{primaries: Unspecified, transfer: Unspecified, matrix: Identity, range: Full, hdr: None}`（CICP 値は ICC 変換で保証不可 — Unspecified に落とす）・`pts`/`duration` 伝播
  - `CmsError` 等 moxcms 型は `Error::InvalidValue` に map — 公開 API に moxcms 型/エラー/feature 名を露出しない
- `lib.rs` カーネル一覧へ feature 記述付きで追加
- inline tests（`#[cfg(feature = "icc")]`）— test-spec `icc-transform-contract-tests.md` 契約: bit-exact moxcms golden（pin 版・同 feature 構成のスカラー executor と同一条件で導出）・intent 写像・端点組合せ・alpha パススルー・metadata・拒否群（不正 profile・shared dst・寸法/格式不一致・padding）・source immutability・`rt_process_*`

### A11 — feature 専用 lane

- `mise.toml` に `test-video-proc-icc`（`cargo test -p ruviform-video-proc --features icc`）・`lint-video-proc-icc`（`cargo clippy -p ruviform-video-proc --all-targets --features icc -- -D warnings`）を追加 — plan「Required mise tasks」節の定義どおり
- `.github/workflows/ci.yml` に `icc-verify` ジョブ（`runs-on: ubuntu-latest`・rt-verify と同形の mise セットアップ・`CARGO_INCREMENTAL=0`）で上記 2 タスクを実行
- ベース経路無退行: `mise run test`・`mise run lint`（feature-off ワークスペース検証が green 維持・`cargo tree -p ruviform-video-proc` に moxcms が出ない）

## Completion Gate

3 entry 全 green 実 log（feature-on lane 2 タスク + ベース `test`/`lint`）+ lead review の後にのみ task 完了・buildable scope commit。

## Verification

- [x] `mise run test-video-proc-icc` — exit 0（feature-on テスト全 pass — lib 163/163・doctests 24/24・bit-exact golden 一致・実 log）
- [x] `mise run lint-video-proc-icc` — exit 0（clippy `--all-targets --features icc -D warnings` clean）
- [x] `mise run test` / `mise run lint` — exit 0（feature-off ベース経路無退行・実 log）
- [x] `cargo tree -p ruviform-video-proc` — feature-off ツリーに `moxcms` を含まない（media-core/video-core のみ実確認）
- [x] `rg -n 'cfg.*feature = "icc"' crates/ruviform-video-proc/src/lib.rs` — mod/re-export が feature-gated（L137 `mod icc`・L148 `pub use`）
- [x] `rg -n "icc" crates/ruviform-video-proc/Cargo.toml` — optional dep + `icc = ["dep:moxcms"]` 定義（L13–24・L27）
- [x] `rg -n "test-video-proc-icc|lint-video-proc-icc" mise.toml .github/workflows/ci.yml` — lane 定義が両方に存在（mise.toml L125/L128・ci.yml L147–151）
