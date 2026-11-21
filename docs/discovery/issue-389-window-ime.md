---
id: "DISC-01KBZWINDOWIME0DISCOVERY00A1"
type: "discovery"
status: "resolved"
title: "Issue #389 real-window-ime — ホスト IME 実測、IMM32 駆動経路、oracle channel と capability claim の確定"
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
  depends-on:
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
    - "docs/discovery/issue-389-ime-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-plugin-editor-session.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #389 real-window-ime — ホスト IME 実測、IMM32 駆動経路、oracle channel と capability claim の確定

## Summary

canonical real-dock lane の #389 残件 3 行のうち `real-window-ime`
を解くため、本ホスト（interactive Windows session）の IME 実測と
drive/oracle 経路を確定した。結果:

- 本ホストの active keyboard layout は `HKL 0x04110411`（Microsoft
  Japanese IME）で、`ImmIsIME(0x04110411) == TRUE` — IMM32→TSF
  bridge 経由で IMM32 API が実際に効く（confirmed）。
- caller 所有 HWND に対し `ImmGetContext` → `ImmSetOpenStatus(true)`
  → `ImmSetCompositionStringW(SCS_SETSTR|GCS_COMPSTR, "かな")` →
  `ImmNotifyIME(NI_COMPOSITIONSTR, CPS_COMPLETE)` がすべて真を返し、
  `WM_IME_STARTCOMPOSITION` → `WM_IME_COMPOSITION` →
  `WM_IME_ENDCOMPOSITION` + `WM_IME_CHAR` + `WM_CHAR` が対象 WndProc
  へ実配達された（confirmed）。
- `ImmGetCompositionStringW(GCS_COMPSTR)` の事後 read-back は WndProc
  外（composition callback 文脈外）では空を返す — pure read-back
  oracle（候補 a）は TSF bridge 下で**信頼できない**（confirmed）。
  採用した oracle は caller 側 `GWLP_WNDPROC` 一時 subclass + bounded
  same-thread `PeekMessage`/`DispatchMessage` pump による
  `WM_IME_*`/`WM_CHAR` 配達ログ（候補 b の lane 側変種 — fixture の
  cdylib に手を入れない）。commit 判定は
  `WM_IME_COMPOSITION` の `lParam & GCS_RESULTSTR` + commit 済み
  テキストの流通（`WM_IME_CHAR`/`WM_CHAR` または dispatch 内の
  `GCS_RESULTSTR` 読み取り）。
- capability claim は backend presence gate: `supported_capabilities()`
  が `os_ime::supported()`（= IMM32 backend 存在 = `cfg!(windows)`）
  のときだけ `ScenarioCapability::Ime` を挿す。IME 不在/無効は case
  verdict `NotRun` + `observed: "environment_blocked"` に落とす
  （`RealOsPointerOrKeyboard` と同規約 — claim 自体は落とさない）。
- session verb・`HostCommand`・wire への IME 経路捏造はしない —
  VST3 `IPlugView`/CLAP `clap.gui` に IME verb は存在せず、判定は
  OS-delivery 層に留める（`L1EditorOps` refusal-only 不変、
  `WIRE_VERSION` 不変）。`EditorCapabilities.ime`/
  `SlotCapabilities.ime`/`WindowCapabilities.ime` の語彙 claim は
  **今回見送り**（OS-delivery の確認だけでは semantic editor IME
  対応を主張できない — 「配達された」≠「editor が IME 処理した」）。
- `real-session-move`（reattach を第 2 slot lease への move として
  試す案）は**不採用** — `real-editor-redock` が同 lease 上の
  reattach を既に実観測しており、move は lease chain 上で別 slot を
  lease する別シナリオであり、IME slice の AC に観測価値を増さない
  （別 Issue 級の新規 case）。

## Exploration Goal

- 本ホストに IMM32 駆動可能な IME（MS-IME 等）が install/enabled
  かを実測する（不在なら floor deliverable = `environment_blocked`
  維持）。
- IMM32 vs TSF のどちらが programmatic composition drive として
  成立するかを確定する。
- oracle channel（(a) `ImmGetCompositionStringW` read-back、
  (b) fixture 側 subclass 観測、の採否）を確定する —
  `editor_view.rs` の `lpfnWndProc = DefWindowProcW` 制約（DLL
  unload 後の dangle）を踏まえる。
- `ScenarioCapability::Ime` の claim 規約（backend presence のみ
  vs IME 環境実効込み）を確定する。
- `real-session-move` の採否を確定する。

## Key Issues and Assumptions

- MS-IME は classic IMM32 `.ime` ではなく TSF TIP — `ImmIsIME` が
  真を返しても `ImmSetCompositionStringW` が no-op になる可能性が
  あった。実測の結果、IMM32→TSF bridge が機能し composition
  設定・commit とも受理された（confirmed）。
