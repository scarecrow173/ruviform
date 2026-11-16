---
id: "SPEC-034YqGGPYfozoUitsF1nl3"
type: "spec"
status: "approved"
title: "Issue #631 real-row 拡充（AC1 残り能力の catalog 化 + AC3 非画素 editor case + headless fail-closed 実証）"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-631-real-row.md"
      - type: "doc"
        path: "docs/designs/issue-631-real-row.md"
      - type: "doc"
        path: "docs/test-specs/issue-631-real-row.md"
      - type: "doc"
        path: "docs/plans/issue-631-real-row.md"
      - type: "doc"
        path: "docs/tasks/issue-631-real-row.md"
      - type: "doc"
        path: "docs/impl/ir/issue-631-real-row.md"
      - type: "doc"
        path: "docs/impl/exp/issue-631-real-row.jsonl"
      - type: "doc"
        path: "evidence/rf631/plugin-dock-real-headless.jsonl"
      - type: "doc"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
      - type: "doc"
        path: "evidence/rf631/verification.md"
    modified:
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "doc"
        path: "docs/development-environment.md"
      - type: "doc"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by:
    - "docs/designs/issue-631-real-row.md"
    - "docs/plans/issue-631-real-row.md"
  depends-on:
    - "docs/specs/issue-631-real-window-dock.md"
    - "docs/specs/issue-631-plugin-dock-scenario.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-631-real-row.md"
    - "docs/test-specs/issue-631-real-row.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-631-real-row.md"
  references:
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
---

# Issue #631 real-row 拡充（AC1 残り能力の catalog 化 + AC3 非画素 editor case + headless fail-closed 実証）

## What Should Be Built

`rf631.plugin-dock-real` シナリオの catalog を拡張し、Issue #631
AC1 の残り採用能力と AC3 の非画素 editor 判定を case 単位で正直に
記録できるようにする。同時に、real session を開始できない
headless host が全行 `NotRun` + `observed.environment_blocked` で
fail-closed することを決定的に実証する lane を追加する。

### 追加 catalog 行（計 7 行、5 → 12 行）

AC1（実 Window の採用能力を case ごとに判定）:

| case | required | 判定内容 |
|---|---|---|
| `real-multiwindow-focus` | `MultiWindow` | 2 window mount、`request_focus` による focus 移譲の `window_focused` 観測、focus release の正直な記録 |
| `real-os-input` | `RealOsPointerOrKeyboard` | injection-effect preflight → focus 済み window への pointer move/click・key 注入が `Injected` で観測されること（物理 cursor read-back を証拠化） |
| `real-window-dpi` | `SemanticAction` | DPI/scale-factor の観測・変更 API が `RealOsGuiTest` 公開面に無いことを `Unsupported` として正直に判定 |
| `real-window-ime` | `Ime` | `Ime` capability は adapter が一切 claim しない → preflight で常に `Unsupported`（#389 Deferred の可視化） |
| `real-window-close` | `SemanticAction` | window close（`CloseRequested`/`respond_close`/Alt+F4 chord）が公開 API に無いことを `Unsupported` として正直に判定 |

AC3（非画素 editor 判定 — 画素を Pass oracle にしない）:

| case | required | 判定内容 |
|---|---|---|
| `real-editor-resize` | `NativePluginDock` | 実 attach 後に `session.set_size` を試行。`InProcessEditorOps` の定義済み `Unsupported` 拒否・session が `Attached` に留まること・close 収束を session event/HWND で観測。拒否は `Unsupported`、受理された場合は非画素 oracle 未整備として `NotRun`（偽 Pass しない） |
| `real-editor-redock` | `NativePluginDock` | 実 attach 後に `session.reattach`（同一 lease）を試行。定義済み拒否 → session `Failed` → close 収束、event 系列 `[Attached, Detached, Failed, Closed]` を観測。受理時も非画素 oracle 未整備として `NotRun` |

### headless fail-closed 実証

