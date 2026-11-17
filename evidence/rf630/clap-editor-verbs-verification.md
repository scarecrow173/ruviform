# Issue #630 CLAP editor verb 実バイナリ coverage slice — 検証記録

`ruviform#754`（merge `0450f92f`）で production chain に着地した
CLAP editor verb（`open_gui`/`close_gui`/`set_gui_size`/
`set_gui_visibility`/`set_gui_parent`）を、実 CLAP cdylib を
`ClapLibrary` 経由で load する `tests/host_path.rs` で検証する
slice。併せて `ruviform#754` で stale 化した外部 ABI validator
証拠（artifact SHA-256・repo.head）を opt-in lane 再実走で更新
した。AC4/AC7 gate は動かさない（実 dock・画素・consumer oracle
は `ruviform#631`/`ruviform#532`/各 consumer Issue の責務）。

## 決裁: test_editor は resize-capable

fixed-size decline 維持か resize-capable 化かの discovery 決裁は
**resize-capable**（`docs/adr/issue-630-clap-editor-verbs.md`）。
`ruviform#754` が VST3 fixture を `canResize=kResultTrue` +
`onSize` 実 resize にしたのと同じ理由で、`set_gui_size` の
success 経路（can_resize gate → `set_size` → 実 HWND resize）を
実バイナリで検証するには fixture の resize 受理が必要。

## 変更内容

- `crates/ruviform-plugin-test-clap/src/editor.rs`:
  `GuiState` に `size` 追加（`Default` 手書き化）、`can_resize`
  → `true`、`set_size` が要求 size を記録して `get_size` に反映
  + attach 済み child HWND を `SetWindowPos` で実 resize、
  `create` 前の `set_size` は defined rejection、`destroy` で
  size を default に戻す（VST3 の per-`createView` fresh state
  と同じ semantics）。`adjust_size`/`get_resize_hints` は
  `None` default 維持、re-embed/transient/scale semantics 不変。
- `crates/ruviform-plugin-test-clap/tests/host_path.rs`:
  非 ignore 層 2 件 + Windows `#[ignore]` 実機層 1 件を追加。
- `crates/ruviform-plugin-test-clap/README.md`: size verbs 記述を
  resize-capable 契約に追従。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows 11 (win32/x64)、Git Bash MINGW64、interactive window station あり |
| Node | v26.12.1（mise 管理） |
| git HEAD（検証対象） | `a3804b04d163d1f52f0bee032ef435b47f8753d0`（branch `scarecrow173/rf630-clap-editor-verbs`） |
| Rust | mise 管理 toolchain（`mise run check|lint|test|build` 経由） |

## 実行した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-plugin-test-clap` | 0 | Pass |
| `mise run lint ruviform-plugin-test-clap` | 101 | **Fail — 既存 main 破損（scope 外・後述）** |
| `mise run test ruviform-plugin-test-clap` | 0 | Pass（lib 66 + host_path 17、ignored 各 1） |
| `mise run check ruviform-plugin-clap` | 0 | Pass（adapter 非回帰） |
| `mise run lint ruviform-plugin-clap` | 0 | Pass（同上 — default feature では host.rs が cfg 外） |
| `mise run test ruviform-plugin-clap` | 0 | Pass（50 unit + 3 doctest） |
| `mise run build ruviform-plugin-test-clap` | 0 | Pass（cdylib 生成） |
| `cargo test -p ruviform-plugin-test-clap --test host_path -- --ignored`（補助証拠 — canonical mise task 未定義） | 0 | **Pass — 実機実走** |
| `cargo test -p ruviform-plugin-test-clap -- --ignored`（補助証拠 — 同上） | 0 | Pass（lib `set_parent_creates_real_child_hwnd` + host_path 新規試験とも ok） |
| `CLAP_VALIDATOR=<pin-47e2424> VST3_VALIDATOR=<vst3sdk-build> mise run test-plugin-abi-validator` | 0 | **Pass — verdict Passed、canonical 7 leg 全 Passed** |

## `lint ruviform-plugin-test-clap` Fail の切り分け（scope 外・既存破損）

`cargo clippy --all-targets -p ruviform-plugin-test-clap --
-D warnings` が `crates/ruviform-plugin-clap/src/host.rs:1425-1426`
の 4 件（`cast_possible_truncation` + `cast_sign_loss` —
`size.width as u32` / `size.height as u32`）で失敗する。該当
コードは `ruviform#754`（merge `0450f92f`）由来で、`host` feature
配下の `src/host.rs` にあるため `-p ruviform-plugin-clap` 単体
lint（default feature）では検出されず、test-clap の dev-dep
（`features = ["host"]`）経由でのみ顕在化する。

**stash 検証済み**: 本 slice の変更を stash した状態（= docs commit
のみの tree）でも同一の 4 error で exit 101 — 本 slice 起因では
ない main 既存破損。`crates/ruviform-plugin-clap/` は本 slice の
編集禁止範囲のため修正せず、証拠つきで報告する（別 Issue 起票
判断は Coordinator — `ruviform#722`/`ruviform#732` 先例）。

