# Issue #389 Parent Lease 移行 — 実装・検証記録（2026-11-13）

- 正本の実装記録: `docs/impl/ir/issue-389-parent-lease-ir.md`
- 実験イベントログ: `docs/impl/exp/issue-389-parent-lease-exp.jsonl`
- 文書チェーン: `docs/specs/issue-389-parent-lease.md` →
  `docs/designs/issue-389-parent-lease-design.md` →
  `docs/plans/issue-389-parent-lease-plan.md` →
  `docs/tasks/issue-389-parent-lease-task.md` /
  `docs/test-specs/issue-389-parent-lease.md`

## 1. 変更概要

raw `GuiParentToken` を公開 API から完全に除去し、generation 付き
不透明 `ParentLease { generation: u32, payload: u64 }` へ移行。

- `Plugin::open_gui`/`ErasedPlugin::open_gui`、`HostCommand::OpenGui`、
  `wire::Command::OpenGui`、`LeaseRedeemer` 戻り値が `ParentLease`。
- `WIRE_VERSION` 4 → 5（`OpenGui` payload 変更の codec 規約どおり）。
- session-facing API（`ParentSlotLease`/`PluginEditorSessionId`/
  `EditorSessionOps`）は不変。OS handle は adapter 内部に閉じ込め。
- `grep -r GuiParentToken crates/` → 0 件。

## 2. 検証条件（比較条件の固定値）

| 項目 | 値 |
| --- | --- |
| 実施日 | 2026-11-13 |
| OS | Windows 11 x86_64 |
| toolchain | mise 管理（`mise.toml` pin、rustc/cargo は mise task 経由） |
| features | `ruviform-plugin-vst3`/`ruviform-plugin-clap` は `--features host` 付きで check/lint/test |
| profile | dev / test（`--locked`） |

## 3. 実行結果（canonical = mise）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass（差分なし） |
| `mise run check ruviform-plugin-core` | 0 | Pass |
| `mise run check ruviform-plugin-host` | 0 | Pass |
| `mise run check ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run check ruviform-plugin-clap --features host` | 0 | Pass |
| `mise run check ruviform-poc-native-dock` | 0 | Pass |
| `mise run check ruviform-ui-gpui` | 0 | Pass |
| `mise run check ruviform-plugin-export` | 0 | Pass |
| `mise run check ruviform-plugin-ofx` | 0 | Pass |
| `mise run check ruviform-audio-plugin-bridge` | 0 | Pass |
| `mise run test ruviform-plugin-core` | 0 | Pass（126 tests + doctests） |
| `mise run test ruviform-plugin-host` | 0 | Pass（lib 226 + editor_session 49 + l1_proxy_e2e 187） |
| `mise run test ruviform-plugin-vst3 --features host` | 0 | Pass（76、real-adapter e2e 3 件 ignored） |
| `mise run test ruviform-plugin-clap --features host` | 0 | Pass |
| `mise run test ruviform-plugin-export` | 0 | Pass（53 tests） |
| `mise run test ruviform-poc-native-dock` | 0 | Pass（bin crate、0 tests） |
| `mise run lint ruviform-plugin-core` | 0 | Pass（`-D warnings`） |
| `mise run lint ruviform-plugin-host` | 0 | Pass |
| `mise run lint ruviform-plugin-vst3 --features host` | 0 | Pass |
| `mise run lint ruviform-plugin-clap --features host` | 0 | Pass |
| `mise run lint ruviform-poc-native-dock` | 0 | Pass |
| `mise run lint ruviform-ui-gpui` | 0 | Pass |

備考: `mise run test ruviform-plugin-host` 初回は
`ruviform-plugin-l1-runner` bin 未ビルドで e2e 2 件が環境要因で
失敗。`mise run build ruviform-plugin-export` 後に再実行し全件
Pass（変更とは無関係のテスト前提バイナリ）。

## 4. 実行状態の正直な記録

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| wire roundtrip / host・session contract / adapter build-link | Pass | headless テストで実測 |
| 実 VST3/CLAP GUI attach（Windows 実 HWND） | NotRun | 外部 plugin + 実 GUI gate（#388/#532/SV#176） |
| Native Dock pixel compositing 実成立 | NotRun | #532 既知の DComp 問題。fake/headless Pass を Dock PASS と主張しない |
| 外部 plugin（audio-lens/synvelis）consumer 追従 | NotRun | cross-repo、RF #219 互換契約で別途調整 |
| L1 GUI | Unsupported | `L1EditorOps` refusal-only、wire GUI verb は将来用語彙 |
| macOS/Linux GUI attach | Deferred | OS-L1 のみ必須、GUI は利用時まで Deferred |
