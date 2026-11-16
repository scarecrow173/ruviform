---
id: "SPEC-034YtuGM7OKrg9UOnR4L7V"
type: "spec"
status: "approved"
title: "Issue #389 editor-session real-session-terminate（real-window session lifecycle L2 case）"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "POLICIES.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/designs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/test-specs/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/plans/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/tasks/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/impl/ir/issue-389-editor-session.md"
      - type: "doc"
        path: "docs/impl/exp/issue-389-editor-session-exp.jsonl"
      - type: "doc"
        path: "docs/discovery/issue-389-editor-session.md"
      - type: "doc"
        path: "evidence/rf389/"
    modified:
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "doc"
        path: "crates/ruviform-plugin-dock-test/README.md"
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
    - "docs/designs/issue-389-editor-session.md"
    - "docs/plans/issue-389-editor-session.md"
  depends-on:
    - "docs/specs/issue-631-real-row.md"
    - "docs/specs/issue-631-real-editor-attach.md"
    - "docs/specs/issue-631-real-window-dock.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-editor-session.md"
    - "docs/test-specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate（real-window session lifecycle L2 case）

## What Should Be Built

`rf631.plugin-dock-real` シナリオの catalog に
`real-session-terminate` case を 1 行追加する（12 → 13 行）。
`test_editor` fixture の実 `IPlugView` child HWND が attach 中の状態で
`HostCommand::Destroy` による host instance 破棄を起こし、
`PluginEditorSessions::notify_terminated(handle, HostLost)` が
`Terminated{HostLost}` を**ちょうど 1 回**だけ emit して session を
`Closed`+`concluded` に収束させ、child HWND が OS 観測で teardown
（`IsWindow(child)==0`）され、かつ host が存続することを real OS
window 上で判定する Issue #389 の real-window session lifecycle L2
case である。

headless lane（`rf631.plugin-dock`）には既に `host-terminate` /
`session-lifecycle` case があるが、そちらは fake-slot / real-ABI
（実 Window 無し）経路である。本 case は real-window lane
（`rf631.plugin-dock-real`）側の別 case であり、`real-` prefix で
case id が衝突しない。headless catalog は変更しない。

## Why Now

Issue `scarecrow173/ruviform#389` は tracker Issue で、real-window
session lifecycle の L2 検証は残件として積み残されている。
rf631 real-row 拡充（merge `078571a0`）で real-window lane は 12 行の
catalog 構造（`REAL_CATALOG` + `real_entries()` + `--blocked-probe`
fail-closed lane）を持つが、session の異常終了経路
（host loss → `Terminated` exactly-once → child HWND teardown →
host 存続）はまだ real-window 上で一度も実行されていない。
本 slice はその最小の 1 case を catalog に載せ、後続の
move/degraded/crash/recovery case が乗る土台を作る。

## Users and Value

- Scenario Console / Test Lab / lane consumer: `--list --include-real`
  と `ScenarioMatrix::project` が session 終了経路の判定状態をそのまま
  投影できる。
- 後続 slice: session move / degrade / crash / hang / recovery の
  case 追加時に、host loss 経路の exactly-once 契約と child HWND
  teardown 観測の型をそのまま再利用できる。
- 運用者: `environment_kind=real-window` を名乗る証拠だけが実走
  verdict を持ち、headless / fake の成功を実 Window Pass に転用しない
  という POLICIES.md §2.1 の規範を維持する。

## Scope

### In Scope

- `crates/ruviform-plugin-dock-test/`:
  - `src/real_catalog.rs` — `REAL_CATALOG` に
    `real-session-terminate`（`required: &[NativePluginDock]`）を
    editor case 群の末尾へ追加（metadata のみ、GPUI-free を維持）。
  - `src/real_dock.rs` — `real_entries()` を 13 entry 化、
    `observe_session_terminate_lifecycle`（`observe_open` seam の
    case 本体）、`session_terminate_body` / `real_session_terminate_case`、
    `editor_attach_body` の verdict 写像を共有 helper へ集約、
    `blocked_reports`/`blocked_probe_violations`/`mod tests` の行数
    assertion 追従、module doc の case 表・行数記述追従。
  - `tests/console.rs` — `list_include_real_appends_real_lane_rows`
    の行数（20 → 21）と期待 case 一覧の追従。
  - `src/lib.rs` / `src/bin/plugin-dock-console.rs` /
    `README.md` — "twelve"/12 行記述と verdict 表の追従。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-editor-session.*` の doc chain。
- `evidence/rf389/` — 本 slice の検証記録（lane 実走 JSONL・
  verification report）を一本化して置く。

### Out of Scope

- `real-session-move`（`session.reattach` は `InProcessEditorOps`
  `Unsupported` — verb 実装か close+open 限定実装かは別 slice 判断）、
  `real-session-degraded`、crash/hang/recovery case、実 Window での
  RT 非干渉観測、lane pump での `tick`/`request_deadline`/`tick_all`
  実駆動 — 後続 slice。
- adapter verb（`reattach`/`set_size`/`set_visibility`/Floating）の
  実装 — `crates/ruviform-plugin-host`・`plugin-vst3`・`plugin-clap`
  を触る別 slice。
- `real-dcomp-pixel` の画素 Pass 解禁（`scarecrow173/ruviform#532`
  決裁待ち — 現行の正直 verdict を維持し本 slice で変えない）、
  Native Dock 画素（`scarecrow173/ruviform#388`/`#532`）、製品受入
  （`scarecrow173/synvelis#176`）、consumer gate
  （`scarecrow173/ruviform#294`）。