## ignored 実機層の実測値（`editor_gui_verbs_over_real_child_hwnd`）

interactive Windows window station で実走・**Pass**。観測値:

- hidden top-level parent ×2（`CreateWindowExW` + `STATIC` class）
  を作成し `ParentLease::new(0, parent)` で `open_gui` 成功。
- `GetWindow(parent, GW_CHILD)` が非 null、`GetParent(child) ==
  parent`、`IsWindow(child) != 0` — 実 child HWND 生成を確認。
- child 生成直後の `GWL_STYLE & WS_VISIBLE != 0`（`show` が
  open_gui 内で実行された証拠）。
- `set_gui_size(GuiSize{800,600})` 成功後、`GetWindowRect(child)`
  が **幅 800・高さ 600** — `can_resize` gate → `set_size` →
  `SetWindowPos` の実 resize を OS レベルで観測。
- `set_gui_visibility(false)` 成功後 `WS_VISIBLE` bit が 0 に
  クリアされ `IsWindow(child)` は生存を維持、`true` で bit が
  復帰 — hide/show が child を破棄せず visibility のみ切替。
- `set_gui_parent`（第二 parent）は fixture の `gui already
  embedded` rejection → `Err(HostError::PluginFailed)`、child の
  parent は第一 parent のまま不変。
- `close_gui` 成功後 `IsWindow(child) == 0` かつ
  `GetWindow(parent, GW_CHILD) == null` — child 破棄を確認。
- 再 `close_gui` は `Err(HostError::Unsupported)`。
- GUI session 終了後も audio pass-through（`Continue` + bitwise
  copy）が継続 — defined rejection が instance を破壊しないことを
  確認。

非 ignore 層（全 OS）:

- `editor_gui_verbs_decline_without_open_gui`: `test_editor` の
  4 verb が GUI 未 open で各々 `Err(Unsupported)`、audio 継続。
- `non_editor_classes_decline_gui_verbs`: index 0-3 の 4 class で
  同 4 verb が `Err(Unsupported)` に収束。

## 外部 ABI validator 再実走（digest 簿記更新）

provisioned validator は run.json 記録の SHA-256 と一致確認済み
（clap-validator pin `47e2424`:
`b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`、
VST3 SDK validator:
`5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd`）。

| 項目 | 前回（`b6d0cc6a`、preset-discovery slice） | 今回（`a3804b04`） |
| --- | --- | --- |
| verdict | Passed | **Passed**（変化なし） |
| canonical legs | 7/7 Passed | 7/7 Passed |
| informational fault leg | Unsupported / exit 101 | Unsupported / exit 101（同じ） |
| `artifacts.clapDll.sha256` | `9fd29888…` | `ba514392cf01642272d4a08c0e018d82afece01a0e388008c296ef4042766bfb` |
| `artifacts.vst3Dll.sha256` | `bf35cb36…` | `b7225c5484bd3a9beecc109bfdba3ed6fcec38bc519abf9c2b5980e6e0268ef7` |
| `repo.head` | `b6d0cc6a…` | `a3804b04d163d1f52f0bee032ef435b47f8753d0` |

clap-validator の leg 別 counts の前回 run との差分は
`process-audio-denormals` timing test の揺らぎのみ（raw txt diff
で確認）: test_fault が success→warning（2.31x slowdown）、
test_editor が warning→success へ反転。実行環境のタイミング依存で
fixture 変更とは無関係、skipped 数は全 leg 不変（gui window 系は
validator が skip — 本 slice の実機層がその coverage を担う）。
vst3-sdk-validator は 235 passed / 0 failed で前回と同一 verdict。

run.json の `repo.root` は本 worktree
（`G:\orca\workspaces\ruviform\rf630-clap-editor-verbs`）を記録 —
前回は `rf630-preset-discovery` worktree だったため path 差分は
worktree 名の違いによるもの。

## 未検証・範囲外（正直な記録）

- **実 dock / message pump / 画素 / Input / Focus / Popup /
  Close / 再 Open の e2e lane**: `ruviform#631` 責務。本 slice の
  ignored 試験は hidden parent への attach + verb 観測に留め、
  AC4 gate は動かさない。
- **DComp 既知 Fail の解消判定**: `ruviform#532` 責務。
- **consumer oracle 照合（synvelis#101/#127/#128/#176・
  audio-lens#127）**: 各 consumer Issue 責務。依存追加なし。
- **`mise run lint ruviform-plugin-test-clap`**: scope 外の既存
  main 破損（`ruviform-plugin-clap` host feature の clippy 4 件）
  で Fail — 本 slice 起因でないことを stash 検証で確認済み。
- **docs index（`docs/**/README.md`）**: 生成物のため未同期 —
  Coordinator が統合時に再生成。
- MSRV lane（`*-msrv`）・`*-full` lane: 実施しない（daily CI 委譲）。
