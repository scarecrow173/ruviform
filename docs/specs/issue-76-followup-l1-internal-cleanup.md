---
id: "SPEC-034N3OsgUWx7hzkr8aicqq"
type: "spec"
status: "approved"
title: "Issue #76 フォローアップ: L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正）"
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
  implements: []
  implemented-by:
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
  depends-on:
    - "docs/specs/plugin-l1-out-of-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/plans/implement-issue-76-plugin-l1-out-of-process.md"
  derived-by:
    - "docs/plans/issue-76-followup-l1-internal-cleanup.md"
    - "docs/tasks/l1-internal-cleanup.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "docs/impl/ir/issue-76-l1-core-wire-task-1-implementation.md"
    - "docs/impl/ir/issue-76-l1-proxy-control-plane-task-3-implementation.md"
    - "docs/impl/ir/issue-76-plugin-host-composite-task-5-implementation.md"
  defers: []
  deferred-by: []
---

# Issue #76 フォローアップ: L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正）

## What Should Be Built

Issue #76 の各 IR に記録された小粒残件 4 項目を集約解消する（Issue #123）。いずれも外部契約・wire 形式・公開セマンティクスを変えない内部整理であり、L1 の defined contract は不変。

1. **frame-I/O helper dedup（4 箇所）** — `[u32 LE payload_len][u8 kind][payload]` フレームの read/write ロジックが `ruviform-plugin-host::l1` の `read_message`/`write_message`、`l1-test-worker` 同名 helper、`ruviform-plugin-export` の `read_frame`/`send`（+ `Fill`/`ReadFail`）、export テスト `l1_runner_e2e` の `read_message` と 4 箇所に同形状で散在している。frame format の所有者である `ruviform-plugin-core::wire` 層へ共通 helper（`read_frame`/`read_message`/`write_message`）を格上げし、4 箇所をその消費者にする。新規クレートは作らない。
2. **`runner_program()` 解決強化** — `l1_proxy_e2e.rs` の cross-package runner 解決（`ruviform-plugin-export` パッケージの `ruviform-plugin-l1-runner` bin）は `current_exe` → `target/<profile>/deps` → `target/<profile>` の相対解決で、workspace test entry に依存する。`CARGO_BIN_EXE_*` は same-package 限定のため cross-package 代替は存在せず（#114 でも同結論）、cargo metadata 起動 + JSON parse は重すぎるため不採用。#114 の fixture 解決先例（`RUVIFORM_L1_REAL_PLUGIN_DIR` env → `current_exe` sibling fallback）と同型に、env override + 強化した `current_exe` 相対 fallback へ改める。
3. **`gui_open` write-only field 除去** — `host.rs` `Instance.gui_open` は `open_gui`/`close_gui` で書き込まれるが全クレート範囲で read する箇所がない dead field（Task 1 IR の follow-up 記録）。read が一箇所も存在しないことを全量 grep で検証済みであり、#115（GUI 設計・別 worktree で並行作成中・未 merge）の verdict を待たずに除去してよい — 将来の GUI 実装は自身の状態を持てばよく、write-only の field を残す理由がない。double-`OpenGui` ゲート案も検討対象だったが、そもそも観測者がいないため gate 追加ではなく除去を採る。
4. **`ruviform-audio-core` cargo doc 警告 2 件** — `planar.rs:123,243` の `` [`Error::InvalidValue("planar_frames")`] `` が intra-doc link として解決不能（pre-existing）。他箇所の確立表記 `` [`Error::InvalidValue`] `` に揃え、payload はリンク外の inline code で表現する。

## Why Now

- frame-I/O helper は Issue #76 Task 2/3/4/5 IR で繰り返し follow-up 記録されており、コピーが増えるほど wire 契約の解釈が分岐するリスクが残る。#114 の real-adapter E2E が同じ frame を今後も喋るため、1 authoritative 実装への集約が早いほどコピー数の増殖を止められる。
- `runner_program()` の fragility は Task 5 IR の follow-up であり、#114 design が「#123 が解決形を強化すればそれに追従」と依存を明記している。
- `gui_open` は L1 で GUI 動詞が `HostError::Unsupported` defined rejection となったため、当初想定した「L1 composite で使途決定」という条件が消え、dead field が確定した。
- doc 警告は `cargo doc` を警告ゼロに保つゲート方針の観点で pre-existing として残っていたものを回収する。

## Users and Value

- このリポジトリで L1 wire protocol を触る開発者 — frame 輸送ロジックの変更点が `wire` 層 1 箇所になり、peer 実装間の解釈ずれを防ぐ。
- テスト実行者 — `runner_program()` が env override を受け付けるようになり、非 cargo レイアウト・IDE 直接実行でも L1 E2E を駆動できる。

