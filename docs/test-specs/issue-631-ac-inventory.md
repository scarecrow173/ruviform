---
id: "TSPEC-rf631-ac-inventory-01"
type: "test-spec"
status: "approved"
title: "Issue #631 AC disposition census — AC×証拠 突合 test spec（fail-closed・verdict 正直性を含む）"
created: "2026-11-24"
updated: "2026-11-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "docs/discovery/issue-631-ac-inventory.md"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-631-ac-inventory.md"
  depends-on:
    - "docs/discovery/issue-631-ac-inventory.md"
    - "docs/specs/issue-631-scenario-catalog.md"
    - "docs/test-specs/issue-631-plugin-dock-scenario.md"
    - "docs/test-specs/issue-631-scenario-catalog.md"
    - "docs/test-specs/issue-389-window-ime.md"
    - "docs/test-specs/issue-313-real-os-input-driver.md"
    - "docs/test-specs/issue-630-clap-test-editor.md"
    - "docs/test-specs/issue-388-virtual-first-ac.md"
    - "POLICIES.md"
  verifies:
    - "docs/specs/issue-631-ac-inventory.md"
metadata:
  issue:
    number: 631
    url: "https://github.com/scarecrow173/ruviform/issues/631"
---

## 概要

Issue #631 の AC 7 行と canonical 証拠を 1 行ずつ突合し、verdict を固定する
test spec である。ledger 行の実測 verdict（`plugin-dock-real-scenario.jsonl`・
`plugin-dock-scenario.jsonl`・blocked-probe ledger）が正本であり、本 spec の
表はそれを誇張しない。fail-closed・verdict 正直性の確認手順を併記する。

## AC × 証拠 突合表

| # | AC 行（要約） | AC 種別 | 実測 verdict | 根拠証拠 |
| --- | --- | --- | --- | --- |
| 1 | 実 Window 起動/入力/Receipt/画素/実 State・2Window Focus/Close/Undo・DPI/IME を Case ごとに判定 | Required | **Pass**（Undo = 記録済み NotApplicable 相当） | `plugin-dock-real-scenario.jsonl` @`a0e34564` — `real-window-mount`/`real-os-input`/`real-command-receipt`/`real-capture`/`real-multiwindow-focus`/`real-window-close`/`real-window-dpi`/`real-window-ime` 全 pass。Undo は `issue-631-real-row` spec で consumer 语义として catalog 非収載 |
| 2 | Runner から Lab/Synvelis/AudioLens GUI へ driver-neutral 投影・製品 Oracle は各 repo | Required | **部分 Pass**（ruviform 側 done） | `plugin-dock-lab.jsonl`・`issue-631-test-lab` doc 群。consumer 投影 legs = external-owner（sv#114/#176・al#127） |
| 3 | 実 Child HWND→NativeSurfaceSlot・DComp 既知 Fail 再現・画素/Input/Resize/Focus/ReDock Pass 判定・未解決を Runner 整備だけで解消済みにしない | Required | **Pass** | `plugin-dock-real-scenario.jsonl` — `real-native-slot`/`real-editor-attach`/`real-dcomp-{present,uncovered,bandescape,childinv,rasterfix,foreground,pixel}`/`real-editor-{input,focus,resize,redock}` 全 pass。class C は `expected_outcome` note で確定機序を記録（`applied`/`win32_error`/`after` 実測保持） |
| 4 | Screenshot/Pixel diff/semantic tree/Command+Receipt/OS&GPU/Seed/Hash/SHA 保存・verdict 異軸 | Required | **Pass** | 全 case row が `environment`（os/arch/dcomp/`display_adapters`）・`case_seed`・`window_identities`・`plugin_artifact` sha256・`repo.sha` を記録。`verdict`/`required`/`capabilities` は別 field |
| 5 | Timeout/Session なし/Driver 不可は NotRun fail-closed・非提供 Capability は NotApplicable・macOS/Linux GUI Deferred | Required | **Pass** | `plugin-dock-real-headless.jsonl`（`--blocked-probe`）32 row 全 `not_run`+`environment_blocked`、bin 自己検査（違反時 exit 1）・`plugin-dock-real-clap-headless.jsonl` 14 row 同様。`real-session-timeout` は定義済 `Failed{Timeout}` event pass |
| 6 | Runner/Test Lab が共通 UI/API 製・GPUI 型を Consumer へ漏らさない | Required | **Pass** | `check-ui-driver-boundary` が ci.yml `ui-gpui-verify` step として収載（cargo-metadata+`syn` 機械検査・`boundary.json`）。`plugin-dock-lab` は `ruviform-ui` ViewSpec で構築 |
| 7 | 3OS OS-L1 GUI seam 既存 CI 統合・Windows real lane を Daily Full CI の PR 必須にしない | Required | **Pass**（CI 構造） | ci.yml `ui-verify`（ubuntu+windows）/`ui-verify-macos`/`ui-gpui-verify`/`ui-gpui-verify-macos` に headless seam 収載。`test-ui-plugin-dock-real*` は manual/opt-in。ci.yml は `workflow_dispatch` 専用で PR trigger なし |

