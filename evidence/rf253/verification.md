# Issue #253 AC1/AC6 vertical slice — 2026-11-25 verification ledger

- date: 2026-11-25
- host: windows-local (DESKTOP-NH00KUI, MINGW64_NT-10.0-26300 x86_64)
- toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), mise-managed
- branch: `feat/253-midi-e2e-stage`
- base SHA: `67afa3becd7f1eec9af98aaa0f6473adbb0cf618` (`origin/main` at implementation time)
- local provisioning state: **not provisioned** — production WinMM enumeration observed 0 input ports / 1 output port, therefore the e2e case terminal verdict is `NotRun — NOT_PROVISIONED`. This ledger does **not** claim a local virtual-endpoint Pass.

## Scope verified locally

- `crates/ruviform-plugin-test-vst3/tests/os_endpoint_midi_e2e.rs`
  - RF#628 `MidiScenario`/`run_midi_trace` supplies the send list for a commands-empty Play-at-0 playthrough.
  - Supply is checked with `midi_trace_frames_monotonic`, `midi_expected_playthrough`, and `midi_trace_eq`; the receive-side expectation remains the independent literal channel-voice sequence.
  - Terminal verdicts emit Case Schema v1 `RunMetadata` (`case_id=rf253.midi.virtual-endpoint.host-plugin-e2e`, `level=ext-l3`, `environment_kind=virtual-os-endpoint`) with explicit provenance evidence.
- `.github/workflows/virtual-audio-driver-probe.yml`
  - Adds the manual-lane `os-endpoint-midi-e2e` stage between `os-endpoint-wiring` and `fault-cases`.
  - The stage builds the fixture, runs the verifier with auxiliary `--nocapture`, validates verdict and RunMetadata identity/result fields, and collects `os-endpoint-midi-e2e-run-metadata-<os>/` as artifact evidence.
- docs chain addenda in `docs/specs/`, `docs/designs/`, and `docs/test-specs/issue-253-virtual-midi-e2e.md`, including stop/seek/overflow oracle ownership mapping.

## Local verification matrix

