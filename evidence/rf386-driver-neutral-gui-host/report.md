# RF386 実施 report — driver-neutral GUI Host + 最小 GPUI Window fixture

- Issue: https://github.com/scarecrow173/ruviform/issues/386
- Branch: `scarecrow173/rf386-gui-v2`(spec 記載の `feature/gui-neutral-host-gpui-driver` からの相違 — Orca 生成 branch をそのまま使用。rename はしなかった)
- Base: `49de02c` → main `ad99d18`(CI PR #392)を merge 済み
- Head: `17454a5e06c7d514fd63686c9c434ae923cc735f`(origin push 済み)
- PR: https://github.com/scarecrow173/ruviform/pull/394 (draft・Refs)

## 実施内容

| 区分 | 内容 |
| --- | --- |
| 共通契約 | `crates/ruviform-ui/src/driver.rs` 新設。`WindowSpec`/`WindowLifecycle`/`DriverEvent`/`WindowDriver`/`WindowCapabilities`/`Capability`/`LogicalSize`/`LogicalPosition`。gpui/DOM/wry/OS handle 不含有を確認 |
| エラー | `UiError::Unsupported { capability, message }` 追加(`#[non_exhaustive]` で非破壊) |
| runtime | `UiRuntime::dispatch_window_intent` 追加。`Action(Intent)` は parse 経路と同一 dispatch。malformed op は両経路 `malformed_message` で一致 |
| IPC | `Intent::new(id, op, params)` 追加(v/kind を schema 準拠で充填) |
| FakeDriver | `ruviform-ui-test::FakeDriver` + 14 semantic fixture |
| GPUI | `ruviform-ui-gpui::GpuiDriver`/`IntentSink`(gpui 0.2.2、非 default `gpui` feature)。`examples/minimal_window.rs` |
| mise lane | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` 追加 |
| docs | discovery/spec/design/plan/test-spec/task(T1-T4) 9 本 |

## GPUI 採用確認(一次情報 + 実 build)

- crate: 公式 `gpui` `0.2.2`(crates.io)。Windows target deps(windows ^0.61, embed-resource)あり
- toolchain: mise pin `rust = "1.99.0"` で feature build 成功
- `PlatformDisplay` は logical bounds + uuid のみ(scale API なし)→ monitor physical は live window scale で推定と明記、#388 の拡張点
- `Window::on_window_closed` は未提供 → `App::on_window_closed` + `sweep_destroyed` で代替
- transitive warning: `proc-macro-error2 v2.0.1` の future-incompat notice(上流、当方制御外)

## 検証(正規 mise 入口・実 command/exit)

| command | 結果 |
| --- | --- |
| `mise run fmt-check` (`cargo fmt --all -- --check`) | exit 0 |
| `mise run check` (`cargo check --workspace --all-targets`) | exit 0(gpui feature off で workspace green = 隔離確認) |
| `mise run lint` | exit 0 |
| `mise run test` (`cargo test --workspace`) | exit 0(全 suite ok。`wedged_child_write_converges_and_teardown_is_bounded` は並行 build 負荷下で 1 度 SpawnFailed Timeout、単独/負荷なし再実行で Pass — flaky、本変更と無関係) |
| `mise run check-ui-gpui` (`cargo check -p ruviform-ui-gpui --features gpui --all-targets`) | exit 0(example 含む) |
| `mise run lint-ui-gpui` (`cargo clippy ... --features gpui -- -D warnings`) | exit 0 |
| `mise run test-ui-gpui` (`cargo test -p ruviform-ui-gpui --features gpui`) | exit 0 |
| `mise run example-ui-gpui-minimal` (`cargo run -p ruviform-ui-gpui --features gpui --example minimal_window`) | NotRun — 実 OS session での手動検証入口として登録済み(実 Window 表示/入力は未観測)。headless では実行しない |

## Review protocol

- Reviewer A(契約面): PASS-WITH-NOTES → 全 finding 反映(create_window Errors 記載、malformed parity、`focus(false)` honest error、Resized/Moved 遷移差分、design doc 同期、Intent::new、DriverEvent→runtime 対応表記)
- Reviewer B(実装): PASS-WITH-NOTES → 全 finding 反映(sweep_destroyed の guard 跨ぎ修正、position:None→Bounds::centered、RequestClose 死 window drop、IntentSink dead drop、inbox bound、doc 修正、backlog 試験の順序/識別 assert 強化)
- Validator: VALIDATED(全 claim をコードで確認、gpui 0.2.2 一次情報と照合)
- Judge: MERGE-READY・blocker なし(minor 3 件を対応: gpui README/index 追加、test-spec/t3 の doc 同期、ui/ui-test README 更新)
- 記録: 独立 reviewer 2 名 → validator → judge を runtime subagent で実施

## RF388/389 への受け渡し

- 最小 Window fixture 利用(正規入口): `mise run example-ui-gpui-minimal`(実 OS session でのみ実行)
- 拡張点: `crates/ruviform-ui-gpui/src/lib.rs` の `RvfRoot`(root view 差替 seam)、`spec_options`(WindowOptions mapping)、`refresh_monitors`(per-display scale 精度化)
- capability 宣言点: `WindowCapabilities::native_surface_slot`/`ime` は Unsupported 宣言済みで Supported 化は RF388/389 実装時
- `WindowSpec::position: Option<LogicalPosition>` は layout restore(#182)用

## 残 Gate / 未達

- 実 OS Window 表示/入力の手動検証は headless CI では未実行(`minimal_window` example を実機で実行する工程)
- macOS/Linux 実機: 未検証(Windows 11 x64 のみ build/clippy)
- RF262 最終採用判断: RF388/389 の実 OS 証拠後(設計書に明記)
- L1 GUI lifecycle: Unsupported のまま(RF389 担当)
- CI required gate 追加なし(RF294 所有、mise lane のみ)
