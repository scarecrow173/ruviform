# Issue #630 editor verb — slice 5（dock-test row flip + 実 GUI lane 実走）検証記録

`docs/designs/issue-630-editor-verbs.md` の「実装 slice 分割案」に対し、slice 5
として `test-ui-plugin-dock-real`（VST3）・`test-ui-plugin-dock-real-clap`
（CLAP）両 lane の editor op case を verb-driven oracle へ移行し、実 GUI lane
を本 Worker で実走して ledger を再生成した。`ci/plugin-abi-validator` の
`test_editor` fault-arm annotation（Coordinator lease 付与範囲）も併せて実施。

## 実行環境

| 項目 | 値 |
| --- | --- |
| OS | Windows (win32/x64)、Git Bash MINGW64 |
| worktree | `G:/orca/workspaces/ruviform/rf630-verbs-docktest` |
| ブランチ | `scarecrow173/rf630-verbs-docktest`（`origin/main` `41b8a6ec` = PR #949 merge 起点） |
| Rust | mise 管理 toolchain（`mise run check|lint|test|fmt` 経由） |
| 実施日 | 2026-11-25 |

## 変更内容

### `crates/ruviform-plugin-dock-test/src/real_dock.rs`

- `oschild::focused_hwnd()` — `GetFocus()` caller-queue oracle を新設
  （Windows + stub）。`GetFocus` は per-thread 契約で、fixture の実
  `SetFocus(child)` と同じ scenario caller thread の queue を読む。
- `EditorAbi`/`ABI_VST3`/`ABI_CLAP` — ABI-parametrized fixture oracle
  語彙（popup class・child-popup title prefix・popup leg cumulative
  suffix）。focus suffix は両 ABI で同一 grammar のため shared const
  `FOCUS_SUFFIXES`。
- `drive_editor_verb` + `VerbDrive` — 全 verb leg を `fresh
  real_editor_ops` 経由で駆動し着地を `{Ok,Refused,Declined(
  EditorOpError),Failed,Missing}` に分類。
- `observe_editor_focus_lifecycle` を 6-leg 化: `set_focus(true)` →
  `set_focus(false)` → `set_focus_target(Editor)` →
  `set_focus_target(EditorElement{7})` → `set_focus_target(HostWindow)`
  → `set_focus(false)` restore。`f:1` leg は `GetFocus()==child`
  oracle、全 leg で fixture title event log の cumulative suffix を検証。
- `observe_editor_popup_lifecycle` を `set_popup` 4-leg 化（Dialog ×
  open/close + ChildWindow × open/close）— 旧 VK_P `send_key` 駆動を撤去。
  `ChildWindow` kind は新設 `EnumChildWindows` grandchild oracle
  （`editor_child_hwnd` で attach child 同定 → `parent == child` +
  ABI title prefix + `WS_CHILD` set/`WS_POPUP` clear + 200x100）で検証。
  popup 存在中の attach/title-log 検証は `direct_children_view`（
  `parent_matches` direct child のみ）で grandchild の誤爆を遮断。
- case rows: `real-clap-editor-{input,focus,popup}` を defined-refusal
  `Unsupported` から effect-verified に flip、VST3 `real-editor-
  {focus,popup}` に target/kind coverage を追加。

### `ci/plugin-abi-validator/`（Coordinator lease 付与範囲のみ）

- `run.mjs` `CLAP_PLUGINS.test_editor` — 共有 `Fault` param を持つため
  `exclude: "param-(set|fuzz)"` + excludeReason を `test_fault` と同
  理由で annotate（CLAP mirror of VST3 #885、Issue #630）。
- `tests/run.test.mjs` — `test_editor` entry の pin と informational-run
  finder が `test_fault` を選び続けることの contract test を追加。

## `HostWindow` leg —— 実走で予測が訂正された経緯（正直記録）

実装前の解析では「container HWND は GPUI app thread 所有・verb 実行は
scenario caller thread の分割 thread 構成で、`AttachThreadInput` 結合は
設計上禁止（Issue #115）のため `SetFocus(container)` は thread queue を
越えられず、VST3 adapter は `GetFocus` verify 失敗で `PluginFailed` に
着地する」と判断した。Coordinator はこの予測に基づき
`VST3=Declined(Plugin)`・`CLAP=Ok+f:0` の宣言 landing を承認した。

**しかし実 lane では予測が外れた**: VST3 `win32::set_focus`（host.rs —
`SetFocus(hwnd); GetFocus()==hwnd` verify）が caller thread から成功し、
`Ok`+`f:0` に着地した（`real-editor-focus` 初走で leg 5 が
`expected Err(Ops(Plugin)) but the verb landed Ok` で fail —— この
failure が予測誤りを検出した）。両 thread の input queue は本 lane
topology では事実上共有/attach されており、caller queue の focus が
app-thread container へ移ることを `GetFocus` で観測できた。

