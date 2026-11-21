---
id: "TSPEC-01KBZWINDOWIME0TESTSPEC00A1"
type: "test-spec"
status: "approved"
title: "Issue #389 real-window-ime テスト仕様 — OsImeOutcome 語彙・ImeProbe mock seam・配達ログ解析・verdict lattice の判定"
created: "2026-11-21"
updated: "2026-11-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
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
  verifies:
    - "docs/specs/issue-389-window-ime.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 real-window-ime テスト仕様 — OsImeOutcome 語彙・ImeProbe mock seam・配達ログ解析・verdict lattice の判定

## Purpose

`real-window-ime` が `Pass` を主張するのは「OS-level drive が
受理され、かつ caller-side tap が commit sequence の配達を実際に
観測した」ときだけであることを固定する — 「受理された」≠
「届いた」の混同で Pass 化しない。環境不成立・前提未達・oracle
不能をそれぞれ `NotRun`/`environment_blocked`/`NotRun` に正しく
分離し、`Fail` と `Unsupported` の境界も固定する。

## Feature

`os_ime` backend（`ImeProbe` OS seam + `drive_inner`/
`preflight_inner`/`analyze` の platform-independent 判定部）と、
`real_dock.rs` の `real_window_ime_case` verdict 写像
（`OpLanding::Blocked` arm 含む）。

## Rules

- **drive_inner**（`&dyn ImeProbe` — scripted mock で全分岐を
  Windows 実機なしに判定可能）:
  - 全 call 受理 → `Injected{action:"ime-compose-commit"}`、call
    順序は `focus → context → [set_open(true)] → set_composition →
    notify(CPS_COMPLETE) → [set_open(restore)] → release`。
  - `text` 空 → `NotRun`（call 未発行）。
  - `owner_thread == 0`（dead HWND）→ `NotRun`。
  - `owner_thread != caller`（foreign thread）→ `NotRun`（pump
    不能を正直に — call 未発行）。
  - `focused() == false` → `EnvironmentBlocked`（focus 後即時
    停止 — context 系 call 未発行）。
  - `context() == None` → `EnvironmentBlocked`。
  - `set_composition() == false` → `EnvironmentBlocked`（context
    は `release` で解放済み）。
  - `notify(CPS_COMPLETE) == false` → `EnvironmentBlocked` —
    half-driven composition を残さず `notify(CPS_CANCEL)` を先行
    （call 列に cancel の notify が現れる）。
  - 開いていなかった IME を drive のため開いた場合、復元
    `set_open_status(ctx, was_open)` が release 前に走る。
- **preflight_inner**: `ime_layout`/`ime_context`/`ime_ui` の全
  check が effective のときだけ `effective == true` — layout 非
  IME・context 無し・IME window 無しの各単独不成立を mock で
  判定。
- **analyze**（platform-independent — 非 Windows でも実行）:
  - `WM_IME_STARTCOMPOSITION` のみ → `commit_confirmed()==false`。
  - START + `GCS_RESULTSTR` flag 付き COMPOSITION + END でテキスト
    流通無し → `false`。
  - 上記 + `WM_IME_CHAR`/`WM_CHAR` → `commit_confirmed()==true`
    （`committed_chars`/`committed_text` 記録）。
  - 上記 + `result_str`（dispatch 内 GCS_RESULTSTR 読み取り）のみ
    でも `true`。
  - `GCS_RESULTSTR` bit を持たない `WM_IME_COMPOSITION` は result
    flag として数えない。
  - `observed == false`（tap/pump 不能）は「空ログ」と区別する。
- **verdict lattice**（`editor_op_body` + `OpLanding::Blocked`）:
  - `landing==Accepted` + `effect_confirmed` + `verified` → `Pass`
  - `landing==Accepted` + `!effect_confirmed` + `verified` →
    `NotRun`（accepted-but-unverifiable 既存 arm）
  - `landing==Blocked` + `verified` → `NotRun`（extras に
    `environment_blocked` marker は環境起因のみ）
  - `verified == false` → `Fail`（accepted+oracle-mismatch は
    `record_obs_fail` 経由でここに来る）
  - `!supported` → `Unsupported`
- **capability**: `caps.contains(&ScenarioCapability::Ime) ==
  os_ime::supported()`（`real_window.rs` テスト）。
- **非 Windows**: `supported()==false` → `preflight`/`drive` は
  `Unsupported`/ineffective を返し、`collect` は
  `ImeDelivery::default()`（`observed:false`）を返す — fabricate
  しない。

## Examples

- MS-IME active layout の interactive Windows:
  preflight effective → drive `Injected` → tap が
  `WM_IME_STARTCOMPOSITION`/`WM_IME_COMPOSITION(lParam &
  GCS_RESULTSTR)`/`WM_IME_ENDCOMPOSITION` + `WM_IME_CHAR`/`WM_CHAR`
  を記録 → `commit_confirmed` → `Pass`。
- 英字 layout（IME 無し）: preflight `ime_layout` 非 effective →
  `NotRun` + `observed:"environment_blocked"`。
- `ImmSetCompositionStringW` 拒否 IME: drive `EnvironmentBlocked`
  → `NotRun` + marker。
- drive 受理後に commit flag 付き配達なし: `landing==Accepted`
  だが `record_obs_fail(ime_delivery)` → `verified==false` →
  `Fail`。

## Guarantees

- `Injected` が `Pass` を含意しない — `commit_confirmed()` が唯一
  の pass-grade oracle。
- `EnvironmentBlocked`/`NotRun`/`Unsupported` は verdict 語彙と
  1:1 に対応し、どれも `Fail`/`Pass` に潰れない。
- mock seam は OS 境界でのみ差し替え — verdict 写像アルゴリズム
  自体を差し替えない。
- tap 由来の `ImeDeliveredMessage` 記録はすべて実配達の結果で、
  合成/補完しない。

## Non-goals

- VST3/CLAP ABI 経由の IME verb、session verb、wire の検証（捏造
  禁止 — 対象外）。
- fixture 側 subclass / `editor_view.rs` の変更検証（不採用）。
- 画素・スクリーンショット oracle（`VisualCapture` 別件）。
- semantic editor IME 処理（capabilities 語彙）の判定。

## Risk

- interactive lane は本ホストの IME 状態依存 — `environment_blocked`
  が出た場合の判定は lane exit code ではなく case verdict + extras
  の marker で行う（Pass 捏造を避けるため）。
- tap 中に目標 HWND が破棄される競合は `IsWindow` 再確認+ proc
  復元で吸収 — 観測不能は `observed:false` → `NotRun` で誤判定
  しない。
