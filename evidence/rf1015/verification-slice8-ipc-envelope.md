# rf#1015 slice 8 — `W-IPC-ENVELOPE` size/field gate decision core verification evidence

## 1. 結果サマリ

| 項目 | 結果 |
| --- | --- |
| slice verdict | `Pass`（local verification。merge 後 main SHA の統合検証は CI が担う） |
| requirement state | `Required` |
| assurance kind | pure decision core に対する bounded exhaustive Kani proof + shell-core接続の deterministic regression/proptest |
| canonical proof command | `mise run test-formal-kani w-ipc-envelope` |
| canonical verdict | `pass`、exit 0、10/10 harness `SUCCESSFUL`、Unwinding Assertions 有効 |
| mutation verdict | `detected`、exit 1。semantic assertion failure。revert後に proof/proptest緑 |
| consumer regression | `NotApplicable`（`ruviform_ui::ipc` crate参照はあるが、変更した exact symbolへの直接利用なし・public behavior unchanged。Issue 全体の consumer ACは未完了） |

## 2. SHA の役割

| 役割 | SHA / 参照 | 意味 |
| --- | --- | --- |
| base SHA | `446afc6abf9d88f344954db21afa737df34771ea` | 着手時・最終検証時の `origin/main`。slice 8 branchはこれを ancestorに持つ |
| 検証対象 SHA / verified Sync Commit | `bf5c61a1c3d1d305851e56a1558424fe90f94b24` | decision core抽出、in-src proof、旧 shell harness退役、deterministic regression、selective laneを含む最後の製品/proof/mise変更 commit |
| evidence sync commit | この branchの後続 commitで確定 | ledger/spec/skill/evidence/runs の同期。製品/proof/mise差分を持たない |
| 最終 head SHA | PR本文で追跡 | evidenceは自分自身の hashを固定点として書けない。verified Sync Commit以降は docs/skill/evidence-onlyであることを確認する |

`runs.jsonl` の proof/regression/mutation/consumer impact recordは `bf5c61a1` を `target_sha` に使う。prek recordは evidence-sync working treeに対する pre-commit runとして `bf5c61a1` に紐づける。

## 3. 実装境界

`crates/ruviform-ui/src/ipc.rs` に crate-privateな decision coreを抽出した。

- `intent_body_size_decision(body_len)` は raw body byte lengthだけから `body_len <= MAX_INTENT_BYTES` の accept/rejectを返す。acceptは JSON parseへ進める判定であり、最終的な intent acceptanceではない。
- `intent_envelope_gate_decision(v, kind, op)` は serde_json parse済みの envelope fieldだけから `Accept` / `UnsupportedVersion` / `UnsupportedKind` / `InvalidOp` を現行優先順位 version→kind→opで返す。
- kind helperは length precheckと fixed length 6 content equalityへ分解した。op gateは既存 `valid_op_bytes` を使う。
- coreは allocation・I/O・serde_json・JSON parse・`String`/`Value`構築・Vec growth・UTF-8 validation・environment access・diagnostic/ErrorBody構築を持たない。
- `parse_intent` は size capを JSON parse前に評価し、post-parse gateを version→kind→opの順に評価する従来挙動を維持する。public API、schema、error code/message、accepted language、serde_json parse挙動、`id`/`params` payload identityは変更していない。

proofは production implementationと同じ `src/ipc.rs` の `#[cfg(kani)] mod proofs` に置く。これは proof placementのための cfgであり、製品挙動を `cfg(kani)` で置き換えていない。modelや test doubleは使っていない。

## 4. Kani proof

canonical command:

```sh
mise run test-formal-kani w-ipc-envelope
```

selectorは native/WSL2両分岐で `cargo kani -j1 --output-format terse -p ruviform-ui --harness w_ipc_envelope` を実行する。filterが選んだ harnessは次の 10件だけであり、退役した `parse_intent_error_contract` や `event_to_json_wellformed`、既存 `w_ipc_op_*` を巻き込まない。

