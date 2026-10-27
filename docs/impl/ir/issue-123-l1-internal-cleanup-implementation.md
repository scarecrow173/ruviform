---
id: "IMPL-034N3QSWxbeesySpYyceVn"
type: "impl"
status: "completed"
title: "Issue #123 L1 internal cleanup implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-76-followup-l1-internal-cleanup.md"
      - type: "file"
        path: "docs/plans/issue-76-followup-l1-internal-cleanup.md"
      - type: "file"
        path: "docs/tasks/l1-internal-cleanup.md"
      - type: "file"
        path: "docs/impl/ir/issue-123-l1-internal-cleanup-implementation.md"
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
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-internal-cleanup.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #123 L1 internal cleanup implementation

## Summary

Issue #123（`docs/tasks/l1-internal-cleanup.md` — Issue #76 実装記録の
follow-up 全 4 item）を実施。frame I/O の 3 本立て複製を
`ruviform-plugin-core::wire` の共有 helper（`read_frame`・
`read_message`・`write_message`）へ統合し、host proxy・test
worker・export runner・export E2E harness の 4 消費サイトを移行。
cross-package `runner_program()` を `RUVIFORM_L1_RUNNER` 環境変数
override + 2 レイアウト fallback へ堅牢化。`gui_open` write-only
フィールドを read 不在確認のうえ `Instance` から除去。
`planar.rs` の intra-doc link warning ×2 を修正。wire 契約・外部
観測挙動は不変。全 gate green 実 log。

## Context

Issue #76 の実装記録（Task 3 IR 等）が明示的に後送りした follow-up
items のクローズ。upstream: approved spec
`docs/specs/issue-76-followup-l1-internal-cleanup.md`・approved plan
`docs/plans/issue-76-followup-l1-internal-cleanup.md`・task
`docs/tasks/l1-internal-cleanup.md`。参照した Issue #76 系 doc:
`docs/designs/plugin-l1-out-of-process.md`・
`issue-76-l1-core-wire-task-1-implementation.md`（Task 3 IR の
「frame helper は各 crate に単純コピー — wire 層への格上げは
follow-up」decision が本 item の起点）・
`issue-76-l1-proxy-control-plane-task-3-implementation.md`・
`issue-76-plugin-host-composite-task-5-implementation.md`。runner
解決の先行例として Issue #114 real-adapter tests（same-package
`CARGO_BIN_EXE_*` — cross-package には使えないことを確認済み）。

## Implementation

### Changes

- `crates/ruviform-plugin-core/src/wire.rs` — 共有 framed I/O
  helper を wire 層に追加（wire 層は `WIRE_FRAME_HEADER_LEN`・
  `WIRE_MAX_PAYLOAD`・`encode`/`decode`/`Message` の正本であり、
  frame format の不変条件を所有する境界）:
  - `read_frame(reader, buf) -> io::Result<bool>` — `Ok(false)` =
    frame 境界での clean EOF・`Ok(true)` = 1 frame 読取（kind byte +
    payload を `buf` に格納）。4B length ヘッダ途中・payload 途中
    truncation は `UnexpectedEof` error・`WIRE_MAX_PAYLOAD` 超過
    declaration は確保前に `InvalidData` 拒否。
  - `read_message(reader, buf) -> io::Result<Message>` —
    `read_frame` + `decode`・clean EOF と decode error を
    `UnexpectedEof`/`InvalidData` の `io::Error` へ写像。
  - `write_message(writer, message, buf) -> io::Result<()>` —
    `encode` + `write_all` + `flush`。
  - 各 helper に doctest（EOF/truncation/roundtrip）を同梱し
    `cargo test` で検証。
- `crates/ruviform-plugin-host/src/l1/mod.rs` — ローカル
  `read_message`/`write_message`（`io::Result<Message>`）を削除し
  `wire::read_message`/`wire::write_message` へ移行。reader
  thread の EOF 経路・driver の `Reply` wait・`Shutdown` write の
  挙動は不変（同じ `encode`/`decode`・同じ error 写像）。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — 同上。
  `main()` に `#[allow(clippy::too_many_lines)]` を追加（helper 化
  後も fixture の argv 解釈・scripted dispatch で 106/100 行 — 局所
  許可・コメントで理由明記）。
- `crates/ruviform-plugin-export/src/lib.rs` — 私有 `read_frame`・
  `send`・`Fill`/`ReadFail`/`fill` を削除。serve loop は
  `wire::read_frame`（`Ok(false)` = clean EOF → 正常終了）+
  `wire::decode`、送信は `wire::write_message`。EOF 区別（frame
  境界 clean EOF = 正常・mid-frame truncation = 非ゼロ終了）と
  oversized-declaration の確保前拒否を維持 — runner exit-code 契約
  不変。
- `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs` — 私有
  `read_message` を削除し reader thread は `wire::read_message`・
  `send` ラッパは `wire::write_message` 呼出しへ移行（`send` は
  session stdin への薄い委譲として残置）。
- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` —
  `runner_program()` を堅牢化: `RUVIFORM_L1_RUNNER` が設定されて
  いればそのまま返す override を追加し、fallback は
  `current_exe()` の自身のディレクトリ（`target/<profile>/deps` に
  隣接した `target/<profile>/runner` 配置）を先に、その親
  （`deps/` の親 = `target/<profile>` 直下）を次に探索。不在時は
  override 名・探索した両 path を含む actionable panic。欠損時の
  失敗挙動（明示 panic）は維持。
- `crates/ruviform-plugin-host/src/host.rs` — `Instance::gui_open`
  フィールド・初期化・`open_gui`/`close_gui` 内の全代入（4 箇所）
  を除去。plugin hook 呼出し・reply・lifecycle 処理は不変。
- `crates/ruviform-audio-core/src/planar.rs` — unresolved
  intra-doc link `[`Error::InvalidValue("planar_frames")]` を
  `[`Error::InvalidValue`] ("planar_frames")` へ修正 ×2（variant
  への link + 引数を link 外の code literal として分離）。
- docs — spec/plan/task/IR 新設 + 各 README 索引行追加。

### Decisions During Implementation

- **frame helper の所有境界は `plugin-core::wire`** — Task 3 IR が
  「wire 層への格上げは follow-up」と記録した経緯どおり。helper は
  `Read`/`Write` ジェネリック（`Read + Seek` 等を要求しない）で
  pipe/file 双方を受け、新クレートは作らない（task 指定どおり）。
- **`read_frame` は `bool` で EOF を返す** — export runner は
  frame 境界 clean EOF と mid-frame truncation で終了 code が
  分岐する契約を持つため、decode 済み `Message` ではなく frame
  層の bool が必要。`read_message` は EOF を含む全失敗を
  `io::Error` 化する上位 helper（proxy/worker/test 向け）。
- **`runner_program()` の dual-layout fallback** — cross-package
  では `CARGO_BIN_EXE_ruviform-plugin-l1-runner` が emit されない
  ため（Issue #114 は same-package 利用で回避）。deps 内 exe と
  profile 直下 exe の両配置を吸収し、CI/手動実行差は
  `RUVIFORM_L1_RUNNER` が最終手段。
- **`gui_open` 除去（gating ではない）** — `rg gui_open` で
  `host.rs` 内の宣言・初期化・4 代入のみ出現し read ゼロを確認。
  L1 GUI commands は `l1/mod.rs`/composite で defined rejection
  （`Err(Unsupported)`・非転送）のまま維持 — この boolean は
  将来の GUI 制御に使われておらず write-only の dead state。
  Issue #115（GUI 設計）で実状態が必要になれば、同 issue が
  自身の状態/invariant を設計として持ち込むべきものであり、
  この bool を復活させる経路は spec の non-goal として明記済み。
- **`l1-test-worker` の `too_many_lines` は局所 allow** — fixture
  の scripted dispatch 分割は本 change の因果範囲外（無関係
  refactor 禁止）のため、workspace lint 規約どおり最小 scope の
  `#[allow]` + 理由コメントで対処。

## Related Experiments

### Adopted

なし — mechanical な dedup + 明確な小 fix のみ（experiment log 不要）。

### Rejected

なし。

## Validation

- gate（実実行・exit 0）:
  - `mise run fmt-check` — pass。
  - `mise run check` — pass（`cargo check --workspace --all-targets`）。
  - `mise run lint` — pass（`cargo clippy --workspace --all-targets
    -- -D warnings` — 初期 run で `l1-test-worker` `main()`
    too_many_lines 106/100 を検出 → 局所 allow 追加で clean）。
  - `mise run test` — pass（`cargo test --workspace` — 初回 run で
    `l1_proxy_e2e` の `hang_converges_on_command_timeout` と
    `wedged_child_write_converges_and_teardown_is_bounded` が
    既知の load-sensitive 経路で flake（full-suite 負荷下の
    `Err(Ipc)`）→ 両 test を孤立 rerun で各 pass を確認後、
    `mise run test` 再実行で全 workspace green。新規 doctest
    `wire::read_frame`/`read_message`/`write_message` ×3 含む）。
  - `mise run test-rt` — pass（`rt_` レーン無退行 — 本 change は
    非 RT の stdio/制御面のみ）。
  - `mise exec -- cargo doc -p ruviform-audio-core --no-deps` —
    pass・warning 0（`planar.rs` の 2 warning 解消）。
- 構造確認（実コマンド）:
  - `rg "gui_open" crates/` — 0 件（除去完全）。
  - `rg "fn read_message|fn write_message|fn read_frame|fn send|
    fn fill" crates/` — frame impl は `wire.rs` の 3 `pub fn` のみ
    （`l1_runner_e2e` の `fn send` は `wire::write_message` への
    薄い委譲、`ruviform-dsp` の `fill` は無関係な既存 API）。
  - `composite_e2e::runner_bin_serves_real_gain_plugin` pass —
    cross-package `runner_program()` fallback + 共有 framing が
    real runner binary 経路で実動作することを直接検証。

## Risks

- `l1_proxy_e2e` の 2 load-sensitive tests は full-suite 負荷下で
  `Err(Ipc)` flake を起こしうる（本 change 起因ではなく既知 —
  孤立実行・suite 再実行ともに pass）。今後 CI で頻発するなら
  command_timeout の環境別調整を別 issue で検討。
- `runner_program()` は `RUVIFORM_L1_RUNNER` を無検証で信頼する
  （override の性質上意図的 — 誤設定は spawn 失敗 panic で即検出）。

## Follow-ups

- Issue #115（GUI 設計）— L1 GUI 実装時は `gui_open` とは別の、
  設計済みの状態/invariant を導入すること。wire の
  `OpenGui`/`CloseGui` defined rejection は本 change で不変。
- Issue #76 の残り deferred items（`Descriptor::matches` ordering・
  `View::attach` header 再検証・u16 generation wrap）は本 scope
  外のまま継続 deferred。