- `docs/designs/issue-389-plugin-editor-session.md` の
  capability matrix / verification-status reconcile — 本 slice の
  リース外。
- Issue `scarecrow173/ruviform#389` の close（tracker — 残 L2/L3
  case・consumer gates は後続 slice）。
- `mise.toml` の lane 構造変更（rf631 が確立した lane をそのまま使う。
  新 lane 追加禁止）、`Cargo.toml`/`Cargo.lock` の依存変更、
  `docs/**/README.md` index の編集・再生成。

## Requirements

- `REAL_CATALOG` と `real_entries()` は case/scenario/required が
  pairwise 一致し、既存 test が drift を検出する。両者の行数は
  13 になる。`real-session-terminate` は catalog 末尾
  （`real-editor-redock` の次）に置き、report 順も一致させる。
- `real-session-terminate` の `required` は
  `&[ScenarioCapability::NativePluginDock]`、spec extras は
  editor case と同形（lane marker + plugin artifact identity）。
- case 本体は `attempt_real_attach` の `observe_open` seam で動き、
  attach 受理後に限り以下をこの順で観測する:
  1. `observe_attached_children` で attach 中の child HWND を記録
     （`real-editor-attach` と同一の子 HWND 契約）。
  2. `host.execute(HostCommand::Deactivate)` →
     `host.execute(HostCommand::Destroy)`（instance は Activated
     のため Destroy 直打ちは lifecycle 上不正 — headless
     `host-terminate` case と同じ順序）。adapter の instance drop
     が `close_view` → `removed()` → `DestroyWindow` を同期的に
     実行する。
  3. `sessions.notify_terminated(handle, SessionFailCause::HostLost)`
     — registry-level の終了通知。
  4. session が `Closed`+`concluded` に収束し、drain した event 列が
     ちょうど `[Attached{Docked}, Terminated{HostLost}]` であること。
  5. 2 回目の `notify_terminated` が no-op で追加 event を生まない
     こと（`by_instance` binding 除去済み — exactly-once の裏付け）。
  6. `check_detached_children` で記録済み child HWND が全て
     `IsWindow(child)==0`（teardown 済み）であること。
  7. host 存続: `host.execute(HostCommand::Scan)` が成功し、
     破棄済み handle への command が `HostError::InvalidHandle`
     で拒否されること。
- verdict 写像は既存規律どおり: refused open（定義済み
  `Unsupported`）→ `Unsupported`、その他の失敗 → `Fail`、
  setup 失敗 → `Fail`、全 step 成立 → `Pass`。実走した step のみ
  `Pass` とし、headless/fake evidence を実 Window Pass に昇格しない。
- `attempt_real_attach` 末尾の teardown（`end_session` →
  slot/lease release → `HostCommand::Destroy`）は変更しない。
  observe 内で Destroy 済みのため 2 回目の Destroy は
  `InvalidHandle` で無害（`let _ =`）。`end_session` は
  `Closed` session に対して `Ok` を返す。
- `blocked_reports`/`blocked_probe`/`blocked_probe_violations` が
  新行を含む 13 行を `NotRun`+`observed.environment_blocked`・
  `environment_kind=real-window` 不請求で生成し、
  `--blocked-probe` lane の self-check が violation 0・exit 0。
- `tests/console.rs` の `list_include_real_appends_real_lane_rows`
  が 21 行・新 case id を含む期待一覧で green。
