# Issue #243 — Windows SendInput environment block diagnostic

Date: 2026-11-10
Lane: `example-ui-gpui-real-os-multiwindow-smoke` (real OS input)
Verdict impact: real-input checks are **environment-blocked** on this host
at the time of the post-refactor re-run. Earlier run (pre-refactor binary)
completed all 17 checks green while `SendInput` was still functional.

## Symptom

`SendInput` returns success (`sent=1`/`sent=2`) but injected events have
**zero effect**: cursor never moves (absolute and relative moves),
NumLock never toggles. `WScript.Shell.SendKeys` (SendInput-based) is
equally dead. Meanwhile legacy injection APIs work: `mouse_event`
(relative move landed exactly +300 px), `keybd_event` (NumLock toggled),
`SetCursorPos` (cursor teleports). The block is system-wide — reproduced
from a Task Scheduler interactive task outside the agent process tree.

## What was ruled out

| Candidate | Result |
| --- | --- |
| Riot Vanguard `vgk` kernel driver | `sc query vgk` = STOPPED; `vgc` = STOPPED; `vgtray.exe` killed — no change |
| Agent sandbox / process tree | Reproduced via `schtasks /it` interactive task — same failure |
| Remote Desktop session | `qwinsta`: console session 1 active, not RDP |
| `BlockInput` | `mouse_event`/`keybd_event` still inject — input queue not blocked |
| Accessibility features (MouseKeys/FilterKeys/StickyKeys/ToggleKeys) | registry flags at defaults (off) |
| Mouse/Keyboard class UpperFilters/LowerFilters | empty for both `4D36E96B`/`4D36E96F` class keys incl. device subkeys |
| UIPI / integrity level | all visible windows Medium IL; foreground (Fork/Orca) Medium; caller Medium; SecHealthUI Low; only Taskmgr/perfmon elevated-and-minimized/protected |
| LG HUB / Logitech Flow | `lghub_agent`, `lghub_system_tray`, `lghub_updater` killed — no change |
| GameInput user-mode proxy | `GameInputRawInputProxy` killed — no change (`GameInputRedistService` needs admin to stop) |
| `semav6msr64.sys` | Intel System Usage Report driver — not an input filter |

## Interpretation

`SendInput`/`SendKeys` events carry the `LLMHF_INJECTED` flag; the legacy
`mouse_event`/`keybd_event` path does not produce the same filtered
signature on this host. Something installed mid-session (after the
earlier all-green run) drops injected-flagged input while unflagged
legacy injection passes. Consistent with a user-mode low-level hook or
kernel input filter from software that updated/started between the two
runs. Remaining untested suspects require admin rights to stop
services (`GameInputRedistService`, Razer services) or are invasive to
kill (Discord, Steam, CrossDeviceService, msrdc).

## Impact on evidence

- The earlier run (pre-refactor binary, `SendInput` functional) recorded
  all 17 checks green: real focus clicks, Alt+F4 close choreography,
  Win+Arrow moves, detach/reattach, regeneration, window/app job policy.
- The post-refactor binary could not be re-verified on real OS input in
  the current host state; the on-disk `verdict-multiwindow-windows.json`
  is the honest `fail` output of that blocked run (click cursor
  read-back stayed at the pre-click position; foreground oracle showed
  an unrelated window).
- The refactor was clippy-driven and behavior-preserving by
  construction; headless lanes (`fmt-check`/`check`/`lint`/`test`/
  `test-rt`/`test-ui-gpui`) all pass on the final code.
- Re-verification lane: re-run
  `mise run example-ui-gpui-real-os-multiwindow-smoke` on a host where
  the `SendInput → GetCursorPos` preflight moves the cursor (e.g. after
  reboot or removal of the filtering component).

## Reproduction (host-level preflight)

Any process on the interactive desktop:

```powershell
# SendInput relative move — returns 1, cursor does not move
# mouse_event relative move — cursor moves exactly
# keybd_event NumLock — toggles
# WScript.Shell.SendKeys {CAPSLOCK} — no effect
```

Diagnostic scripts used (not committed): `%TEMP%\sendinput-rel-test.ps1`,
`%TEMP%\legacy-apis-test.ps1`, `%TEMP%\sched-test.ps1`,
`%TEMP%\il-all-windows.ps1`, `%TEMP%\class-filters.ps1`.

## Resolution (2026-11-11 追記)

本ブロックはその後解消した。最有力仮説は **Riot Vanguard `vgk.sys` の
armed 状態**（LoL/Riot Client 起動から kernel driver の完全 unload まで
の間、`SendInput` 注入イベントのみが実効を失う）。PR #551 の追加調査で:

- 独立 Rust プローブ（P/Invoke を介さない extern 直宣言）が SendInput
  API レベルで全 pass・`GetLastError=0`・`sizeof(INPUT)=40` を確認 —
  呼び出し側の構造体/ABI 問題を否定。
- LL hook 実測で `SendInput` と `mouse_event` は共に injected flag
  （`inj=1`）付きで到達 — 本文書の「`LLMHF_INJECTED` 付きイベントのみ
  drop される」仮説は反証された。
- 回復時系列: LoL 終了 → `vgk` ドライバログ停止（17:47）→
  20:43 SendInput 全回復。

因果は**未確証**である点を明記する: Vanguard 関与は時系列相関と症状
シグネチャに基づく最有力仮説であり、直接の因果証明はない。初回切り分け
で `sc query vgk` = STOPPED を「否定済み」としたが、service 状態の
STOPPED は kernel driver の armed/unload 状態と一致しない可能性があり、
その点が初回記録と後続観測を整合させる説明となる。

解消後の実測:

- PR #551（2026-11-10・schema v1 時点）: Vanguard unload 後に
  `mise run example-ui-gpui-real-os-multiwindow-smoke` を再実走し
  verdict=pass・全 check green を canonical path に記録済み。
- Issue #552 で `os_input::preflight`（注入 → 独立 OS API read-back の
  実効検証）を 3 lane の lane 本体前に fail-closed 導入。再発時は
  verdict `environment_blocked`・exit 3 で lane 本体を走らせず明示的に
  失敗するため、「受理 ≠ 実効」を GUI fail として誤記録する経路は
  塞がれた（再発防止）。
- 2026-11-11（HEAD `4eea9115`・schema v2 時点）: 同 lane を再実走し
  preflight effective（pointer+keyboard）・verdict=pass・exit 0・
  16 checks 全 green。
  `evidence/issue-243-multiwindow-verify/verdict-multiwindow-windows.json`
  は v2 schema（`preflight` 付き）で再生成済み。
