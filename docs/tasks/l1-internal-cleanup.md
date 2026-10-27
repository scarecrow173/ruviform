---
id: "TASK-034N3QIPDPFbB37OxHMZLy"
type: "task"
status: "done"
title: "Issue #123 L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/123"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-audio-core/src/planar.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
  implemented-by:
    - "docs/impl/ir/issue-123-l1-internal-cleanup-implementation.md"
  depends-on:
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
    - "docs/tasks/plugin-l1-closeout.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
    - "docs/specs/issue-76-followup-l1-internal-cleanup.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #123 L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正）

## Work

plan `PLAN-034N3PvP9HH1MlVyewXBHB` の全 9 actions。Issue #76 IR の小粒残件 4 項目を 1 task で集約解消する（`small-fixes-from-followup-triage` 先例）。外部契約（wire 形式・shm レイアウト・HostReply・exit code）は不変。

- [ ] `crates/ruviform-plugin-core/src/wire.rs` に frame 輸送 helper を `pub` で追加: `read_frame(reader, buf) -> io::Result<bool>`（`Ok(true)` 完全 frame・`Ok(false)` boundary clean EOF・`Err` 切断/limit/io — export `Fill`/`ReadFail` 意味論を `io::Error` へ写像）、`read_message(reader, buf) -> io::Result<Message>`（boundary EOF → `UnexpectedEof`）、`write_message(writer, message, buf) -> io::Result<()>`（encode + write_all + flush）
- [ ] `l1/mod.rs` 私有 `read_message`/`write_message` 削除 → `wire::` 消費（`reader_main`/`driver_main` に `frame_buf`/`encode_buf` scratch 確保）
- [ ] `l1-test-worker.rs` 私有同名 helper 削除 → `wire::` 消費（per-call `Vec::new()` scratch、signature 不変）
- [ ] `export/src/lib.rs` 私有 `read_frame`/`send`/`fill`/`ReadFail` 削除 → `serve` を `wire::read_frame` + `wire::decode` + `wire::write_message` 消費へ（`undecodable frame` 診断・終了コード・`normalize` 経路を保存）
- [ ] `export/tests/l1_runner_e2e.rs` `read_message`/`MiniHost::send` を `wire::` 消費へ
- [ ] `l1_proxy_e2e.rs` `runner_program()` を `RUVIFORM_L1_RUNNER` env（`var_os`）→ `current_exe` 相対（exe dir → deps/ 親、`is_file` 検査）fallback へ強化
- [ ] `host.rs` `Instance.gui_open` 除去（宣言・`new` 初期化・`open_gui`/`close_gui` の 4 write — read 不在は全量 grep 確証済み）
- [ ] `planar.rs:123,243` intra-doc link を `[`Error::InvalidValue`] (`"planar_frames"`)` へ

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-76-followup-l1-internal-cleanup.md`（plan — actions 1–9）
- `docs/specs/issue-76-followup-l1-internal-cleanup.md`（spec — 要件・scope）

## Dependencies

- `docs/tasks/plugin-l1-closeout.md`（#76 closeout — follow-up の発生元・完了済み）

## Affected Files

- `crates/ruviform-plugin-core/src/wire.rs`（helper 追加）
- `crates/ruviform-plugin-host/src/l1/mod.rs`・`src/bin/l1-test-worker.rs`・`src/host.rs`・`tests/l1_proxy_e2e.rs`
- `crates/ruviform-plugin-export/src/lib.rs`・`tests/l1_runner_e2e.rs`
- `crates/ruviform-audio-core/src/planar.rs`

## Done When

- [ ] wire 層 helper が `pub` で定義され、4 箇所の私有 framing 実装が全て消費側へ転換されている（同形状ロジック残存なし）
- [ ] `runner_program()` が `RUVIFORM_L1_RUNNER` を受理し、未設定でも現行どおり runner を解決する
- [ ] `grep -rn "gui_open" crates/` が 0 件
- [ ] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件

## Verification

- [ ] `mise run fmt-check` exits 0
- [ ] `mise run check` exits 0
- [ ] `mise run lint` exits 0
- [ ] `mise run test` exits 0
- [ ] `mise run test-rt` exits 0
- [ ] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件で exits 0
- [ ] `grep -rn "gui_open" crates/` が 0 件