- `EnvironmentKind::RealWindow` は実 session 内でのみ emit する
  （既存契約の維持）。GPUI/Win32 型を公開 API・evidence schema
  に漏らさない（`check-ui-driver-boundary` green — HWND は
  既存どおり整数として記録）。
- `NativePluginDock` capability は Windows-only — 非 Windows では
  defined refusal の `Unsupported`/`NotRun` を正直記録する。

## Acceptance Criteria

- [ ] `REAL_CATALOG`/`real_entries()` が 13 行で一致し、
  `real_catalog_matches_entries`/`blocked_reports_fail_closed`
  test が新行数で green。
- [ ] `plugin-dock-console --list --include-real` が新行を
  projection 表示し、real selector の `--run` 拒否（exit 2）が
  不変。
- [ ] `plugin-dock-real-scenario --features real-dock` が 13 case の
  JSONL を出力する（interactive host では新 case が実判定、
  headless host では全行 `NotRun`+`environment_blocked`）。
- [ ] `--blocked-probe` が 13 行の fail-closed 実証 JSONL を出力し、
  自己検査違反 0・exit 0。
- [ ] `mise run test-ui-plugin-dock-real-headless` が evidence を
  生成し exit 0（本 slice の証拠は `evidence/rf389/` へ記録）。
- [ ] interactive 実走可能な環境では `mise run
  test-ui-plugin-dock-real` を実走し、新 case の実 verdict と
  `environment_kind=real-window` を `evidence/rf389/` に記録する。
  実走不可なら全行 `NotRun`+`environment_blocked` をそのまま
  証拠化し、実行していない行を Pass にしない。
- [ ] 既存 real 12 case と headless lane（`test-ui-plugin-dock` 8
  case）の verdict を本 slice の変更で動かさない。
- [ ] `check`/`lint`/`test`（default feature・`real-dock`・
  `lab-gui`）・`build ruviform-plugin-test-vst3`・
  `test ruviform-plugin-host --test editor_session
  --test editor_session_driver`・`check-ui-driver-boundary`・
  `prek run --all-files` が green。
- [ ] `review-protocol` を最低 1 cycle 実施し、blocker が残って
  いないことを PR 本文に記録する。
- [ ] Issue `scarecrow173/ruviform#389` は OPEN のまま（tracker —
  残 L2/L3 case・`#532`・consumer gates は後続 slice）。

## Deferred Design Concerns

- session move / degrade / crash / hang / recovery、tick/deadline
  実駆動、実 Window RT 非干渉 — `EditorSessionOps`/lane pump の
  verb・観測面が前提 | 後続 slice | verb 未到着のまま Pass を
  主張するリスク。
- `real-dcomp-pixel` 画素 Pass — `scarecrow173/ruviform#532` 決裁
  待ち | 決裁後の専用 slice | 既知 occlusion のまま Pass 誤記の
  リスク（現行は正直 `Fail` を維持）。
- capability matrix / verification-status reconcile —
  `docs/designs/issue-389-plugin-editor-session.md` は本 slice の
  リース外 | Coordinator 判断 | matrix と実 catalog の drift が
  残るリスク（PR 本文「未対応の作業」に明記）。

## Sources and Relations

- Issue `scarecrow173/ruviform#389`（tracker — auto-close しない。
  PR 本文は `Refs #389` のみ）。
- 前段 slice: `docs/specs/issue-631-real-row.md`（catalog 12 行化・
  `--blocked-probe` 構造）、`docs/specs/issue-631-real-editor-attach.md`、
  `docs/specs/issue-631-real-window-dock.md`。
- `POLICIES.md`（Pass/Fail/NotRun/Unsupported、fail-closed、
  OS-L1/GUI-L2/EXT-L3、仮想優先受入 §2.1）。
- session API（read-only 参照）:
  `crates/ruviform-plugin-host/src/editor_session.rs`
  （`PluginEditorSessions::notify_terminated` — `by_instance`
  binding 除去により 2 回目以降は no-op、exactly-once の根拠）。
- 実装先例: `attempt_real_attach` の `observe_open` seam、
  `observe_attached_children`/`check_detached_children`、
  `check_session_converged`、headless `host-terminate` case
  （`crates/ruviform-plugin-dock-test/src/scenario.rs`）。
- host teardown 連鎖: `InProcessHost::destroy` → instance drop →
  `Vst3Plugin::drop` → `close_view` → `removed()` → fixture
  `DestroyWindow`（`crates/ruviform-plugin-vst3/src/host.rs`、
  `crates/ruviform-plugin-test-vst3/src/editor_view.rs`）。
