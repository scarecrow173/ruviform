# Issue #532 child-level inventory + capture-plane 診断 — 実走結果

- 実施日: 2026-11-21
- 環境: interactive Windows x86_64 desktop、NVIDIA RTX 4070 SUPER、
  multi-monitor（DISPLAY1 primary）
- 実行: `plugin-dock-real-scenario`（`real-dock` feature）、case
  `real-dcomp-childinv`。canonical JSONL は
  `evidence/rf631/plugin-dock-real-scenario.jsonl`、本 doc の raw
  証拠は `childinv-dcomp-on.jsonl` / `childinv-dcomp-off.jsonl` /
  `childinv-pixels/`（68 PNG）

## 採取した inventory（measured facts）

| role | class_name | style (GWL_STYLE) | exstyle (GWL_EXSTYLE) | class_style (GCL_STYLE) | owner thread | parent |
| --- | --- | --- | --- | --- | --- | --- |
| editor_child | `RuviformTestEditor` | `0x54000000` = WS_CHILD\|WS_VISIBLE\|WS_CLIPSIBLINGS | `0x0` | `0x3` = CS_HREDRAW\|CS_VREDRAW | caller thread | container |
| container | `Static` | `0x56000000` = +WS_CLIPCHILDREN | `0x0` | **`0x4088`** = CS_GLOBALCLASS\|CS_PARENTDC\|CS_DBLCLKS | GPUI thread | root |
| root | `Zed::Window` | top-level | **`0x240000`** = WS_EX_NOREDIRECTIONBITMAP\|WS_EX_APPWINDOW | `0x3` | GPUI thread | — |

- `container_class_parentdc = true` — `0x4088` の `CS_PARENTDC`
  含有を実測で確定（前 slice の decode 予想と一致）。
- child は caller thread 所有（`thread_id` = caller）、
  container/root は GPUI thread 所有 — thread 所有は別々。
- child の `DWMWA_CLOAKED = 0`、`has_update = true`（update
  region 残存 — caller thread が pump しないため WM_PAINT
  不配送のまま）。

## capture-plane 分離（measured facts）

| capture path | DComp-on green | DComp-off green |
| --- | --- | --- |
| `PrintWindow(child, flags=0)` | 1.000 | 1.000 |
| `PrintWindow(child, PW_CLIENTONLY)` | 1.000 | 1.000 |
| `PrintWindow(child, PW_RENDERFULLCONTENT)` | **0.000** | 0.000 |
| `PrintWindow(child, f=3)` | 0.000 | 0.000 |
| `GetDC`+`BitBlt`（child window DC） | **0.000** | **1.000** |
| screen `BitBlt`（child rect） | 0.000 | 0.000 |

- `PW_RENDERFULLCONTENT`（DWM の実表面を要求）が 0.0 で、
  legacy `WM_PRINT` 再描画（flags 0/1）が 1.0 — green は
  synthesized re-render としてのみ存在し、child の real backing
  raster には無い。
- **window DC が DComp-on で 0.0 / env-off で 1.0** — DComp-on
  では child の real backing raster 自体が空。env-off では
  raster に green が実在するのに **screen は依然 0.0** —
  阻害は「raster 無し」だけでなく「最終合成への未到達」の
  2 層に分離できる。
- `PrintWindow(container)` は flags 0/1 で error 87（拒否）、
  flags 2/3 で raster 返却・green 0.0。

## `GPUI_DISABLE_DIRECT_COMPOSITION` env arm（verified, not assumed）

| run | env | root `WS_EX_NOREDIRECTIONBITMAP` | `inferred_dcomp_path` |
| --- | --- | --- | --- |
| on | unset | true | `dcomp_active` |
| off | `=1` | false | `dcomp_disabled_by_env` |

env=1 で root exstyle から `0x200000` が消失 — **env var は
fork に実際に届き、DComp path を off にする**（gpui
`window.rs:409` の exstyle 付与分岐と `directx_renderer.rs:147`
の DComp skip 分岐どおり）。ただし swap-chain present は両経路で
走るため、env-off でも screen green は不変（0.000）。

## child-level layered/reparent 残余実験（measured facts）

child HWND は caller thread 所有（same-thread write が可能なはず）:

| stage | op | 結果 |
| --- | --- | --- |
| child のまま | `SetWindowLongPtrW(GWL_EXSTYLE \| WS_EX_LAYERED)` | **silent refusal**（`applied: false`、`win32_error: null`） |
| 〃 | `SetLayeredWindowAttributes(LWA_ALPHA)` | **error 87**（container-level と同一） |
| 〃 | `SetWindowLongPtrW(GWL_EXSTYLE, <baseline>)` | **error 6**（ERROR_INVALID_HANDLE） |
| reparent | `SetParent(GetDesktopWindow())` | applied（parent → desktop） |
| 〃 | `SetWindowPos(HWND_TOP, rect)` | applied |
| top-level 化後 | `SetWindowLongPtrW(GWL_EXSTYLE \| WS_EX_LAYERED)` | **applied、read-back `layered: true`** |
| 〃 | `SetLayeredWindowAttributes` | applied |
| restore | parent/exstyle/rect 全 verify = restored | exstyle baseline write op は err 6 を報告するも final 値一致（`ops_ok=false`） |

`latched_as_child = false`、`latched_as_toplevel = true` —
**`GWL_EXSTYLE` 書き込み失敗は cross-thread ではなく
child-window-state 制約**。`SetParent` は同じ child で成功する
一方、exstyle write は top-level 化まで latch しない。container-level
の silent drop（band-escape）と同じ機構が child-level でも再現。

## verdict

`real-dcomp-childinv` = **fail** — `ci_apply_layered_exstyle` /
`ci_apply_layered_lwa` / `ci_apply_reparent_exstyle` の必須 op
拒否と `ci_restore` の `ops_ok=false` は probe semantics どおりの
honest 記録（実行データは全て step rows に保存）。

## 確定した判別

1. child raster は composition tree に乗らない — `PW_RENDERFULLCONTENT`
   / window DC / screen の 3 面で緑無し、`WM_PRINT` 系のみ緑。
2. env arm は機能有効 — `WS_EX_NOREDIRECTIONBITMAP` の有無が
   fork の DComp 分岐の正直な proxy。
3. layered escape は child-level でも「child のままでは不可、
   top-level で latch」— exstyle write 制約は child-state 由来で
   cross-thread 説を棄却。
4. `CS_PARENTDC`（container `0x4088`）含有は実測確定だが、
   capture-plane 阻害との因果は未証明 — 次候補は GPUI fork 側の
   swap-chain present と child HWND の組成順（DComp visual
   clipping）の観測。
