---
id: "PLAN-034N3PvP9HH1MlVyewXBHB"
type: "plan"
status: "approved"
test-spec-skip: "internal cleanup Issue — wire 契約・外部観測挙動は不変のリファクタ + dead code 除去 + doc 修正のみ。検証は既存 L1 テスト（wire codec / runner e2e / proxy contract がそのまま回帰）+ 検証列で行う"
title: "Issue #123 実施計画 — Issue #76 フォローアップ: L1 内部クリーンアップ"
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
    - "docs/specs/issue-76-followup-l1-internal-cleanup.md"
  implemented-by:
    - "docs/tasks/l1-internal-cleanup.md"
  depends-on:
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
    - "docs/specs/issue-76-followup-l1-internal-cleanup.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-76-followup-l1-internal-cleanup.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #123 実施計画 — Issue #76 フォローアップ: L1 内部クリーンアップ Implementation Plan

## Goal

Issue #76 IR の小粒残件 4 項目（frame-I/O dedup・`runner_program` 解決強化・`gui_open` 除去・audio-core doc 警告）を、wire 層への 1 authoritative helper 格上げと dead code 除去で解消する単一クリーンアップストリームを実施する。

## Scope

単一 plan・単一 task。4 項目はいずれも公開契約を変えない内部整理で、同一検証列（`check`/`lint`/`test`/`test-rt` + `cargo doc`）を共有するため分割しない。新規クレート・新規依存・feature 追加なし。

## Upstream Documents

- `docs/specs/issue-76-followup-l1-internal-cleanup.md`（approved — 4 項目の要件・scope 境界）

## Design Inputs

- `docs/designs/plugin-l1-out-of-process.md`（approved — L1 wire/shm 契約の正本。本 cleanup は wire 形式・shm レイアウト・exit code を変えない）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（approved — fixture 解決先例「env → `current_exe` sibling」および「#123 が解決形を強化すればそれに追従」の依存明記）

## File Map

- `crates/ruviform-plugin-core/src/wire.rs` — frame 輸送 helper（`read_frame`/`read_message`/`write_message`）を `pub` で追加。frame format（`[u32 LE payload_len][u8 kind][payload]`・`WIRE_MAX_PAYLOAD`）の authoritative 所有者として輸送層も所有する
- `crates/ruviform-plugin-host/src/l1/mod.rs` — 私有 `read_message`/`write_message` を削除し `wire::` 呼出しへ。`reader_main`/`driver_main` で `frame_buf`/`encode_buf` を確保
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — 同上（fixture 側の呼出しは per-call `Vec::new()` scratch で簡潔に）
- `crates/ruviform-plugin-export/src/lib.rs` — 私有 `read_frame`/`send`/`fill`/`ReadFail` を削除。`serve` は `wire::read_frame`（EOF 区別保存）+ 既存 `wire::decode` + `wire::write_message` へ
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — `read_message` を `wire::read_message` 消費に、`MiniHost::send` を `wire::write_message` 消費に置換
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — `runner_program()` を `RUVIFORM_L1_RUNNER` env override → `current_exe` 相対（exe dir + deps/ 親）fallback へ強化
- `crates/ruviform-plugin-host/src/host.rs` — `Instance.gui_open` 宣言・初期化・4 箇所の write を除去
- `crates/ruviform-audio-core/src/planar.rs` — `` [`Error::InvalidValue("planar_frames")`] `` ×2 を `[`Error::InvalidValue`]`（+ payload の inline code）へ修正

## Implementation Sequence

