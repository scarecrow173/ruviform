# ruviform#910 検証記録 — real_adapter_l1_full_series / real_test_plugin_l1_full_series の GUI refusal 期待値修復

実施日: 2026-11-23 / worktree: `rf910-stale-gui-refusal` /
branch: `scarecrow173/rf910-stale-gui-refusal`。OS: Windows /
workspace pin toolchain。base SHA `62937a49`（branch 起点・作業
開始時点の origin/main、#907 着地済み）。

Issue #910: #907（merge `f6958503`、Issue #884 slice 3 — wire v6
GUI verb family + negotiated capability gate）で `*_full_series`
4 系列が失敗するようになった件の修復。判定は「実装は refusal
lattice と整合しており、**テスト側の stale な期待値を正す**
修正が正しい」であり、意味回帰（定義側の修正）は不要と結論
付けた。根拠は下記の照合表。

## 実測で確定した失敗の実態

base `62937a49` で `mise run test-l1-real-plugin` を実行:

- step 3 `--test l1_real_adapter_e2e -- --ignored` で
  `ruviform-plugin-clap` の `real_adapter_l1_full_series` が
  `tests/l1_real_adapter_e2e.rs:417`（`CloseGui` assert）で
  `left: Err(InvalidValue) / right: Err(Unsupported)` となり
  FAIL → lane がここで中断（残 step は fail-fast で未到達）。
- `OpenGui` assert（`:406-416`）は `CloseGui` より先に実行済みで
  pass した = **Windows でも `OpenGui` の実測値は
  `Err(Unsupported)`**（Coordinator 照合どおり plugin hook
  decline 経路。`Err(InvalidValue)` 化という件名の読みは実測で
  否定）。
- `SaveState`/`RestoreState` assert は変更不要（実測でも
  `Err(Unsupported)`）。

## refusal 語彙照合表（定義側との突き合わせ — #884 残 AC「語彙照合」材料）

| 経路 | 定義側の根拠 | 判定される Err | 期待値判定 |
| --- | --- | --- | --- |
| `SaveState`/`RestoreState` | wire 往復 → child 側 state verb 未実装。設計 doc D-verbs error 語彙「`Unsupported`（未実装 plugin hook）」（`docs/designs/issue-872-l1-oos-plugin-editor-host.md` L383-385） | `Err(Unsupported)`（両 OS） | 現行期待どおり・stale なし |
| `OpenGui` `{ mode: Floating }`（GUI なし fixture） | host gate 通過時は wire 往復 → `open_floating` → `plugin.open_gui` が decline（`crates/ruviform-plugin-export/src/gui.rs` L403-459）。CLAP: `clap.ext.gui` 未実装 → `Err(Unsupported)`（`plugin-clap/src/host.rs` L1732-1734）。VST3: `createView` null → `Err(Unsupported)`（`plugin-vst3/src/host.rs` L3263-3265） | `Err(Unsupported)`（Windows） | 期待値は正しいが、経路説明 comment は stale（→ 修正） |
| `OpenGui` `{ Floating }`（非 Windows） | `AUDIO_HELLO_CAPABILITIES = 0`（`plugin-export/src/lib.rs` L149-153）→ host gate が wire 前に拒否（`plugin-host/src/l1/mod.rs` L3092-3102）。設計「未広告 child への GUI op は全て `Err(Unsupported)` で現行拒否の意味を維持」（同 doc L171） | `Err(Unsupported)` | 期待値どおり（値は両 OS で同一・provenance が異なる） |
| `CloseGui`（live surface なし・Windows） | wire 往復 → `dispatch_gui` が `gui_handle.is_none()` を `Err(InvalidValue)` で拒否（`plugin-export/src/lib.rs` L505-506、`gui.rs` L343-349）。`wire.rs` `CloseGui` doc「An instance with no live surface is `Err(HostError::InvalidValue)`」（`plugin-core/src/wire.rs` L789-792）、設計 doc D-verbs「`InvalidValue`（`CloseGui`-with-no-surface・`payload` 域外）」（同 doc L334-337・L386） | `Err(InvalidValue)` | **stale（本件の破損箇所）** → 修正 |
| `CloseGui`（非 Windows） | host gate → `Err(Unsupported)`（同上） | `Err(Unsupported)` | 期待値どおり（Windows と同じ `Err(InvalidValue)` へ無条件で書き換えると壊れる） |
| `SetGuiParent`（参考・対象外） | `l0_only_gui_verb` guard → `Err(Unsupported)`（設計 doc L364-368、本 test では未 assert） | `Err(Unsupported)` | 変更なし |