## Scope

### In Scope

- `ruviform-plugin-core::wire` への frame 輸送 helper（`read_frame`/`read_message`/`write_message`）追加 — `std::io` Read/Write 前提・`pub` API・既存の decode/encode/limit 定数を消費する thin layer
- 上記 4 箇所の私有実装削除と helper 消費化（export の `Fill`/`ReadFail` を含む EOF 意味論は `io::Result<bool>` 形へ写像）
- `l1_proxy_e2e.rs::runner_program()` の `RUVIFORM_L1_RUNNER` env override + `current_exe` 相対 fallback（exe dir 自身 + deps/ 親の両方を検査）
- `host.rs` `Instance.gui_open` 除去（write 箇所 4 箇所と宣言・初期化を含む）
- `planar.rs` 2 箇所の intra-doc link 修正

### Out of Scope

- `Descriptor::matches` 順序感度・`View::attach` header 再検証・u16 generation wrap — Issue #123 明記の documented decisions（条件発生時のみ再評価）
- wire 形式・`WIRE_VERSION`・shm レイアウト・exit code 等の外部契約変更 — dedup は pure refactor であり wire 互換性を変えない
- #114 real-adapter E2E 側の `runner_program()`/`fixture_path()` 追随 — #114 の完了済み scope であり、別途追随が必要ならフォローアップとする
- `stderr` piping・underrun metrics surface — 継続 follow-up（本 Issue の対象外）

## Requirements

- frame format（`[u32 LE payload_len][u8 kind][payload]`）と `WIRE_MAX_PAYLOAD` 強制は `wire` 層の authoritative 実装のみが持つ。消費側で長 prefix の parse・limit 検査・encode/write/flush の順序を複製しない
- `read_frame` は clean EOF at frame boundary（`Ok(false)`）と mid-frame 切断（`Err(UnexpectedEof)`）を区別する — export runner の終了コード分岐（boundary EOF → `SUCCESS`、truncation → `FAILURE`）が依存する意味論
- `runner_program()` は `RUVIFORM_L1_RUNNER` 未設定時に現行同等の `target/<profile>/` 解決で runner を見つけ、見つからない場合は skip ではなく明示的に fail する（`assert!`/panic — 既存先例）
- `gui_open` 除去後、`open_gui`/`close_gui` の HostReply・plugin hook 呼出し・lifecycle 連携は現行どおり動作する
- `cargo doc -p ruviform-audio-core` が warning 0 で完了する

## Acceptance Criteria

- [x] `mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全て成功する（実績: 全 exit 0 — task `docs/tasks/l1-internal-cleanup.md` Verification・IR Validation に実 log 記録）
- [x] `cargo doc -p ruviform-audio-core --no-deps` が warning 0 件で完了する（実績: exit 0・warning 0 — `planar.rs` の 2 warning 解消）
- [x] `grep -rn "read_message\|write_message\|read_frame" crates/` が wire 層定義と各消費側の呼出しのみを示し、同形状の framing 実装が残らない（実績: frame 実装は `wire.rs` の 3 `pub fn` のみ — `l1_runner_e2e` の `fn send` は `wire::write_message` への薄い委譲）
- [x] `grep -rn "gui_open" crates/` が 0 件を返す（実績: 0 件）
- [x] `l1_proxy_e2e` の runner 消費テストが `RUVIFORM_L1_RUNNER` 未設定で現行どおり pass する（実績: `composite_e2e::runner_bin_serves_real_gain_plugin` が dual-layout fallback 解決で pass）

## Deferred Design Concerns

- 継続 follow-up（`Descriptor::matches` 順序感度・`View::attach` header 再検証・u16 generation wrap・stderr piping・underrun metrics）は Issue #123 スコープ外として据え置き — 条件発生時に再評価する documented decision を変更しない
- `gui_open` 相当の GUI 状態追跡が #115 の GUI 設計で必要になった場合、同 Issue が自身の型・不変条件と共に再導入する（write-only bool を継続利用する想定ではない）

## Sources and Relations

- 元 Issue: [#123](https://github.com/scarecrow173/ruviform/issues/123)（#76 各 IR の closeout Follow-ups + task-1/3/5 IRs が一次記録）
- 上流: `plugin-l1-out-of-process.md` spec/design（L1 の defined contract の出典）・`implement-issue-76-plugin-l1-out-of-process.md` plan（follow-up の発生元）
- 先例: #114 の `RUVIFORM_L1_REAL_PLUGIN_DIR` env → `current_exe` sibling fixture 解決（`docs/designs/issue-114-real-adapter-l1-e2e.md` — 「#123 が解決形を強化すればそれに追従」と依存明記）