`plugin-dock-real-scenario --blocked-probe` を追加する。real
session を開始せずに `blocked_reports` と同一の fail-closed 写像を
実行し、生成 report が全行 `NotRun` + `observed.environment_blocked`
+ `environment_kind != real-window` であることを bin 自身が検査して
exit code に反映する。出力は
`evidence/rf631/plugin-dock-real-headless.jsonl` へ記録する
`test-ui-plugin-dock-real-headless` mise task を新設する。

## Why Now

前 slice までで 5 case（mount / native-slot / editor-attach /
capture / dcomp-pixel）が揃ったが、AC1 の「2Window Focus/Close・
DPI/IME・実 OS 入力の採用能力を case ごとに判定」と AC3 の
「Input/Resize/Focus/ReDock の Pass 判定が可能」のうち、公開 API で
正直に判定できる行がまだ catalog に無い。また headless host の
fail-closed は doc 記述と単体 test に留まり、evidence としての
実証 lane が無かった。

## Users and Value

- Scenario Console / Test Lab の consumer: `--list --include-real`
  と `ScenarioMatrix::project` が残り能力の判定状態
  （実行可能 / Unsupported / preflight Unsupported）をそのまま
  投影できる。
- 後続 slice: `InProcessEditorOps` に resize/reattach verb が
  実装された時点で、同じ case がそのまま Pass 判定へ進化する
  （case を書き換えずに済む）。
- 運用者: headless CI/SSH セッションで lane を実行しても
  `environment_blocked` が機械的に確認でき、実走証拠の偽装が
  構造的に不可能であることが証明される。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/`:
  - `src/real_catalog.rs` — `REAL_CATALOG` に 7 行追加（metadata のみ、
    GPUI-free を維持）。
  - `src/real_dock.rs` — `real_entries()` 同期、2 window mount、
    `RealDockContext` への第 2 window 保持、新規 case 7 本、
    `attempt_real_attach` の `observe_open` へ `lease_raw` 追加、
    row 数 assertion の更新、blocked-report test の全行網羅。
  - `src/bin/plugin-dock-real-scenario.rs` — `--blocked-probe`
    モード（fail-closed 実証 + 自己検査）。
  - `src/lib.rs` / `README.md` — lane 説明の整合。
- `mise.toml` — `test-ui-plugin-dock-real-headless` task 追加と
  `test-ui-plugin-dock-real` コメントの行数・内容整合。
- `docs/development-environment.md` — lane 表の整合。
- `evidence/rf631/plugin-dock-real-headless.jsonl` — probe 実走記録。
- Doc chain `docs/{specs,designs,plans,tasks,test-specs,impl/ir,
  impl/exp}/issue-631-real-row.*`。

### Out of Scope

- editor Input / Focus case — `EditorSessionOps` に input/focus verb
  が存在せず、child HWND への照準 API も無いため「判定可能な行」
  の条件を満たさない。後続 slice へ正直に残す。
- window Close の実駆動 — `WindowDriver::request_close`/
  `respond_close` は driver 内部に存在するが `RealOsGuiTest` 公開面へ
  未転送（`crates/ruviform-ui-gpui` は本 slice の lease 外）。
  `real-window-close` は `Unsupported` 判定として記録する。
- window Move / DPI 変更 / ReDock（window 間移動）/ Crash /
  Timeout — 公開 API または安全な判定 oracle が無く Deferred。
- Undo — consumer（Synvelis/AudioLens）側の意味論であり
  driver capability ではない。Ruviform catalog には置かない。
- IME 実装（#389）、DComp 画素問題の解消（#532）、Test Lab /
  Scenario Console GUI、3OS 対応、Issue auto-close。
- `crates/ruviform-ui-gpui`・`crates/ruviform-plugin-host`・
  `crates/ruviform-ui-test` の変更（scenario contract は既存の
  まま利用）。

## Requirements

- `REAL_CATALOG` と `real_entries()` は case/scenario/required が
  pairwise 一致し、既存 test が drift を検出する。両者の行数は
  12 になる。
- 新規 AC1 行の `required` は宣言どおり: `MultiWindow` /
  `RealOsPointerOrKeyboard` / `SemanticAction` / `Ime`。
- `real-multiwindow-focus` は `gui.windows()` が 2 件以上を返し、
  `request_focus` → `settle_until(window_focused)` で focus 移譲を
  観測する。focus release (`focused=false`) は driver の返す
  `Unsupported`/`Ok` をそのまま記録し、失敗扱いしない。
- `real-os-input` は `os_input_preflight` で family ごとの effect
  を確認し、`EnvironmentBlocked`/`Unsupported`/`NotRun` outcome を
  そのまま verdict 写像する（`Injected` のみ Pass 材料）。
  pointer move は component の painted bounds から物理座標を解決し
  cursor read-back を証拠に残す。aim 対象は実 native slot を
  bind した `plugin.editor`（dock panel は canvas を paint しない
  ため bounds を持たない）。key 注入は focus 済み window の
  `ScenarioTarget` に対してのみ行う（adapter の `NotRun` 規約に従う）。
- `real-editor-resize`/`real-editor-redock` は `attempt_real_attach`
  の `observe_open` を再利用し、`rejecting_editor_ops`（本番
  `InProcessEditorOps` — redeemer はこれら verb では参照されない）
  経由で op を試行する。子 HWND 契約・session 状態・event 系列・
  close 収束を `observed.steps` に記録する。
- 非画素 case は pixel capture を一切呼ばず、Pass oracle に画素を
  使わない。
- `--blocked-probe` は session を開始せず `blocked_reports` と同一
  写像で report を生成し、全行が `NotRun` +
  `observed.environment_blocked` + `environment_kind` が
  `real-window` でないことを検査する。違反時は exit 1。
- `EnvironmentKind::RealWindow` は実 session 内でのみ emit する
  （既存契約の維持）。headless console の list/refuse 契約は不変。

## Acceptance Criteria

- [ ] `REAL_CATALOG`/`real_entries()` が 12 行で一致し、
  `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  test が新行数で green。