## 修復方式の採用理由

`CloseGui` 期待値の分岐は **`host.negotiated_gui_capabilities(handle)`
の実値に `WIRE_CAPABILITY_GUI_FLOATING` が立つか**で行う
（`#[cfg(target_os = "windows")]` ではなく）。理由: host gate
自身が negotiated bit を見て拒否する実装（`l1/mod.rs`
L3096-3101）であり、テストも同じ negotiated contract を見る
ほうが OS cfg より正直 — 非 Windows runner が将来 bit を立てても
テストが追従する。`#[cfg]` 版だと `AUDIO_HELLO_CAPABILITIES` の
`#[cfg]` と平行の暗黙同期になり、runner 実装が変わったとき
検出しない。

`OpenGui` の assert 値は両 OS で `Err(Unsupported)` のまま維持
（実測・定義側ともに確認済み）。変更したのは provenance の説明
comment と series doc comment（「never reach the wire」「host-side
short-circuits」は Windows では偽になるため）に加え、clippy
`too_many_lines`（100 行）を超えた `full_series` から GUI refusal
probe を `assert_gui_refusals` helper へ括り出した構造整理。

## 検証記録（全て Windows・worktree で実施、canonical = `mise run`）

| lane | 結果 |
| --- | --- |
| `mise run test-l1-real-plugin` | **PASS（exit 0）** — 8 step 全緑。`real_adapter_l1_full_series` ×2・`real_test_plugin_l1_full_series` ×2 の全 4 系列 pass を確認。修復前再現 run では step 3 が FAIL（上記「失敗の実態」） |
| `mise run check-l1-real-plugin` | PASS（exit 0） |
| `mise run lint-l1-real-plugin` | PASS（exit 0、`-D warnings`） |
| `mise run fmt-check` | PASS（exit 0） |
| `mise run check ruviform-plugin-clap` | PASS（dead_code warning ×2 は main 既存） |
| `mise run check ruviform-plugin-vst3` | PASS |
| `mise run check ruviform-plugin-test-clap` | PASS |
| `mise run check ruviform-plugin-test-vst3` | PASS |
| `mise run lint ruviform-plugin-vst3` | PASS |
| `mise run lint ruviform-plugin-test-clap` | PASS |
| `mise run lint ruviform-plugin-test-vst3` | PASS |
| `mise run lint ruviform-plugin-clap` | **FAIL — main 既存の破損（後述・範囲外）** |
| `mise run test ruviform-plugin-clap` | PASS（54 + 3 doctests green） |
| `mise run test ruviform-plugin-vst3` | PASS（44 green） |
| `mise run test ruviform-plugin-test-clap` | PASS（77 + 23 green） |
| `mise run test ruviform-plugin-test-vst3` | PASS（116 + 24 green） |

### `mise run lint ruviform-plugin-clap` の main 既存破損（範囲外・報告のみ）

default feature（`host` 無し）の lint は `src/adapt.rs` の
`translate_output_event`/`translate_note_event` が dead_code で
`-D warnings` に落ちる — `git stash` で作業ツリーを main 状態に
戻しても同一 signature で再現することを確認（rf891 の記録
`evidence/rf891/verification.md` でも同じ件が報告済み）。本
slice の変更ファイル（`tests/` 4 件）とは無関係のため修正せず。
本 crate の canonical lint は feature 付き `lint-l1-real-plugin`
であり、そちらは PASS。

## 未実施（対象外・NotRun）

- 非 Windows leg（ubuntu-latest・macos-latest）の green — ローカルは
  Windows のみ。negotiated bit 分岐は `AUDIO_HELLO_CAPABILITIES`
  の `#[cfg]` 実装と論理整合するが、3OS の最終証拠は merge 後の
  CI（`l1-real-verify`・`l1-real-verify-macos` lane）が持つ。
- `*-full`・`*-msrv`・Feature Matrix — 本 slice の対象外（POLICIES
  §5 の比例検証）。