- [x] action 1: `wire.rs` に `read_frame`（`Ok(true)` frame/`Ok(false)` boundary EOF/`Err` 切断・limit・io）、`read_message`（`read_frame` + `decode`・boundary EOF → `UnexpectedEof`）、`write_message`（`encode` + `write_all` + `flush`・共有 scratch buf）を `pub` で追加。`std::io` Read/Write 前提・doc に EOF 意味論と non-RT 契約を明記（実績: 3 helper + doctest ×3 — review で Err taxonomy pin の unit test を追加）
- [x] action 2: `l1/mod.rs` の私有 helper を削除し呼出しを `wire::` へ（`reader_main` に `frame_buf`・`driver_main` に `encode_buf` を確保し、per-message alloc を現行どおり再使用へ転換）。`WIRE_FRAME_HEADER_LEN`/`WIRE_MAX_PAYLOAD` import の残存要否を確認（実績: 私有 helper 削除・両定数の残存 import なし）
- [x] action 3: `l1-test-worker.rs` の私有 helper を削除し呼出しを `wire::` へ（`block_in`/`block_in_dispatch` 系の signature 変更は避け、per-call scratch を使う）（実績: per-call `Vec::new()` scratch・signature 不変）
- [x] action 4: `export/src/lib.rs` の `read_frame`/`send`/`fill`/`ReadFail` を削除。`serve` の read 分岐を `wire::read_frame` + `wire::decode` に写像（`ReadFail` の 3 診断は `io::Error` kind/message で等価表現、終了コード・normalize 経路は不変）（実績: EOF 区別・oversized 確保前拒否・exit code 契約を保存）
- [x] action 5: `l1_runner_e2e.rs` の `read_message`/`MiniHost::send` を `wire::` 消費へ置換（実績: reader は `wire::read_message`・`send` は `wire::write_message` 委譲）
- [x] action 6: `l1_proxy_e2e.rs` の `runner_program()` を `RUVIFORM_L1_RUNNER` env（`var_os` verbatim）→ `current_exe` 相対（exe dir 自身→deps/ 親の順で `is_file` 検査）へ書き換え、欠落時の panic message に両経路を明記（実績: dual-layout fallback + panic に `RUVIFORM_L1_RUNNER` と探索した両候補 path を明記）
- [x] action 7: `host.rs` の `Instance.gui_open` を除去（宣言・`new` 初期化・`open_gui`/`close_gui` の 4 write）。read が存在しないことは全量 grep で確証済み（実績: `rg gui_open crates/` = 0 件・GUI 動詞の defined rejection 不変）
- [x] action 8: `planar.rs:123,243` の intra-doc link を `[`Error::InvalidValue`] (`"planar_frames"`)` へ修正（実績: `cargo doc -p ruviform-audio-core --no-deps` warning 0 件）
- [x] action 9: `mise run fmt` → `fmt-check`/`check`/`lint`/`test`/`test-rt` + `cargo doc -p ruviform-audio-core --no-deps`（warning 0）を通し、IR・index 更新・監査後に commit（実績: 全 gate exit 0・doc audit 0 findings・commit `0c8eec3` — lint は `l1-test-worker` `too_many_lines` 局所 allow・test は負荷依存 flake の孤立 rerun + suite 再実行で green）

## Dependencies and Risks

- `read_frame` の clean-EOF 区別は export runner の終了コード分岐が依存する唯一の非対称 semantics — header 4 byte 単位で「0 byte EOF」と「部分 header」を区別する実装を共有化で保全する（`read_exact` 一括では区別不能）
- `l1_proxy_e2e` の負荷依存 flake（既知）— 発生時は隔離再実行で切り分ける
- `gui_open` 除去は #115（GUI 設計・別 worktree 並行・未 merge）と座標が重なり得る — write-only 確証を取った上で除去し、#115 側が必要とする GUI 状態は同 Issue が自身の型で再導入する前提を IR に記録する

## Task Breakdown

- `docs/tasks/l1-internal-cleanup.md` — 全 9 actions を 1 task として実施（`small-fixes-from-followup-triage` 先例どおり follow-up 集約は 1 task に束ねる）

## Verification

- [x] `mise run fmt-check` が差分なしで終了する（実績: exit 0）
- [x] `mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全て成功する（実績: 全 exit 0 — `test` は `l1_proxy_e2e` 2 test の既知負荷依存 flake を孤立 rerun + suite 再実行で green 確認）
- [x] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件で完了する（実績: exit 0・warning 0）
- [x] `grep -rn "gui_open" crates/` が 0 件を返す（実績: 0 件）
- [x] `grep -rn "fn read_message\|fn write_message\|fn read_frame\|fn send" crates/ruviform-plugin-host crates/ruviform-plugin-export` が私有実装残存なしを示す（実績: frame 実装は `wire.rs` の 3 `pub fn` のみ — `l1_runner_e2e` の `fn send` は `wire::write_message` への薄い委譲）
- [x] `RUVIFORM_L1_RUNNER` 未設定で `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` の runner 消費テストが pass する（実績: `composite_e2e::runner_bin_serves_real_gain_plugin` 等が dual-layout fallback 解決で pass）

## Open Questions / Gaps

- なし — 4 項目とも一次証拠（IR follow-up・grep 全量確認・#114 design 先例）が揃っている

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 task・外部契約変更なし・委任不要）
- Independent work streams: なし
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `wire` helper の EOF semantics（boundary EOF vs mid-frame 切断）が export の `ReadFail` 分岐と等価に写像されているかが主眼。`gui_open` 除去は read 不在の grep 証跡を IR に転記する