| command | exit | evidence | result |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | `validation-2026-11-25/fmt-check.txt` | Pass |
| `mise run check ruviform-plugin-test-vst3` | 0 | `validation-2026-11-25/check-plugin-test-vst3.txt` | Pass |
| `mise run lint ruviform-plugin-test-vst3` | 0 | `validation-2026-11-25/lint-plugin-test-vst3.txt` | Pass — clippy `-D warnings` |
| `mise run test ruviform-plugin-test-vst3` | 0 | `validation-2026-11-25/test-plugin-test-vst3.txt` | Pass |
| `mise run check-l1-real-plugin` | 0 | `validation-2026-11-25/check-l1-real-plugin.txt` | Pass — `--all-targets` feature-enabled compile |
| `mise run test-l1-real-plugin` | 0 | `validation-2026-11-25/test-l1-real-plugin.txt` | Pass — existing L1 real-plugin suites non-regressed |
| `mise run test ruviform-audio-test` | 0 | `validation-2026-11-25/test-audio-test.txt` | Pass — RF#628 seek/loop/pause oracle tests included |
| `mise run test ruviform-audio-plugin-bridge` | 0 | `validation-2026-11-25/test-audio-plugin-bridge.txt` | Pass — stop/overflow/ledger-eviction oracle tests included |
| `mise run test-ci-daily` | 0 | `validation-2026-11-25/test-ci-daily.txt` / `.exit` | Pass — 113 ci/daily contract tests |
| `mise run test-ci-tracker` | 0 | `validation-2026-11-25/test-ci-tracker.txt` / `.exit` | Pass — 108 ci/tracker contract tests |
| `mise run lint-l1-real-plugin` | 0 | `validation-2026-11-25/lint-l1-real-plugin-rerun.txt` | Pass — feature-enabled clippy `-D warnings` |
| `mise run test-os-endpoint-midi-e2e` | 0 | `validation-2026-11-25/test-os-endpoint-midi-e2e.txt` | canonical lane completed; terminal case verdict is inside libtest capture |
| `mise exec -- cargo test -p ruviform-plugin-test-vst3 --features l1-runner --test os_endpoint_midi_e2e -- --ignored --test-threads=1 --nocapture` | 0 | `validation-2026-11-25/aux-os-endpoint-midi-e2e-nocapture.txt` | auxiliary verdict harvest: `verdict: NotRun — NOT_PROVISIONED`; RunMetadata emitted and written |
| `mise run test-os-endpoint-wiring` | 0 | `validation-2026-11-25/test-os-endpoint-wiring.txt` / `.exit` | canonical wiring lane completed; passing output captures terminal verdicts |
| `mise exec -- cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --test-threads=1 --nocapture` | 0 | `validation-2026-11-25/aux-os-endpoint-wiring-nocapture.txt` / `.exit` | auxiliary verdict harvest: audio `NotRun — no input devices on this host`; MIDI `NotRun — NOT_PROVISIONED`; no local Pass claimed |
| `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | 0 | `validation-2026-11-25/actionlint-correction.txt` | Pass — includes provenance fail-closed and UTF-8/ASCII-regex correction |
| PowerShell parse of extracted `os-endpoint-midi-e2e` run block | 0 | `validation-2026-11-25/powershell-parse-correction.txt` | Pass — syntax parse only, not execution; includes UTF-8/ASCII-regex correction |
| `mise exec -- prek run --all-files` | 0 | `validation-2026-11-25/prek-all-files-final2.txt` / `validation-2026-11-25/prek-all-files-evidence-completion.txt` | Pass — all hooks green; evidence-completion rerun passed after summary line-ending normalization |
| `actions.jsonl` schema/JSONL validation | 0 | `validation-2026-11-25/actions-jsonl-validate.txt` | Pass — machine-readable action ledger parses |
| provenance env override auxiliary run | 0 | `validation-2026-11-25/aux-os-endpoint-midi-e2e-provenance-override.txt` | Pass — override reaches RunMetadata evidence; verdict remains local honest NotRun |
| provenance regex fixture validation | 0 | `validation-2026-11-25/provenance-regex-fixture.txt` | Pass — positive fixture maps to `endpoint_provenance=rvfmidi-testsigned-driver-probe` and `pass_without_provenance=False` |
| pwsh original/hardened regex comparison | 0 | `validation-2026-11-25/provenance-regex-pwsh-check.txt` | Pass — pwsh reads the fixture as UTF-8 and both regexes match; hardening removes host-encoding sensitivity |

The compact machine-generated summary is `validation-2026-11-25/summary.txt` (`overall=0`). The machine-readable action ledger is `evidence/rf253/actions.jsonl` and now uses the specified field shape `seq`/`ts`/`action`/`command`/`exit_code`/`observed` (the same shape used by the recent rf1001/rf1002 ledgers); evidence paths and the fact that some initial rows were reconstructed after coordinator review are preserved inside `observed`. The canonical verification ledger is this file, `evidence/rf253/verification.md`; `verification-2026-11-25.md` is retained only as a compatibility pointer because earlier raw logs reference that dated name. After the final provenance-condition edit, `fmt-check`, `lint-l1-real-plugin`, `actionlint`, PowerShell parse, and `prek` were re-run successfully; the final evidence/docs tree additionally re-ran `prek` successfully (`prek-all-files-final.txt`); other final-tree logs use the `*-rerun.txt` names in the table.

## Terminal RunMetadata observed locally

The auxiliary `--nocapture` run emitted:

- `result`: `not-run`
- `observed`: `NOT_PROVISIONED — no bidirectional name match (inputs=0 outputs=1)`
- `repo`/`sha`: `unknown` locally (workflow injects `RUVIFORM_RUN_REPO`/`RUVIFORM_RUN_SHA`)
- provenance evidence: `virtual endpoint provenance: manual-unverified-loopback-candidate`
- output file: `target/tmp/run-metadata-rf253-midi-virtual-endpoint-host-plugin-e2e.json`

Interpretation: the local run verifies honest fail-closed behavior only. The `environment_kind=virtual-os-endpoint` field is the case envelope; acceptance-grade Pass interpretation still requires the workflow's RvfMidi provisioning provenance (`midi-endpoint-probe` candidates and `midi-loopback` Pass), and no such Pass is claimed locally.

## order/loss/overflow/stop/seek/RT/disconnect/timestamp-clock oracle ownership mapping

The normative mapping is in `docs/specs/issue-253-virtual-midi-e2e.md` (2026-11-25 addendum). It now records run ids and `direct`/`delegated`/`residual` classification for every required concern. Summary:

| concern | classification and owner summary |
| --- | --- |
| order | delegated scheduler/pipeline order oracles plus direct ordered-subsequence observation in `os_endpoint_midi_loopback_drives_test_instrument` |
| loss | delegated fault/loss-counter oracles plus direct `InputTx`/`InputStage` counter observation |
| overflow | delegated scheduler budget + bridge ledger/chase/eviction oracles; not injected in this case |
| stop | delegated scheduler command/budget semantics and bridge `stop_releases_sounding_notes` / `stop_releases_live_notes_too` |
| seek/loop/pause | delegated RF#628 trace tests and bridge `seek_releases_and_chases_sounding_notes` |
| RT | delegated media-core/bridge no-alloc tests; this EXT-L3 wall-clock case adds no new RT-safety claim |
| disconnect/close | close invalidation is direct; restart/invalidate/vanish semantics are delegated; provisioned endpoint disappearance/recovery final acceptance remains residual to #629/#1002 evidence |
| timestamp-clock | delegated domain/late/unmapped/clock-jump oracles plus direct OS timestamp/clock-domain/stamp-base/PCM-offset observation |

## Provisioned workflow evidence

### Final branch-ref run

- run: `45609378500` — https://github.com/scarecrow173/ruviform/actions/runs/45609378500
- artifact/evidence directory: `workflow-run-45609378500-attempt1/`
- workflow head SHA: `a1b6954f785c94f584b5706d1cb0552635f75a56`
- run conclusion: `success` (GitHub-hosted `windows-2025`)
- `midi-endpoint-probe`: `ok`; `devnode=ok; probe_exit=0; probe_exit=0; midi_in_ports=1; midi_out_ports=2; loopback_candidates=2; provisioning=CANDIDATES_OBSERVED; candidate_ports=Ruviform MIDI Loopback`
- `midi-loopback`: `ok`; `test_exit=0; verdict=Pass — silent messages accepted by real WinMM output port; probe_exit=0`
- `os-endpoint-midi-e2e`: `ok`; `build_exit=0; test_exit=0; verdict=Pass — OS endpoint → attach → L1 test_instrument; case=pass; metadata=1; copied=1; provisioning_observed=True; midi_loopback_pass=True; endpoint_provenance=rvfmidi-testsigned-driver-probe; pass_without_provenance=False`
- RunMetadata identity: `case_id=rf253.midi.virtual-endpoint.host-plugin-e2e`, `repo=scarecrow173/ruviform`, `sha=a1b6954f785c94f584b5706d1cb0552635f75a56`, `level=ext-l3`, `environment_kind=virtual-os-endpoint`, `result=pass`
- case observed counters: `sent=4 observed=4 matched=4 mapped=4 delivered=4 rejected=0 late=0 dropped_backend=0 unsupported_backend=0 backend_dropped=0 backend_unsupported=0 tick_span=Some(25) note_tick_span=25 blocks_needed=18 invalidated_in=true invalidated_out=true provides_timestamps=true clock_domain=Some(ClockDomainId(5932742974072178993)) stamp_base=1/1000 provenance=Carried monotonic=true trace_entries=4 blocks_run=5 spacing_frames=384 total_frames=2400 block_frames=480`
- provenance evidence in the same metadata: `virtual endpoint provenance: rvfmidi-testsigned-driver-probe`; stage detail includes `endpoint_provenance=rvfmidi-testsigned-driver-probe` and `pass_without_provenance=False`

Interpretation: this is genuine provisioned branch-ref evidence for the corrected vertical slice because the same attempt observed the RvfMidi loopback candidate and the MIDI loopback round-trip Pass before the e2e stage, and the e2e stage evaluated the fail-closed provenance path without triggering `pass_without_provenance`. It is still not post-merge integration acceptance; Coordinator must verify the merged main SHA through the required integration path.

### Superseded or non-evidence runs

- run `45599137022` / SHA `b54b8d588f699deb4235cb0018b188cfb1ffa191` completed `success` with case-level Pass, but predates the coordinator-requested fail-closed correction; harvested under `workflow-run-45599137022-attempt1/` and retained only as implementation-time evidence.
- run `45597817992` / SHA `8691a81fb7ffe0a176c0e1ded9e6e33a8afabae6` predates the first provenance-detection fix and is likewise superseded implementation-time evidence.
- run `45606042015` was cancelled before the rf253 stages after `driver-build` remained in progress for more than 10 minutes versus roughly 3.5 minutes in prior successful runs. It has no rf253 evidence value.
- run `45607932634` was cancelled before the rf253 stages so the encoding/regex hardening commit `a1b6954f` could be tested instead of the superseded `a7993436` workflow. It has no rf253 evidence value.
