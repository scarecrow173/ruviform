---
id: "DISC-rf768staleOracle01"
type: "discovery"
status: "resolved"
title: "Issue #768 l1_fault / l1_stateful E2E staleness oracle 監査（#763 同 class）— findings"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/768"
    - "crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-clap/tests/l1_fault_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-vst3/tests/l1_stateful_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-test-clap/tests/l1_stateful_test_plugin_e2e.rs"
    - "crates/ruviform-plugin-host/src/l1/data.rs"
    - "crates/ruviform-plugin-host/src/composite.rs"
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "docs/test-specs/issue-630-test-instrument.md"
    - "evidence/rf768/reproduction.md"
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
  related:
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "docs/test-specs/issue-630-test-instrument.md"
    - "evidence/rf768/reproduction.md"
    - "evidence/rf768/verification.md"
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

# Issue #768 l1_fault / l1_stateful E2E staleness oracle 監査 — findings

## Summary

`ruviform#763`（instrument E2E flaky）と同 class の staleness 経路が
`l1_fault` / `l1_stateful` E2E（両 ABI・4 file）にも残っていないかを
全 assertion site 監査した。**production defect は見つからなかった** —
hold-last/drop semantics は `data.rs` module docs の decision table
どおりの設計意図で、全 finding は test oracle 側の分類欠如（`ruviform#766`
が instrument 側で解消したのと同じ 2 機構）:

1. **substituted output**: L1 leg は output 不達時に `last_output`
   （最後に matched した child output）を再生して
   `Ok(ProcessResult::CONTINUE)`・`tail_frames == 0` を返す。
   分類なしで計数すると stale replay が fresh 証拠として採用され得る。
2. **dropped submit**: `dropped_submits` delta > 0 の call は input
   block — queue 済み event を含む — が child 未到達。永続 state を
   変える単発 event（mutation/arm/disarm）が落ちると、後続の
   matched block でも意図しない child state が描かれる。

## Exploration Goal

4 つの E2E file の全 assertion site を `ruviform#766` と同じ基準
（LegDeltas per-call counter 分類・matched のみ content/status
assert・永続 event は `dropped_submits` delta == 0 まで bounded
resend・assertion/最低到達数 gate 不変）で分類し、該当 site を同じ
pattern で修正する。

## Findings

機構的前提（production 読み取り・`ruviform#767` scope の原因調査は
範囲外）:

- `data.rs` submit drop 経路は `dropped_submits` を increment したうえで
  `pass_through(outputs)` を実行し `Ok(CONTINUE)` を返す — **drop は
  passthrough を兼ねる**ため `passthrough == 0` が matched の必要十分条件。
- `Failed` mirror 経路（`data.rs:663` 付近）は counter を動かさず
  `Ok(CONTINUE)`+silence を返すが、全 content gate は live leg 上に
  あるため matched 分類の健全性に影響しない（dead leg 上の post-fail
  `host_block` は counter 非計数のまま result のみ assert — 維持）。
- `audio_counters(handle)` は L1 route で `Some` を返す per-handle
  snapshot API（hang test が既に使用 — 使用先例として妥当）。

### stateful（両 ABI — mutation event 喪失 + unclassified gate）

- `l1_stateful_test_plugin_e2e.rs` の mutation `ParamValue` は単発
  `submit_block` で送出されていた（vst3/clap 各 round-trip test の
  2 箇所）。mutation block が drop されると event ごと gain 変更が
  喪失し、後続 `wait_for_gain_output` は意図した gain を**構造上
  永遠に観測できない** — `ruviform#767` が実測した周期的 drop
  （無負荷 ~7/44 call）下では flaky fail 確定経路。
- `wait_for_gain_output` 自体も matched/substituted 無分類の content
  gate であり、substituted replay（hold-last `last_output`）が期待
  gain equality を満たせば fresh 証拠なしに gate を通り得た。
- 対象外と確定した site: `rejects_malformed_state`（両 ABI）は
  process block を持たず audio gate 非依存 — 修正不要。