| harness | bound / input domain | post-revert verification time | cover |
| --- | --- | ---: | --- |
| `w_ipc_envelope_size_exactness` | symbolic `usize` body length、unwind 2、assumeなし | 0.048038643 s | 4/4（0、cap-1、cap、cap+1） |
| `w_ipc_envelope_version_exactness` | symbolic `u32` version、valid kind/op固定、unwind 10、assumeなし | 0.22729509 s | 4/4 |
| `w_ipc_envelope_kind_length_exactness` | symbolic `usize` kind length、unwind 2、assumeなし | 0.029281205 s | 4/4 |
| `w_ipc_envelope_kind_content_exactness_len6` | symbolic `[u8; 6]` content、unwind 10、assumeなし | 0.0840906 s | 2/2 |
| `w_ipc_envelope_kind_exactness_len0_8` | symbolic slice len 0..8、入力域分割 `assume(len <= 8)`、unwind 10 | 0.5369595 s | 6/6 |
| `w_ipc_envelope_op_exactness_len0_8` | symbolic slice len 0..8、入力域分割 `assume(len <= 8)`、unwind 10 | 2.0370834 s | 4/4 |
| `w_ipc_envelope_op_boundary_len127_valid` | concrete len 127、unwind 131 | 4.509123 s | coverなし |
| `w_ipc_envelope_op_boundary_len128_valid` | concrete len 128、unwind 132 | 4.542554 s | coverなし |
| `w_ipc_envelope_op_boundary_len129_invalid` | concrete len 129、unwind 132 | 0.04668236 s | coverなし |
| `w_ipc_envelope_gate_priority` | symbolic version、kind/op len 0..8、入力域分割 `assume(len <= 8)`、unwind 12 | 2.4692264 s | 4/4 |

post-revert canonical runでは 10/10 harnessが `SUCCESSFUL`、0 failures、Unwinding Assertions有効だった。verification time合計は約 14.53 s（Kani報告値、compile時間を含まない）。len 0..8分割の `assume` は充足可能であり、coverは len 0/6/8、accept/reject、wrong-content same-length、priority各経路へ到達した。len 9以上、op 17..126 byte interior、129 byte超任意長はこの sliceの未証明領域であり、閉じた扱いにしない。

kind/op oracleは独立した byte-wise predicateで、production implementationの欠陥をコピーしない。op oracleは `W-IPC-OP` で使った independent oracle `valid_op_spec` を再利用する。gate priority exactnessは behavior-preserving refactorの oracleであり、error messageや gate priorityを新しい stable wire contractへ昇格させるものではない。

## 5. 未証明領域と補完

次は Kani proof boundaryに含めない。

- serde_json parser correctness、任意 malformed byte/text input、JSON object typing、missing/extra/duplicate fields
- `id`/`params` payload identity、float厳密性、ErrorBody message構築
- shellが coreを呼ぶ接続条件、UTF-8 validation、`String`/`Value`/allocation/Vec growth、I/O
- kind len 9以上、op 17..126 byte interior、129 byte超 arbitrary length
- `W-IPC-SERDE` typed Intent/Event serializationと `event_to_json_wellformed` の redesign
- `W-FRM-*` body transport/read classification/oversize drain、Archive残部、WAV残部

接続条件は `formal_ipc.rs` の deterministic `envelope_gate_connection_regression` と既存 proptestで確認する。deterministic regressionは exact-cap acceptance、cap+1 pre-parse `MESSAGE_TOO_LARGE`、valid envelope acceptance、wrong version/kind/op、複合違反時の version→kind→op priorityを直接確認する。message assertionは regression oracleであり、新しい stable wire contract宣言ではない。proptest regression seed fileは変更していない。

## 6. 実行環境とコスト

| 項目 | 値 |
| --- | --- |
| runner OS | Windows 11 x86_64 |
| Kani environment | WSL2 Ubuntu 24.04.4 x86_64 |
| kernel | `6.6.87.2-microsoft-standard-WSL2` |
| rustc / cargo（WSL） | 1.99.0 |
| Kani | 0.68.0 |
| CBMC | 6.11.0 |
| canonical wall clock | 07:50:47Z–07:51:11Z、約24 s |
| auxiliary resource command | `wsl.exe -d Ubuntu -- bash -lc "cd /mnt/g/orca/workspaces/ruviform/rf1015-ipc-envelope && CARGO_TARGET_DIR=\"$HOME/.cache/ruviform-kani/target\" /usr/bin/time -v cargo kani -j1 --output-format terse -p ruviform-ui --harness w_ipc_envelope"` |
| auxiliary measurement | wall 21.79 s、user 17.69 s、system 1.58 s、max RSS 273,764 KiB、exit 0 |
| 事前予算 | 10 CPU-min / 2 GiB |
| 判定 | 予算内。dominant costは len 127/128 boundaryの byte comparison展開と len 0..8 exactnessの symbolic byte window |

direct `cargo kani` と `/usr/bin/time` は補助証拠であり、canonical verdictは mise laneである。

旧 shell向け `parse_intent_error_contract` は rf806で約45 CPU-min・3.7 GiBを消費して verdictを返せなかった。本 sliceは shell proofを退役し、allocation-free decision subboundaryに限定したため、旧測定は「現行形が実用でない下限証拠」として残し、予算達成コストとはみなさない。

## 7. Mutation-fail

