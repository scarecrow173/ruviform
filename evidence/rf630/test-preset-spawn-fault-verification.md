# Issue #630 — `test_stateful` preset surface + spawn/pre-Hello fault verification

Slice: `test_stateful` Preset 面（save/load/restore 往復）+ pre-Hello/spawn-phase fault injection fixtures, VST3+CLAP, headless, through the production host bridge and `ruviform_plugin_export::run` in isolated L1 child processes.

## Scope delivered

### Preset surface

- **VST3** (`crates/ruviform-plugin-test-vst3/src/preset.rs`): `IUnitInfo`, `IUnitHandler`, `IUnitHandler2` on the controller; `IProgramListData`, `IUnitData` on the component. One `"Factory"` list (id 0), one root unit `"Main"` (`kRootUnitId`), four presets — `Default`→1.0, `Half`→0.5, `Boost`→1.5, `Mute`→0.0. Program/unit data shares the live `SharedGain` cell and the deterministic 16-byte `RFST` state codec (`getState`/`setState` identical). `IUnitInfo::setUnitProgramData` returns `kNotImplemented` (controller cannot reach the data plane). Finite out-of-range gains clamp; malformed blobs rejected.
- **CLAP** (`crates/ruviform-plugin-test-clap/src/preset.rs` + `stateful.rs`): `clap.factory.preset-discovery` factory + provider + `Location::Plugin` (`IS_FACTORY_CONTENT`); `clap.ext.preset-load` maps load key → catalog gain → existing state-blob restore; `ParamRescanFlags::VALUES` on changed gain. Unknown provider ids, unknown load keys, malformed locations, invalid inputs rejected. Catalog is `static` (pointer-identity assertions valid) and identical to the VST3 pool.
- **Dependency decision**: `clack-extensions` `preset-discovery` feature enabled (coordinator-approved). `state-context` evaluated and **not** enabled — preset loading reuses the existing state blob; no extra context channel is needed.

### Spawn / pre-Hello fault injection

- Both `*-l1-runner` bins parse `--ruviform-test-fault=MODE` from `OutOfProcessSpec::args` **before** `run` is reached, after the fixture cdylib resolves+loads (genuinely post-spawn/pre-`Hello`): `exit:N`, `abort`, `hang`, `stderr:N`, `bad-fixture`. Unrecognized argv falls through to `run`'s usage rejection (exit 2, still pre-`Hello`). `run`'s no-argv contract is untouched — the channel lives entirely in the test binaries.
- `test_spawn_fail` fixture class added to both suites: VST3 `CLASS_ROWS` class 10 (`createInstance` declines `kInvalidArgument`); CLAP `plugin_count` 6 (`create_plugin` returns `None`). Both scan cleanly (descriptor field-match succeeds) but decline at `init`.

## L1 oracles verified (`l1_fault_test_plugin_e2e.rs`, both crates — 16 tests each)

| fault | expected | observed |
| ----- | -------- | -------- |
| `bad-fixture` (missing fixture path → load exit 1) | `SpawnFailed`, stderr diagnostic retained | pass — `Transport`/`ExitObserved`/`Internal` |
| `exit:3` before `Hello` | `SpawnFailed` | pass |
| `abort` before `Hello` | `SpawnFailed` | pass |
| `stderr:4096` marker → exit 3 | `SpawnFailed` + diagnostics `Found` with marker bytes | pass |
| `hang` before `Hello` + 500 ms `command_timeout` | `SpawnFailed{Timeout}` | pass |
| unrecognized argv → `run` usage exit 2 | `SpawnFailed` | pass |
| descriptor field mismatch | `SpawnFailed{Handshake}` | pass |
| `test_spawn_fail` create decline | `Initialize` → `Err(PluginFailed)`; child stays healthy; gain plugin still drives | pass |

Pre-existing `test_fault` in-band fault tests (abort/exit/hang/error/bundled/dedicated) all still pass.

## Honest boundary notes

