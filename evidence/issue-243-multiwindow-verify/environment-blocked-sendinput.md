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