実施 mutantは `intent_body_size_decision` の `body_len <= MAX_INTENT_BYTES` を `body_len < MAX_INTENT_BYTES` へ変える 1箇所だけである。

- mutation run: 2026-11-27T07:50:02Z–07:50:28Z
- command: `mise run test-formal-kani w-ipc-envelope`
- exit code: 1
- 検出: `w_ipc_envelope_size_exactness` で `assertion failed: actual == IntentBodySizeDecision::Accept`。1/16 failed、cover 3/4
- 性質: exact-cap acceptanceの semantic violationであり、unwinding assertion failureではない
- revert: mutantを完全に revertし、`git diff --exit-code -- crates/ruviform-ui/src/ipc.rs` で差分なしを確認
- post-revert: selective Kani exit 0、`mise run test-formal` exit 0

mutationの failは lane regressionではなく検出成功として記録する。transient mutantは commitしていない。

## 8. 正規検証

| command | result | 備考 |
| --- | --- | --- |
| `mise run fmt-check` | exit 0 | rustfmt差分なし |
| `mise run check ruviform-ui` | exit 0 | 対象 crate check |
| `mise run lint ruviform-ui` | exit 0 | 対象 crate lint |
| `mise run test ruviform-ui` | exit 0 | `formal_ipc` 8/8を含む crate regression pass |
| `mise run test-formal` | exit 0 | `formal_ipc` 8/8、`formal_transport` 5/5、`formal_archive` 19/19、`formal_structured` 35/35、`formal_wav` 9/9 |
| `mise run test-formal-kani w-ipc-envelope` | exit 0 | 10/10 harness pass。Required proof |
| `mise run test-formal-kani invalid-selector` | exit 2 | usage validation negative smoke。environment detection/Kani実行前に停止 |
| `mise run test-formal-kani w-ipc-op` | NotApplicable | `valid_op_bytes` と既存 `w_ipc_op_*` proofに差分なし。envelope proofから呼ぶ共有実装は変更していない |
| selectorなし `mise run test-formal-kani` | NotRun | verdictを返せない legacy wire/archive shell harnessが残る aggregate laneを本 sliceの証拠にしない |
| Workspace full lane / Feature Matrix / MSRV / 3OS / GUI-L2 / EXT-L3 / 手動CI dispatch | NotRun | 本 sliceの Requiredではない。main統合検証は merge後のCI証拠 |

`mise exec -- prek run --all-files` の最終結果は `runs.jsonl` の `prek_all_files` recordを参照する。

## 9. consumer impact

2026-11-27T08:18:50Z–08:18:56Zに crate参照と exact symbol利用を分けて再確認した。

- crate参照: `rg -l 'ruviform_ui::ipc' <repo>/crates` は audio-lens 5 files、synvelis 49 files。代表例は audio-lens `crates/al-gui/src/desktop.rs:39`（`ErrorBody, JobState, codes`）、`crates/al-gui/src/service.rs:13`（`ErrorBody, codes`）、synvelis `crates/synvelis-app/src/app.rs:32`（`ErrorBody, codes`）、`crates/synvelis-app/src/binding.rs:37`（`ErrorBody, Event, Intent, JobState, LifecycleState, codes`）。
- exact symbol: `rg -n '(parse_intent|valid_op|MAX_INTENT_BYTES|intent_body_size_decision|intent_envelope_gate_decision)' <repo>/crates` は両 consumerとも 0件。`intent_body_size_decision` / `intent_envelope_gate_decision` は crate-privateで外部到達不能であり、`parse_intent` / `valid_op` / `MAX_INTENT_BYTES` の direct useもない。

したがって consumer側には `ruviform_ui::ipc` の型・定数・Event/Intentを使う crate参照がある一方、今回内部再配置した exact symbolや decision coreへの直接到達はない。runtime経由の間接利用はあり得るが、本 sliceは crate-private refactorであり、public API、schema、error code/message、size cap、accepted language、gate priority、serde_json parse挙動を変えないため、consumer independent regressionは `NotApplicable` と判断する。consumer repoのコード変更、Issue起票、pin更新、独立回帰は実施していない。この判断は Issue 全体の consumer regression ACを完了扱いにしない。

## 10. NotRun / Unsupported

- aggregate Kani lane: NotRun。legacy wire/archive shell harnessが verdictを返せない既知状態は解消していない。
- `W-IPC-SERDE`、`W-FRM-READCLASS`、`W-FRM-OVERSIZE`、Archive残部、WAV/read_frame/SampleFormat: 後続 slice。
- consumer independent regression: NotApplicable（第9節）。
- docs index（`docs/**/README.md`）: 編集しない。Coordinatorが統合時に整備する。