- fixture law 確認: `src/stateful.rs`（両 crate）の gain cell/mirror は
  `ParamValue` の plain overwrite — 同一 gain の再送は冪等で
  bounded resend が law-abiding。

### fault（両 ABI — unclassified substituted 計数 + blind resend）

- `arm_fault` は blind 4-shot resend で dropped delta の着地確認なし —
  4 連続 drop（低確度だが非零）で未 arm のまま
  `wait_host_failed` deadline flaky 経路。disarm（`none` event）も
  同じ喪失経路。
- `drive_clean_blocks` / `dedicated_isolation` の delivered 計数は
  stale replay も非 silent として計数し得た（provenance gap —
  閾値上実害は小さいが `ruviform#766` 基準では分類が必要）。
- `error` disarm の `consecutive_continue`（6 連続 Continue）は
  substituted `Ok(CONTINUE)` を disarm 証拠として誤計数可能 —
  assertion は通っても子の disarm を未観測のまま抜ける
  soundness gap。
- `bad_time` disarm の clean 計数（`Continue + tail == 0`）も同型 —
  substituted call は定義上 tail 0 を返すため無条件で真。
- `rt_alloc` の marker 計数・`invalid` の NaN 計数は armed 期の
  stale replay が条件を満たし sustained ≥2 の実観測数を水増し可能。
- 維持確認した良例: `hang` test は `passthrough_submits` counter を
  既に使用（abort/exit/bundled 系も lifecycle/notification oracle で
  本 class 外 — spawn 時点の argv fault injection も同様）。
- fixture law 確認: `src/fault.rs`（両 crate）は block 内 last
  `Fault` event wins・armed 値は block 跨ぎで持続 — arm/disarm
  event の再送は冪等。

### pre-fix loop 観測（補助証拠 — `evidence/rf768/reproduction.md`）

- dual-binary concurrent loop（vst3+clap stateful 30 iter・fault 各
  5 iter・別 worktree build load 併走）で **staleness-oracle 由来の
  失敗は 0 件** — mutation drop は本環境では観測されず flake は
  未再現（finding は構造論証に基づく）。
- 副次観測: fault loop 6 fail は全て `hang_converges_on_timeout` の
  `instantiate` `SpawnFailed { cause: Timeout }` — spawn/hello
  deadline 経路の環境 flake（`evidence/rf763` 記録済みと同 class・
  本 slice 対象外 residual）。

## Decision

`ruviform#766` と同 pattern を各 test file 内に持ち込む（file 間
helper 共有経路なし — instrument 側も file 内複写）:

- `host_block` が per-call `audio_counters` 前後差分の `LegDeltas`
  （`passthrough`・`dropped`）を result と併せて返す。
- content/status assert は `passthrough == 0`（matched）call のみ
  計数。substituted call は skip — `error` disarm の連続性判定では
  計数もリセットもしない。no assertion・最低到達数 gate の緩和。
- `arm_fault`（arm/disarm 両方）と stateful mutation submit は
  `dropped == 0` の着地確認まで bounded resend（各 file 内
  `ARM_RESEND_CALLS` / `MUTATION_RESEND_CALLS` = 8 — instrument 側
  `NOTE_OFF_RESEND_CALLS` と同 bound）。未着地は正直に fail。
- `wait_for_gain_output` は matched のみで判定・失敗時は
  substituted 数を `eprintln` で切り分け情報として出力。

fixture `src/fault.rs`・`src/stateful.rs`（両 crate）は law 上の不足
なし — read-only 監査どおり zero-diff。production host crate も
zero-diff（test-file-only slice）。

## Residual

- `dropped_submits` の周期的発生そのものは production 側調査
  `ruviform#767` の scope — counter semantics が同 Issue で変わる
  場合は `LegDeltas` 分類の前提再確認が必要（PR 本文で相互リンク）。
- `hang` test の spawn-deadline flake（`SpawnFailed{Timeout}` —
  `evidence/rf763` 記録済み環境 flake）は本 class 外・未対応。
- lease 外の `l1_real_adapter_e2e.rs` に同 class site があれば残件化。
- docs index（`docs/**/README.md`）は Coordinator が統合時に整備。