- `GCS_COMPSTR` read-back は WndProc 文脈外で空を返し、
  `GCS_RESULTSTR` の事後読みも予期しない文字列（`:~`）を返した
  — OS 側が報告する `WM_IME_COMPOSITION` の `lParam` フラグと、
  dispatch 内（callback 文脈）で行う `ImmGetCompositionStringW`
  読み取りだけが信頼できる（confirmed）。
- 配達観測は caller thread が行う必要がある — lane の editor
  child HWND は scenario caller thread 所有（documented D1
  thread-affinity deviation）なので、caller-side `GWLP_WNDPROC`
  subclass + same-thread pump が合法。fixture 側（cdylib 内 proc）
  の subclass は DLL unload dangle 制約に触れるため不採用
  （`editor_view.rs`:411-419 の設計制約 — subclass proc は lane
  process の `plugin-dock-real-scenario` binary 内にあり、
  window が cdylib を生き延びても dangle しない）。
- `PeekMessage`/`DispatchMessage` は caller thread の queue のみ
  pump できる — foreign-thread 所有 HWND は `NotRun`（pump 不能を
  正直に報告）。
- `SetFocus` は caller thread 作成のウィンドウに対して有効 —
  focus が取れないと `WM_IME_*` が別 window へ route されるため
  drive の前提条件として検証する（focus 拒否 = `EnvironmentBlocked`）。
- 日本語以外の IME（中韓等）でも同契約 — composition text は
  `かな`（hiragana）で、conversion-dependent な commit 結果を
  判定に使わない（delivery sequence + テキスト流通を見る）。

## Investigation Process

1. レジストリ `HKCU\Keyboard Layout\Preload` → `00000411`
   （Japanese）。`HKL 0x04110411` に対し `ImmIsIME` が TRUE —
   IMM32 到達可能 IME が active layout として存在。
2. PowerShell + C# P/Invoke probe（一時スクリプト — リポジトリ外）
   で caller 所有 window を生成し `WM_IME_*` ロギング WndProc を
   持たせて実測:
   - `ImmGetContext` → 有効 HIMC
   - `ImmGetOpenStatus` → false → `ImmSetOpenStatus(true)` → TRUE
   - `ImmSetCompositionStringW("かな")` → TRUE
   - `ImmNotifyIME(NI_COMPOSITIONSTR, CPS_COMPLETE)` → TRUE
   - pump 後の観測: `WM_IME_STARTCOMPOSITION`,
     `WM_IME_COMPOSITION`, `WM_IME_ENDCOMPOSITION`,
     `WM_IME_CHAR`, `WM_CHAR`, `WM_IME_NOTIFY` 系が配達
   - 事後 `GCS_COMPSTR` read-back → 空、事後 `GCS_RESULTSTR`
     read-back → 予期しない文字列（callback 文脈外の read-back は
     TSF bridge 下で信頼できないと確定）
3. fixture `editor_view.rs` の制約確認: class の `lpfnWndProc` は
   `DefWindowProcW`（cdylib 内関数を指せない — DLL unload で
   dangle）。`SetWindowSubclass` proc も同 dangle 問題。
   fixture 側 oracle（候補 b-素）は不採用、lane-side subclass
   （候補 b-変種）を採用 — proc は `plugin-dock-real-scenario`
   binary 内コードで unload しない。
4. `os_input.rs` の `OsInputOutcome`/preflight/Probe seam 規約を
   踏襲して `os_ime.rs` を同型新設（`drive_inner`/`preflight_inner`
   が `&dyn ImeProbe` を取り、unit test は scripted mock）。
5. capability 語彙の claim 可否を評価: `ScenarioCapability::Ime`
   は lane の OS-delivery 判定の入口として claim。
   `EditorCapabilities.ime` 等の semantic 語彙は OS-delivery が
   editor semantic を証明しないため claim しない。

## Findings

- **IMM32 drive は本ホストで成立** — floor deliverable（IME 不在
  での blocked 維持）ではなく実 drive path が取れる。
- **oracle は配達ログ（subclass+pump）に確定** — read-back は
  supplementary evidence（dispatch 内読み取りのみ記録）に留める。
- **verdict 写像**（spec へ転記）:
  - accepted drive + commit sequence 観測 → `Pass`
  - preflight 不成立（IME 不在/無効/context 無し）→ `NotRun` +
    `observed: "environment_blocked"`
  - drive が environment 理由で拒否（focus 拒否/context 無し/
    composition 拒否/commit 拒否）→ `NotRun` +
    `environment_blocked`
  - run precondition 未達（dead HWND/foreign thread/empty text）
    → `NotRun`（marker 無し）
  - accepted drive だが配達ログに commit sequence が無い → `Fail`
    （accepted-without-effect は契約不一致、NotRun に降格しない）
  - oracle 機構自体が動かない（tap 不能/pump 不能）→ `NotRun`
    （accepted-but-unverifiable）
  - backend 不在（非 Windows）→ capability 未 claim → capability
    preflight `Unsupported`