## verdict 正直性チェック

- real-window ledger の全 `pass` は `observed`/`evidence` の実測値（window
  identities sha256・pixel counts・win32_error 等）を伴う。`expected_outcome`
  note 化した class C であっても refusal の実測値は残り、未解決を解消済みと
  記録していない。
- headless ledger の `editor-attach` `unsupported`・`dock-pixels` `not_run` は
  test_gain に editor surface がない fixture 法則と VisualCapture driver 不在の
  構造的事実を記録した verdict であり、Pass へ読み替えない。
- `--blocked-probe` ledger は `environment_kind=real-window` を主張せず全件
  `not_run` — headless 環境で real verdict を偽造しない fail-closed を実証。
- 集計は「AC 行 × verdict」で `Required` に紐づく Fail/未記録はない。
  `Deferred`/`NotApplicable`/`Unsupported`/`NotRun` はいずれも原因軸が
  分離記録されている。

## scope

- 本 spec は verdict の突合と正直性確認のみを扱う。case catalog の意味変更・
  新規 case・Issue 本文更新は含まない。
- CLAP leg（`plugin-dock-real-clap.jsonl`）は Issue #630 AC4 の証拠として
  rf630 側 spec に記録する — 本 spec は VST3 leg を正本とする。
- stale ledger の再生成（`plugin-dock-scenario`/`plugin-dock-console`/
  `plugin-dock-console-list*`）は本 slice で `06a5a9e0` に更新済み —
  real-window ledger は read-only で据え置き。

## lane record

| 確認 | 方法 | 結果 |
| --- | --- | --- |
| headless ledger 再生成（canonical） | `mise run test-ui-plugin-dock`（VST3 fixture cdylib build 込み） | `plugin-dock-scenario.jsonl` → `06a5a9e0`・9 case = 7 pass + 1 unsupported + 1 not_run |
| console ledger 再生成（canonical） | `mise run test-ui-plugin-dock-console` | `plugin-dock-console.jsonl` → `06a5a9e0`・exit 0 |
| catalog list 再生成（補助） | `mise exec -- cargo run --locked -p ruviform-plugin-dock-test --bin plugin-dock-console -- --list --out …` | `plugin-dock-console-list.jsonl`（9 row） |
| catalog include-real 再生成（補助） | 同上 `--features real-dock -- --list --include-real --out …` | `plugin-dock-console-list-include-real.jsonl`（41 row・crash/hang/rt-boundary の `required` に `out-of-process-plugin-host` 追加を現行化） |
| real ledger 実測確認 | python3 で JSONL parse（read-only） | `plugin-dock-real-scenario.jsonl` 32 pass・`plugin-dock-real-clap.jsonl` 9 pass + 5 unsupported・blocked-probe 32+14 全 `not_run` |
| 文書 lint | `mise run fmt-check` + `mise exec -- prek run --all-files` | （PR 本文の Verification 節に結果を記録） |
