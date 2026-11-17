# Issue #629 os endpoint wiring lane registration — 実装実験記録（2026-11-18）

`docs/tasks/issue-629-os-endpoint-lane.md` の AC6 残部 slice
（`os_endpoint.rs` の `#[ignore]` wiring leg 用 canonical
opt-in lane 登録 + stale 記述同期 + fail-closed self-run
evidence）で試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-629-os-endpoint-lane.md`](../ir/issue-629-os-endpoint-lane.md)
（`IMPL-034Zfqq9gdwjWSsYVJXdgk`）、イベントログは
[`issue-629-os-endpoint-lane-exp.jsonl`](issue-629-os-endpoint-lane-exp.jsonl)
を参照。

## 試行 1: lane 定義の確定（feature gate なし・lint lane なし）

- **目的**: `test-os-endpoint-wiring` の `run` 文字列・
  feature 要否・lint coverage を確定する。
- **観測**: `ruviform-audio-test/Cargo.toml` の device crate
  参照は `[dev-dependencies]` の path 依存であり test
  target の compile graph にのみ載る — `--features` フラグ
  は不要（audio-device 系 lane の `real-device` feature とは
  事情が異なる）。`tests/os_endpoint.rs` は既存 target で
  あり新規 test target ではないため、既定 `lint
  ruviform-audio-test`（`cargo clippy --locked --all-targets
  -p`）が既に coverage（merged IR Validation に記録済み）。
  `test-midi-virtual-endpoint` が同じ「feature gate なし・
  専用 lint lane なし」の先例。
- **結論**: run は `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --test-threads=1`（Coordinator
  決定）。feature gate なし・専用 lint lane なしを design・
  test-spec・dev-env doc に記録。この判断は slice 6
  （`test-midi-virtual-endpoint`）先例どおり。

## 試行 2: stale 記述の同期範囲の切り分け

- **目的**: 「canonical lane 未登録 / lane 登録は後続
  slice」を示す記述を全て洗い出し、更新可否を分類する。
- **観測**: `os_endpoint.rs` module doc（「no canonical lane
  exists yet; `mise.toml` is out of this slice's scope」）、
  `docs/designs/issue-629-test-runtime-wiring.md` §5 Risks、
  `docs/designs/issue-629-provisioning-runbook.md` Deferred、
  `docs/designs/issue-629-virtual-endpoint-lane.md`、
  `docs/discovery/issue-629-virtual-endpoint-survey.md`
  Evidence 節の 2026-11-17 entry に stale 記述が残っていた。
  survey Evidence・両 lane design は append-only 規約。
  merged IR `docs/impl/ir/issue-629-test-runtime-wiring.md`
  はリース外。
- **結論**: `os_endpoint.rs` module doc と wiring design は
  リース範囲内で更新/reconcile。survey Evidence・両 lane
  design は dated 追記のみ（既存 entry 無編集）。merged IR
  は無変更で本 slice 側の IR・design・survey で消化を記録。

## 試行 3: canonical lane self-run の fail-closed 確認

- **目的**: 本ホスト（未 provisioning）で登録 lane の
  self-run を実行し、`verdict: NotRun` を正直に記録する。
- **観測**: `mise run test-os-endpoint-wiring` → exit 0、
  `#[ignore]` leg 2 件とも `ok`（2 passed・3 filtered out
  — deterministic legs は `#[ignore]` ではないため lane 対象
  外であることを `filtered out` で確認）。canonical lane の
  `run` には `--nocapture` を含めないため verdict/run_metadata
  行は libtest capture 内 — 補助証拠として `mise exec --
  cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --nocapture` を併記。audio leg が
  `observed:"NOT_PROVISIONED — no render↔capture virtual
  name-stem pair"`、MIDI leg が `observed:"NOT_PROVISIONED —
  no bidirectional name match"` の `run_metadata` JSON
  （いずれも `result:"not-run"`, `level:"ext-l3"`,
  `environment_kind:"virtual-os-endpoint"`）+ `verdict:
  NotRun` を emit。
- **結論**: fail-closed 契約を維持 — 未 provisioning で
  Pass を偽装しない。evidence は
  `evidence/rf629/os_endpoint_wiring_lane_selfrun-windows-
  local.txt` に canonical lane 実行ログ + 補助 `--nocapture`
  出力を「canonical = mise lane、直接 cargo は補助証拠」
  と明記して記録。provisioned runner での実走 Pass は外部
  gate として残る。

## 試行 4: 既存 lane 非回帰の確認

- **目的**: `mise.toml` への task 追加が既存 opt-in lane を
  壊していないことを確認する。
- **観測**: `mise run test-audio-virtual-endpoint` → exit 0
  （1 passed）、`mise run test-midi-virtual-endpoint` →
  exit 0（2 passed）。`mise run fmt-check` / `mise run
  check ruviform-audio-test` / `mise run lint
  ruviform-audio-test` / `mise run test ruviform-audio-test`
  / `mise exec -- prek run --all-files` 全て exit 0。
- **結論**: additive 変更で既存 lane・3OS 経路は不変。
  `check-toml` が `mise.toml` の TOML 検査を兼ねることを
  prek 経由で確認。