- **非回帰**: `real-session-crash`/`real-session-hang` は in-process
  lane の構造限界のまま（本 slice 対象外）。他 28 case の verdict
  写像は不変。

## Alternatives and Comparison

- **oracle (a) `ImmGetCompositionStringW` read-back** — 事後の
  `GCS_COMPSTR`/`GCS_RESULTSTR` 読み取りは WndProc 文脈外では
  TSF bridge 下で空/不正値を返す（実測）。棄却。
- **oracle (b-素) fixture 側 subclass** — `editor_view.rs` の
  unload-safety 制約（class proc は `DefWindowProcW`、cdylib 内
  proc を指す subclass は DLL unload で dangle）に触れる。採用は
  host 側 proc に限るが、fixture の cdylib 境界を越える観測を
  lane に置く方が単純。棄却。
- **oracle (b-変種) lane-side `GWLP_WNDPROC` tap + bounded pump**
  — proc は lane binary 内コードで unload しない。caller thread
  所有の child HWND に合法。採用（confirmed）。
- **drive: TSF (`ITfThreadMgr`/`ITfContextComposition`) 直叩き**
  — TSF TIP の正規経路だが COM session+document manager+context
  push の一式が要り、本 slice の bounded 性（IMM32 数行で成立が
  実測済み）に対して過剰。IMM32→TSF bridge が動く以上棄却。
- **capability claim: IME 環境実効込み gate**（`preflight.effective`
  を claim 条件にする案）— claim が session 状態で揺れて capability
  preflight の意味が安定しない。`RealOsPointerOrKeyboard` の
  presence-only 規約に揃え、環境実効は case verdict 側で診断する
  設計に確定。

## Tentative Conclusions and Hypotheses

- `ScenarioCapability::Ime` claim + IMM32 drive + subclass 観測で
  `real-window-ime` は本ホストで `Pass` 可能と予測（実走で確証）。
- `WM_IME_COMPOSITION` の `lParam & GCS_RESULTSTR` は bridge 経由
  でも正しく立つと予測（probe で flag 付き配達を確認済み — lane
  の analyze が同フラグで判定する）。
- commit 済みテキストは conversion-dependent（MS-IME が `かな` を
  そのまま commit するか変換するかは環境依存）のため、verdict
  入力は delivery sequence + テキスト流通の有無に限定する。

## Open Questions

- 日本語 MS-IME 以外の layout（英字 IME 無し環境）では preflight
  `ime_layout` が `effective == false` → `environment_blocked` に
  正しく落ちるか — headless/CI lane での観測は後続検証。
- `SetFocus` が foreground-lock で拒否される稀なケース（別 window
  が foreground を握るタイミング）— drive 側が `focused()` false で
  `EnvironmentBlocked` に落ちる設計で吸収済み。

## Promotion Candidates

- `docs/specs/issue-389-window-ime.md`（本 spec で確定した
  verdict 写像・capability 規約・oracle 契約を正式化）。
- `docs/test-specs/issue-389-window-ime.md`（mock seam の unit
  test + lane verdict の判定仕様）。

## Sources and Relations

- 先行 chain: `docs/discovery/issue-389-ime-verbs.md`（editor-verb
  slice）、`docs/discovery/issue-388-389-native-dock-current-state.md`
  （real-window 現状）、`docs/designs/issue-389-plugin-editor-session.md`
  （改訂 5 追記先）。
- 実装: `crates/ruviform-ui-gpui/src/gpui_driver/os_ime.rs`、
  `crates/ruviform-ui-gpui/src/gpui_driver/real_window.rs`、
  `crates/ruviform-plugin-dock-test/src/real_dock.rs`、
  `crates/ruviform-plugin-dock-test/src/real_catalog.rs`。

## Evidence

- 実測 probe（リポジトリ外一時スクリプト — `WM_IME_*` ログ付き
  caller 所有 window + IMM32 drive）: `ImmIsIME=TRUE`、
  `ImmSetCompositionStringW("かな")=TRUE`、
  `ImmNotifyIME(CPS_COMPLETE)=TRUE`、`WM_IME_*`/`WM_CHAR` 実配達、
  事後 `GCS_COMPSTR` 空・`GCS_RESULTSTR` 不正値を確認。
- lane 実走: `evidence/rf631/plugin-dock-real-scenario.jsonl` の
  `real-window-ime` 行（append — verdict + steps/observed が正本）。