- Wire `CreateInstance` is **dormant** for both bridges — real ABI instantiation runs at `Initialize`. The plugin-level create rejection therefore surfaces as `Initialize → PluginFailed`, not a `SpawnFailed` at `CreateInstance`.
- `HostError::InstanceLimit` is **unreachable through `PluginHost`**: `Bundled` fills each child to `WIRE_MAX_INSTANCES` then spawns the next; `Dedicated` gives every instance its own child. A dedicated bound-probe test was written, observed minting past the bound, and removed as testing an unreachable-via-host path.
- Mid-handshake (post-`Hello`, pre-mint `Scan`/`CreateInstance`) child death is **not fixture-injectable**: the child's `Scan`/`CreateInstance` handlers run no plugin code (descriptors are cached at load; `create()` is dormant). Pre-`Hello` death/hang and the `Handshake`/`PluginFailed` rejections are the reachable spawn-phase oracles.
- Preset surfaces (`IUnitInfo`/`preset-discovery`/`preset-load`) are ABI-specific and are **not** part of the neutral `Plugin`/L1 wire surface — they are verified by the in-process ABI tests in each crate's `preset.rs` test module; the shared `RFST` state codec round-trips through L1 via the existing `l1_stateful` state save/load tests.

## Verification run

- `cargo fmt --all -- --check` — clean (auxiliary; canonical entry is `mise run fmt-check`).
- `cargo clippy -p ruviform-plugin-test-vst3 -p ruviform-plugin-test-clap --features l1-runner --all-targets -- -D warnings` — clean (auxiliary per-crate lint; canonical CI gate is `mise run lint*`).
- `cargo test -p ruviform-plugin-test-vst3` / `-p ruviform-plugin-test-clap` — unit tests pass (auxiliary; canonical lane is `mise run test*` / `test-l1-real-plugin`).
- `mise run test-l1-real-plugin` — **all green** (canonical): unit tests (68/84/66/91 + bin tests), `l1_real_adapter_e2e` 3+3, `l1_real_test_plugin_e2e` 10+10, `l1_fault_test_plugin_e2e` 16+16, `l1_stateful` 2+2.
- `mise run test-rt` — **pass** (canonical; RT/no-allocation lane, no RT-path changes in this slice).
- `mise exec -- prek run --all-files` — all hooks pass (canonical).

Unit tests per crate: VST3 91 passed/1 ignored; CLAP 66 passed/1 ignored (preset ABI tests included).

### Verification scope (honest distinctions)

- **canonical mise vs auxiliary cargo**: `mise.toml` tasks
  (`test-l1-real-plugin`, `test-rt`, `prek`) are the canonical
  verification entry points shared with CI. Direct `cargo fmt` /
  `clippy` / `test` invocations were auxiliary per-crate checks,
  not substitutes for the canonical lanes.
- **local vs integration**: all verification above ran on the local
  worktree at commit `c38f2b0c`. Post-merge integration
  verification on `origin/main` is **not run** — merge is the
  Coordinator's decision.
- **validator opt-in re-run**: **NotRun**. The preset surface
  changes fixture ABI surface, but re-running the external ABI
  validators (clap-validator / VST3 SDK validator via
  `ci/plugin-abi-validator/run.mjs`) is owned by the separate
  abi-validator slice and was not executed here.
- **notrun lanes**: `*-msrv` / `*-full` lanes and non-Windows OS
  runs are outside this slice.

One transient parallel-load flake observed (`exit_before_hello` cause race) — hardened by accepting the defined `Internal` liveness cause alongside `Transport`/`ExitObserved` for pre-`Hello` death; `Timeout`/`Handshake` remain strict.

## Files

- `crates/ruviform-plugin-test-vst3/src/preset.rs` (new), `factory.rs`, `stateful.rs`, `lib.rs`, `src/bin/…-l1-runner.rs`, `tests/l1_fault_test_plugin_e2e.rs`, `tests/host_path.rs`
- `crates/ruviform-plugin-test-clap/src/preset.rs` (new), `lib.rs`, `stateful.rs`, `Cargo.toml`, `src/bin/…-l1-runner.rs`, `tests/l1_fault_test_plugin_e2e.rs`, `tests/host_path.rs`
- No production-crate (`ruviform-plugin-vst3`/`clap`/`host`/`export`/`core`) source changes — the bridges' generic scan/create paths carry the new classes unchanged.

Coordinator approval on record: `clack-extensions` `preset-discovery` enabled; `state-context` not needed (preset load reuses the state blob).
