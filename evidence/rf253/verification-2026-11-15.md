# rf253 V9+V11+V12 verification evidence

- Date: 2026-11-15
- OS: Windows (MINGW64_NT-10.0-26300, MSYS2/Git Bash)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1, mise 2026.11.2
- environment_kind: headless (no real MIDI device, no real plugin host, no GUI)
- Base SHA: c6e4f334242567dbd0e0d8b7713d9d480a6aab66 (origin/main at branch point)
- Branch: scarecrow173/rf253-midifix

All commands run via `mise run` (canonical). Auxiliary `cargo` runs were used
during development only and are not claimed as verification.

| Command | Exit | Result |
| --- | --- | --- |
| `mise run fmt-check` | 0 | cargo fmt --all -- --check clean |
| `mise run check ruviform-plugin-core` | 0 | Finished dev profile |
| `mise run check ruviform-audio-plugin-bridge` | 0 | Finished dev profile |
| `mise run lint ruviform-plugin-core` | 0 | clippy -D warnings clean |
| `mise run lint ruviform-audio-plugin-bridge` | 0 | clippy -D warnings clean |
| `mise run test ruviform-plugin-core` | 0 | all unit + doctests pass |
| `mise run test ruviform-audio-plugin-bridge` | 0 | 4+5+6+10+16(15+1 ignored)+9 unit/integration + 2 doctests pass; 1 ignored (real MIDI device opt-in) |
| `mise run test-plugin-core` | 0 | pass |
| `mise run lint-plugin-core` | 0 | clippy clean |
| `mise run test-rt` | 0 | workspace `rt_` tests pass, no regressions |
| `mise run test-l1-real-plugin` | 0 | 3+3 l1_real_adapter tests pass |

## Notes

- `midi_wiring.rs::real_midi_device_drives_input_tx_to_stage` remains
  `ignored` — requires an OS MIDI input device; opt-in, not claimed as Pass.
- `test-l1-real-plugin` exercises the real-plugin adapter harness
  (spawn/kill/report series), not a physical MIDI device or GUI session.
- RT no-alloc non-regression covered by
  `rt_midi_delivery_collect_and_merge_no_alloc` (midi_delivery.rs) and
  `rt_midi_wiring_no_alloc` (midi_wiring.rs), both passing.
- New V9/V11/V12 tests in `midi_delivery.rs`:
  `midi_delivery_anchor_none_pending_waits_then_maps`,
  `midi_delivery_anchor_republish_domain_switch`,
  `midi_delivery_anchor_republish_remaps_pending`,
  `midi_delivery_anchor_rejection_classification` — all pass.