訂正後の宣言: 両 ABI で `Ok`+`f:0` が宣言 landing。Coordinator 決定の
honest-vocabulary 条件どおり、同 leg は `landing:"accepted-declared"`・
`effect_verified:false` で記録する（`f:0` token の delivery は他 leg と
同様に検証するが、container への OS focus move materialize は
lane-topology 依存 —— input queue の共有状態に左右される —— ため
verified OS focus routing とは主張しない）。step には caller queue の
`GetFocus` 値を観測値として残す。**同-queue `host_path` lane が
`GetFocus` の end-to-end 真の oracle**（container と検証 thread が同一
queue を共有する lane 構成）。

注: `crates/ruviform-plugin-vst3/src/host.rs` の `win32::set_focus` doc
（"a foreign-thread handle makes `SetFocus` fail"）は foreign-thread で
失敗し得る事実としては正しいが「必ず失敗する」とは読めなくなった —
— queue 共有/attach 下では成功する。当該 crate は本 slice の lease 外の
ため doc 修正は未実施（報告事項）。

## 実走した検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run test-ui-plugin-dock-real`（VST3） | 0 | **32 case 全 pass** — `evidence/rf631/plugin-dock-real-scenario.jsonl` 再生成（`run_id=rf631-plugin-dock-real`、`repo.sha=41b8a6ec`）。`real-editor-focus`/`real-editor-popup` の 6-leg/4-leg verb-driven oracle 全 pass |
| `mise run test-ui-plugin-dock-real-clap`（CLAP） | 0 | **14 case 全 pass** — `evidence/rf630/plugin-dock-real-clap.jsonl` 再生成（`run_id=rf630-plugin-dock-real-clap`、同一 sha）。flip した `real-clap-editor-{input,focus,popup}` 全 pass + `real-clap-session-{crash,hang}`（fault-carrier）も pass |
| `mise run test-ui-plugin-dock-real-headless`（VST3 `--blocked-probe`） | 0 | **32 row 全 `not_run` + `environment_blocked`** — `evidence/rf631/plugin-dock-real-headless.jsonl` 再生成（`repo.sha=8a000038`）。flip 後 matrix で fail-closed 実証済み |
| `mise run test-ui-plugin-dock-real-clap-headless`（CLAP `--blocked-probe`） | 0 | **14 row 全 `not_run` + `environment_blocked`** — `evidence/rf630/plugin-dock-real-clap-headless.jsonl` 再生成（`repo.sha=8a000038`） |
| `mise run test-ui-plugin-dock`（headless catalog 回帰） | 0 | 9 case = 7 pass + `editor-attach` unsupported + `dock-pixels` not_run — 記録済み baseline と一致 |
| `mise run check ruviform-plugin-dock-test --features real-dock` | 0 | Pass（feature-gated canonical lane — gate 照合で未実測と判明し追走） |
| `mise run lint ruviform-plugin-dock-test --features real-dock` | 0 | Pass（初走で doc backtick×3 + `cast_possible_wrap`×2 を検出・修正 — feature 配下コードは本 lane が初の canonical lint） |
| `mise run test ruviform-plugin-dock-test --features real-dock` | 0 | 37 test 全 pass（lib 18 + console 14 + plugin_dock 5） |
| `mise exec -- cargo check -p ruviform-plugin-dock-test` | 0 | Pass（補助証拠 — canonical = mise） |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 33 test 全 pass（新規 contract test 含む） |
| `mise run fmt-check` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass |

注1: 初回 lane 実走（`HostWindow` 訂正前）は `real-editor-focus` のみ
fail —— 上述の宣言 landing 訂正で解消し再実走で全 pass。fail 記録は
訂正の実測根拠として本記録に残す。

注2: gate 照合で指摘された検証欠落（`--features real-dock` 付き
check/lint/test・`test-ui-plugin-dock`・両 `--blocked-probe` lane）は
本 commit 範囲で追走済み。`mise.toml` の comment 差分と
`evidence/rf631/plugin-dock-real-scenario.jsonl`・両 headless ledger の
再生成差分は Coordinator が lane 生成物差分ルールの事後承認を明示
（JSONL 再生成指示との矛盾のため）—— 承認済み差分として記録する。

## 判断・残件

- `HostWindow` leg は `accepted-declared`・`effect_verified:false` の
  honest vocabulary で ledger に記録 —— Coordinator 条件どおり、
  effect-verified Pass とはせず「container focus の OS move は
  lane-topology 依存・同-queue `host_path` lane が真の oracle」を
  legend/evidence に明記（本 file + `ac-inventory.md` AC4 注記）。
- `real-clap-session-{crash,hang}` は旧 ledger（`a0e34564`）では
  unsupported だったが、fault-carrier 実装済みのため現 HEAD では
  14 case 全 pass —— `ac-inventory.md` の ledger SHA・内訳を
  `41b8a6ec` / 全 pass に更新済み。
- ledger JSONL は両 lane で `repo.sha=41b8a6ec` の meta row を持ち、
  artifact digest は evidence row の `plugin-artifact:...@<digest>`
  に記録（AC5 の tuple 再現条件）。
