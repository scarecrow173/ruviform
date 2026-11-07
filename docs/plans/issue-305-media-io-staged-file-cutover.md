---
id: "PLAN-gWrX94JcoMQacCOPFR5FKD"
type: "plan"
status: "approved"
title: "Issue #305 Media I/O 旧 StagedOutput 撤去・StagedFile 移行 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/287"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/sink.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/render.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-wav/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/src/muxer.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/README.md"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/contract.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted:
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/designs/issue-305-media-io-staged-file-cutover.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 Media I/O 旧 StagedOutput 撤去・StagedFile 移行 実装計画

## Goal

`ruviform-media-io` の旧 `staged` 実装を互換層なしで撤去し、
`ruviform-audio::render`・`ruviform-io-wav` tests・関連 rustdoc/README を
`ruviform-io-file` の `StagedFile`/`SealedFile`/`PublishFailure` 契約へ
移行する。正規検証を緑にし、候補 revision として PR を作成する。

## Scope

単一実装ストリーム・1 plan・1 task（撤去と移行は不可分 —
`staged.rs` を消すと全 call site が壊れるため同一コミット系列で行う）。
変更は File Map 記載のクレート内ファイル + `Cargo.lock` +
本 doc チェーンに限定。Consumer repo・`ruviform-io-file` の
新契約追加は対象外。

## Upstream Documents

- `docs/specs/issue-305-media-io-staged-file-cutover.md`（approved）
- `docs/designs/issue-305-media-io-staged-file-cutover.md`（approved —
  対応表・D1–D5）
- Issue #305 本文・Issue #287（IO-05）対応表・#295/#296 実装済み API

## Design Inputs

- IO-05 #287 の新旧対応表・順序契約・非互換方針。
- `ruviform-io-file` 公開 API: `StagedFile: Write+Seek+Send`・
  `seal()`→`SealedFile`・`open_readback()`→`ReadbackFile`（Read+Seek）・
  `publish(PublishOptions)`→`Result<PublishReceipt, PublishFailure>`・
  `PublishOptions::new(PublishMode, Durability)`。
- `ruviform-audio/src/render.rs` 現行構造（`RenderOutput` 注入・
  `run_pipeline` の drain→finish→verify→publish テール）。

## File Map

- `crates/ruviform-media-io/src/staged.rs` — 削除
- `crates/ruviform-media-io/src/lib.rs` — `mod staged`・re-export 撤去
- `crates/ruviform-media-io/src/sink.rs` — durability 責務の言及更新
- `crates/ruviform-audio/Cargo.toml` — `ruviform-io-file` 依存追加・
  コメント更新
- `crates/ruviform-audio/src/render.rs` — `StagedFile` 移行
  （`RenderOutput`/`render`/`run_pipeline`/`RenderError`/`RenderReceipt`）
- `crates/ruviform-audio/tests/render.rs` — 新 API へ移行
  （`MuxerCtor`/`PublishFn`/`VerifyFn`・`FailSink` 汎用化・publish
  失敗注入は実 FS 失敗へ）
- `crates/ruviform-io-wav/Cargo.toml` — `ruviform-io-file` dev 依存追加
- `crates/ruviform-io-wav/src/lib.rs`・`src/muxer.rs`・`README.md` —
  `StagedOutput` 言及を `ruviform-io-file` 契約へ更新
- `crates/ruviform-io-wav/tests/contract.rs` — 新 API へ移行 +
  finalize→seal→readback 独立 decode→publish→再 Open E2E 追加
- `Cargo.lock` — `ruviform-io-file` edge 追加の再解決（正規入口経由）

## Implementation Sequence

- [ ] 1. `ruviform-io-wav` contract test を新 API へ移行 + E2E 追加
       （`WavMuxer<&mut StagedFile>` の RED→GREEN 確認）
- [ ] 2. `ruviform-audio` render.rs + tests/render.rs を移行
       （`ruviform-io-file` 依存追加）
- [ ] 3. `ruviform-media-io` の `staged` 撤去 + `sink.rs` doc 更新
- [ ] 4. rustdoc/README 更新（io-wav）・`Cargo.lock` 再生成
- [ ] 5. `git fetch origin` → `git merge origin/main` →
       `mise run fmt`/`check`/`lint`/`test` → コミット → push → PR

## Dependencies and Risks

- `RenderError::Publish { source: PublishFailure }` への変更で
  `RenderError` の `Display`/`source()` を調整。`RenderReceipt` から
  `Clone` derive を除去する破壊的変更 — spec の Deferred Design
  Concerns に記録済み。
- `verify` hook が `&mut SealedFile` を受ける形 — hook 内の独立
  decode は readback bytes → `Cursor<Vec<u8>>` で構成（`ReadbackFile`
  は `Send`/`'static` でない）。`FnOnce(&mut SealedFile)` 契約で
  publish 迂回を型で排除。
- publish 失敗注入: `PublishFailure` は crate 外で構築不能
  （`pub(crate)` ctor）のため、実 FS 失敗（宛先=既存 dir で Replace
  publish → `NotPublished`/Io）を注入する。これは mock より強い
  実挙動証拠となる。
- 並行 worktree: 共有 `Cargo.lock` のみ触る。最終検証前に
  `origin/main` を取り込み再解決する。

## Task Breakdown

- T1（`docs/tasks/issue-305-t1-staged-file-cutover.md`）: 上記全工程
  （移行 + 撤去 + 検証 + IR）。

## Verification

- [ ] `mise run fmt` → `mise run check` → `mise run lint` → `mise run test`
      （workspace 全体、`CARGO_BUILD_JOBS=2`）
- [ ] `cargo tree -p ruviform-media-io` で Domain Core3 edge のみ
      （補助証拠、canonical = mise）
- [ ] `StagedOutput`・media-io 旧 `PublishOptions` の workspace 検索
      ゼロ（`docs/` 歴史資料は対象外）
- [ ] test-spec の試験一覧が全て緑（WAV E2E・render 失敗行列・
      新 API contract）

## Open Questions / Gaps

- 4GiB 境界試験は既存 contract test の sink-side 試験で維持
  （`data_bytes + length > u32::MAX` で `InvalidData`）— 実 4GiB
  ファイルを生成する試験は従来どおり存在しない（未測定を PASS に
  しない）。
- macOS/Linux での実機検証は本環境（Windows）では NotRun —
  io-file OS backend の実測は #296 側の Gate を参照。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本 worker が実装）
- Independent work streams: none
- User approval required before dispatch: review-protocol で代替承認
- If approved: execute task with review checkpoints.

## Review Handoff

- 焦点: `make_muxer` の借用型変更、`RenderError::Publish` の
  `PublishFailure` 保持、非 Indeterminate 時の recovery cleanup、
  `verify` が `&mut SealedFile` で publish 迂回不能であること、
  `RenderReceipt` の `Clone` 除去。
