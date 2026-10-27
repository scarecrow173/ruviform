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

- [x] `crates/ruviform-plugin-core/src/wire.rs` に frame 輸送 helper を `pub` で追加: `read_frame(reader, buf) -> io::Result<bool>`（`Ok(true)` 完全 frame・`Ok(false)` boundary clean EOF・`Err` 切断/limit/io — export `Fill`/`ReadFail` 意味論を `io::Error` へ写像）、`read_message(reader, buf) -> io::Result<Message>`（boundary EOF → `UnexpectedEof`）、`write_message(writer, message, buf) -> io::Result<()>`（encode + write_all + flush）（実績: EOF 意味論の doc + doctest ×3 同梱・`cargo test` で検証済み）
- [x] `l1/mod.rs` 私有 `read_message`/`write_message` 削除 → `wire::` 消費（`reader_main`/`driver_main` に `frame_buf`/`encode_buf` scratch 確保）（実績: reader の EOF/違反は converge 経路不変・`WIRE_FRAME_HEADER_LEN`/`WIRE_MAX_PAYLOAD` の残存 import なし）
- [x] `l1-test-worker.rs` 私有同名 helper 削除 → `wire::` 消費（per-call `Vec::new()` scratch、signature 不変）（実績: `block_in`/`block_in_dispatch` 系 signature 不変で消化）
- [x] `export/src/lib.rs` 私有 `read_frame`/`send`/`fill`/`ReadFail` 削除 → `serve` を `wire::read_frame` + `wire::decode` + `wire::write_message` 消費へ（`undecodable frame` 診断・終了コード・`normalize` 経路を保存）（実績: boundary clean EOF → 正常終了・mid-frame truncation → 非ゼロ終了の分岐を `io::Error` kind で保存）
- [x] `export/tests/l1_runner_e2e.rs` `read_message`/`MiniHost::send` を `wire::` 消費へ（実績: reader thread は `wire::read_message`・`send` は `wire::write_message` への薄い委譲として残置）
- [x] `l1_proxy_e2e.rs` `runner_program()` を `RUVIFORM_L1_RUNNER` env（`var_os`）→ `current_exe` 相対（exe dir → deps/ 親、`is_file` 検査）fallback へ強化（実績: env override + dual-layout fallback・不在時 panic は `RUVIFORM_L1_RUNNER` 名と探索した両候補 path を明記）
- [x] `host.rs` `Instance.gui_open` 除去（宣言・`new` 初期化・`open_gui`/`close_gui` の 4 write — read 不在は全量 grep 確証済み）（実績: `rg gui_open crates/` = 0 件・`open_gui`/`close_gui` の reply・hook 呼出し不変）
- [x] `planar.rs:123,243` intra-doc link を `[`Error::InvalidValue`] (`"planar_frames"`)` へ（実績: `cargo doc -p ruviform-audio-core --no-deps` warning 0 件）

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

- [x] wire 層 helper が `pub` で定義され、4 箇所の私有 framing 実装が全て消費側へ転換されている（同形状ロジック残存なし）（実績: `rg "fn read_message|fn write_message|fn read_frame|fn send|fn fill" crates/` で frame 実装は `wire.rs` の 3 `pub fn` のみ — `l1_runner_e2e` の `fn send` は `wire::write_message` への薄い委譲・`ruviform-dsp` の `fill` は無関係な既存 API）
- [x] `runner_program()` が `RUVIFORM_L1_RUNNER` を受理し、未設定でも現行どおり runner を解決する（実績: env override + exe dir/deps 親 dual-layout fallback — 未設定で `composite_e2e::runner_bin_serves_real_gain_plugin` が real runner binary 経路で pass）
- [x] `grep -rn "gui_open" crates/` が 0 件（実績: 0 件）
- [x] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件（実績: exit 0・warning 0 — `planar.rs` の 2 warning 解消）

## Verification

- [x] `mise run fmt-check` exits 0（実績: exit 0・差分なし）
- [x] `mise run check` exits 0（実績: exit 0）
- [x] `mise run lint` exits 0（実績: exit 0 — 初回 run で `l1-test-worker` `main()` `too_many_lines` 106/100 を検出 → 局所 `#[allow]` + 理由コメントで clean）
- [x] `mise run test` exits 0（実績: exit 0 — 初回 run で `l1_proxy_e2e` の 2 test が既知の負荷依存 flake（`Err(Ipc)`）→ 孤立 rerun 各 pass + suite 再実行で全 green。wire doctest ×3 含む）
- [x] `mise run test-rt` exits 0（実績: exit 0 — `rt_` レーン無退行・本 change は非 RT の stdio/制御面のみ）
- [x] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件で exits 0（実績: exit 0・warning 0）
- [x] `grep -rn "gui_open" crates/` が 0 件（実績: 0 件）

補足（review 対応で追加・本 task の元 scope 外の小修正）: `wire.rs` に `read_frame` の Err taxonomy を pin する unit test 追加（clean-EOF→`Ok(false)`・partial header→`UnexpectedEof`・over-limit `payload_len`→`InvalidData` — runner exit-0/exit-1 契約の load-bearing な分岐）、`runner_program()` 不在時 panic を探索した両候補 path 名指しへ改善、`l1-test-worker` の `#[allow(clippy::too_many_lines)]` に理由コメント、`join_threads` doc の `read_exact` 表記を `wire::read_message` へ修正。