- [ ] `plugin-dock-console --list --include-real` が新行を
  headless build で一覧でき、real selector の `--run` 拒否が不変。
- [ ] `plugin-dock-real-scenario --features real-dock` が 12 case の
  JSONL を出力する（interactive host では新行が実判定、headless
  host では全行 `NotRun`+`environment_blocked`）。
- [ ] `--blocked-probe` が fail-closed 実証 JSONL を出力し、
  自己検査違反時に exit 1 する。
- [ ] `mise run test-ui-plugin-dock-real-headless` が evidence を
  `evidence/rf631/` に書き exit 0。
- [ ] `mise run test-ui-plugin-dock` / `test-ui-plugin-dock-console`
  他の非回帰 lane が green。

## Deferred Design Concerns

- editor Input/Focus・window Close 実駆動・Move/DPI 変更/ReDock —
  `RealOsGuiTest`/`EditorSessionOps` の公開面拡張が前提 | 後続
  slice（ui-gpui/plugin-host 側の verb 追加）| verb 未到着のまま
  Pass を主張するリスク。
- `test_editor` の `onSize`/`onFocus` 実応答を oracle にする判定 —
  fixture 側の観測面整備が前提。

## Sources and Relations

- Issue `scarecrow173/ruviform#631`（tracker — auto-close しない）。
- 前段 slice: `docs/specs/issue-631-real-window-dock.md`、
  `docs/specs/issue-631-real-editor-attach.md`、
  `docs/specs/issue-631-scenario-catalog.md`、
  `docs/specs/issue-631-scenario-console.md`。
- `POLICIES.md`（Pass/Fail/NotRun/Unsupported、fail-closed、
  OS-L1/GUI-L2 分離）。
- `EditorSessionOps` の defined refusal
  （`crates/ruviform-plugin-host/src/editor_session.rs`）。
- Issue #552 injection-effect preflight（`os_input.rs`）。
