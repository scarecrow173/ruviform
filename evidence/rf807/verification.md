# rf807 — frozen DSP SIMD bench harness + ratio regression gate verification record

Issue `scarecrow173/ruviform#807` slice 1 の検証記録。canonical = `mise`
lane（`bench-frozen` / `bench-smoke`）。補助証拠（bench バイナリ直接
呼び出し・cargo 直接実行）は明記済み。

- Date: 2026-11-20
- OS: Windows (x86_64) — i9-9900K (16 threads), `detect()` → `avx2`
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `55e56b99067eca2124e3ff8bfcce6fef70fa862d`（ブランチ起点。
  各 JSONL の `repo_sha` は実行時 HEAD を記録）
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

## Actions → resulting state

| # | Action (command) | Resulting state |
| --- | --- | --- |
| 1 | `mise run bench-frozen`（実走・gate） | 6/6 cells `pass`（dot/256–4096 ratio 1.75–2.13 vs min_ratio 1.3–1.5、fir 1.46–1.75 vs 1.2–1.3）、`errors 0`、gate verdict `pass`・exit 0。記録: `run.jsonl` / `rf807-run.log` |
| 2 | `mise run bench-frozen --selftest`（B 側を fallback 注入） | 全 6 cells ratio ~1.0 < `min_ratio` で `fail`（reason `ratio_below_threshold`）、gate verdict `fail`・exit 1（task failed）。記録: `selftest.jsonl` / `rf807-selftest.log` |
| 3 | `mise run bench-frozen --test`（CI smoke 経路） | 1 rep・`gate_mode:"report_only"`・6/6 cells pass・gate verdict `not_run`（perf gate は smoke で強制しない）・exit 0。記録: `test-smoke.jsonl` / `rf807-test.log` |
| 3b | `mise run bench-frozen --test --baseline ../../evidence/rf807/dsp_baseline_high_threshold.demo.json`（smoke-fail 実演 — min_ratio=99.0 の合成 pin） | 6/6 cells が `fail` 診断（`ratio_below_threshold`）を保持しつつ、gate verdict `not_run`・exit 0 — perf gate が smoke でプロセスを落とさないことの実演。記録: `test-smoke-fail.jsonl`・合成 pin: `dsp_baseline_high_threshold.demo.json` |
| 4 | `mise run bench-smoke`（非回帰 — 既存 CI bench 契約） | 新規 `dsp_frozen` を含む全 5 crate の `--test` が完走・exit 0（出力内 `mode:"test"` 行を確認）。CI `bench` ジョブ契約不変 |
| 5 | `mise run bench-frozen --write-baseline ../../evidence/rf807/dsp_baseline.candidate.json` | `"approval":null` の candidate を原子的に出力（tmp+rename）・exit 0、gate は未評価（`verdict:"not_run"` / reason `write_baseline_mode`）。記録: `dsp_baseline.candidate.json` |
| 6 | candidate を pin へ promote（copy + `approval` 人手記入 → `benches/frozen/dsp_baseline.json`） | 以後 run/selftest の gate が pin を `baseline_status:"ok"` で読む（action 1–3 の状態） |
| 7 | 未 promote candidate を `--baseline` に指定した run（補助証拠・バイナリ直接） | `baseline has no approval` → 全 cells `not_run`・gate verdict `fail`（run モード fail-closed） |
| 8 | 存在しないパスを `--baseline` に指定した run（補助証拠・バイナリ直接） | baseline unreadable（os error 3）→ 全 cells `not_run`・gate verdict `fail`（不存在は pass 材料にならない） |

## Lane / verification results

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run bench-frozen` | Windows x64 | 0 | pass | gate pass・6/6 cells・errors 0 |
| `mise run bench-frozen --selftest` | Windows x64 | 1（task failed — 期待どおりの fail 実証） | fail | 注入 ratio ~1.0 < threshold で全 cell fail |
| `mise run bench-frozen --test` | Windows x64 | 0 | pass | smoke は report_only・gate verdict `not_run`（perf gate 非強制） |
| `mise run bench-frozen --test --baseline <99.0 pin>` | Windows x64 | 0 | pass | smoke-fail 実演 — cells fail 診断を残し gate `not_run`・exit 0（`output_mismatch` のみ smoke でも hard-fail） |
| `mise run bench-frozen --write-baseline …` | Windows x64 | 0 | written | candidate 出力のみ・gate なし |
| `mise run bench-smoke` | Windows x64 | 0 | pass | `dsp_frozen --test` 含め完走・CI 契約不変 |
| `mise run fmt-check` | Windows x64 | 0 | pass | `cargo fmt --all -- --check` |
| `mise run check ruviform-math` | Windows x64 | 0 | pass | `--all-targets`（bench ターゲット含む） |
| `mise run lint ruviform-math` | Windows x64 | 0 | pass | clippy `-D warnings` |
| `mise run test ruviform-math` | Windows x64 | 0 | pass | 全テスト pass |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | hook 全量（whitespace/EOF/yaml/toml/line-ending/BOM/key/actionlint） |

## Measured pins（i9-9900K / avx2）

| cell | measured_ratio | min_ratio (pinned) | errors | work_count | limiter |
| --- | --- | --- | --- | --- | --- |
| dot/256 | 1.77 | 1.3 | 0 | 58,720,256 | compute-bound（~1 KiB streams、dispatch コスト償却） |
| dot/1024 | 2.14 | 1.5 | 0 | 58,720,256 | compute-bound（~4 KiB streams） |
| dot/4096 | 2.01 | 1.5 | 0 | 58,720,256 | compute-bound（~16 KiB streams） |
| fir/256 | 1.47 | 1.2 | 0 | 917,504 | compute-bound（len×64-tap MAC、zero-history head は固定コスト） |
| fir/1024 | 1.77 | 1.3 | 0 | 917,504 | compute-bound（同上） |
| fir/4096 | 1.74 | 1.3 | 0 | 917,504 | compute-bound（同上） |

- 閾値ルール: `min_ratio = floor(1 + (measured − 1) × 0.5, 0.1)` — SIMD
  優位が pin 時の半分に縮退したら fail（dispatch が scalar に退化 →
  ratio ~1.0 で確実検出）。判断経緯は `decision.tsv` d01/d02。
- 絶対 ns 値は informational（`measured_*_median_ns`）。gate は ratio
  のみを見る（Coordinator 決定 — 機器非依存が主 gate）。
- `inner` は側ごとに calibrate（TARGET_REP_NS ≈ 300 µs/rep）。非対称に
  なり得るが per-call ns に正規化済みで ratio の意味は保たれる。

## 転用経緯（AC5）

- 入力生成器・等価性契約: `tests/simd_poc/{inputs,equiv}.rs` を
  `#[path]` include で転用（`benches/simd_poc.rs` の先例に倣う。
  `#[allow(dead_code)]` は include 側の未使用シンボル用 — 転用元
  ファイル自体は未変更）。
- env metadata・時間予算 calibrate・JSONL 実験ログ形式:
  `benches/simd_poc.rs` の先例。
- candidate → promote 2 相・atomic tmp+rename:
  `crates/ruviform-ui-test/src/golden/` BaselineStore 先例。
- verdict 語彙（pass/fail/not_run/unsupported）・fail-closed・
  `evidence/<issue>/` 規約: verify-ruviform skill（rf805/rf631 先例）。

## Honest notes

- `--selftest` は dispatch 退化の注入経路であり、実機退行時の検出
  感度は閾値設定（優位半減ルール）に依存する。
- pin 閾値は i9-9900K（avx2）実測由来。他機器では `measured_env` /
  各 run の `b_level` で環境差を観測する。3 OS 動作は `detect()` 任せ
  （本 slice の必須検証は Windows x64 のみ — 他 OS は未検証=NotRun）。
- baseline 未読時の gate は run モードでのみ fail-closed。`--test`
  smoke では report_only（CI bench 契約を変えないため）—— perf 系
  cell 失敗は診断として残るが gate verdict は `not_run` で exit 0。
  例外は `output_mismatch` のみ（正しさの欠陥は機器差ではないため
  smoke でも exit 1 — この判定は本 slice 内の設計判断）。
- 当初実装では `--test` で ratio 未達 cell が gate `fail` → exit 1
  になり得る欠陥があった（report_only が baseline.is_err のみを
  免除していた）。Completion Gate 指摘により修正 — 修正後の挙動は
  action 3b の smoke-fail 実演で検証済み。

---

# rf807 — frozen E2E pipeline bench（slice 2）verification record

Issue `scarecrow173/ruviform#807` slice 2 の検証記録。canonical =
`mise` lane（`bench-frozen-e2e`）。補助証拠（bench バイナリ直接
呼び出し）は明記済み。

- Date: 2026-11-21
- OS: Windows (x86_64) — i9-9900K (16 threads)
- Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13) / cargo 1.99.1
- Repo SHA: `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`（計測時 HEAD。
  各 JSONL の `repo_sha` は実行時 HEAD を記録）
- 証拠格付け: OS-L1 deterministic（scripted backend・ヘッドレス・
  手動 pump）。WASAPI/WinMM・実スレッド・GUI・plugin ABI の証拠
  ではない。callback 内 RT 不変条件は `rt_pipeline_*` テストの責務。
- ローカル検証 = 変更範囲の検証。統合検証は merge 後の対象 SHA に
  対する必須 CI 結果が証拠（本記録はそれを代替しない）。

## Actions → resulting state

| # | Action (command) | Resulting state |
| --- | --- | --- |
| E1 | `mise run bench-frozen-e2e`（実走・gate） | 2/2 cells `pass`（output/session norm 2.79 vs pin 3.97、input/typed-rt norm 3.12 vs pin 4.38）、errors 0、digest 一致、gate verdict `pass`・exit 0。記録: `e2e-run.jsonl` |
| E2 | `mise run bench-frozen-e2e --selftest`（wall floor + Xrun 注入） | 両 cell `fail`（norm 4.97/5.38 > pin、`device_errors 7` ずつ）、gate verdict `fail`・exit 1（期待どおりの fail 実証）。記録: `e2e-selftest.jsonl` |
| E3 | `mise run bench-frozen-e2e --test`（smoke 経路） | 1 rep・`gate_mode:"report_only"`・2/2 cells pass・gate verdict `not_run`・exit 0。記録: `e2e-test-smoke.jsonl` |
| E3b | `mise run bench-frozen-e2e --test --baseline ../../evidence/rf807/e2e_baseline_low_threshold.demo.json`（smoke-fail 実演 — max_norm_ratio=0.01 の合成 pin） | 両 cell が `fail` 診断（`norm_above_threshold`）を保持しつつ、gate verdict `not_run`・exit 0 — perf gate が smoke でプロセスを落とさないことの実演。記録: `e2e-test-smoke-fail.jsonl`・合成 pin: `e2e_baseline_low_threshold.demo.json` |
| E3c | `mise run bench-frozen-e2e --test --baseline ../../evidence/rf807/e2e_baseline_bad_digest.demo.json`（smoke correctness 実演 — expected_digest=0x0 の合成 pin） | 両 cell が `fail`（`digest_mismatch`）→ gate verdict `fail`・exit 1 — correctness 欠陥は smoke でも hard-fail することの実演。記録: `e2e-test-smoke-digest.jsonl`・合成 pin: `e2e_baseline_bad_digest.demo.json` |
| E4 | `mise run bench-frozen-e2e --write-baseline <path>` | `"approval":null` の candidate を出力・exit 0、gate 未評価。記録: `e2e_baseline.candidate.json`（計測 norm 2.642 / 2.918、candidate 閾値 3.97 / 4.38） |
| E5 | candidate を pin へ promote（copy + `approval` 人手記入 → `benches/frozen/pipeline_baseline.json`） | 以後 run/selftest の gate が pin を `baseline_status:"ok"` で読む |
| E6 | 未 promote candidate を `--baseline` に指定した run（補助証拠・バイナリ直接） | `baseline has no approval` → 全 cells `not_run`・gate verdict `fail`・exit 1 |
| E7 | 存在しないパスを `--baseline` に指定した run（補助証拠・バイナリ直接） | baseline unreadable（os error 2）→ 全 cells `not_run`・gate verdict `fail`・exit 1 |
| E8 | `mise run bench-frozen`（非回帰 — slice 1 lane） | 6/6 cells pass・exit 0 — 既存 lane 無変更 |
| E9 | `mise run bench-smoke`（非回帰 — 既存 CI bench 契約） | 対象 5 crate の `--test` 完走・exit 0（`ruviform-audio-test` は対象外のまま。CI 契約不変） |

## Lane / verification results

| Command | OS | Exit | Verdict | 備考 |
| --- | --- | --- | --- | --- |
| `mise run bench-frozen-e2e` | Windows x64 | 0 | pass | gate pass・2/2 cells・errors 0・digest 一致 |
| `mise run bench-frozen-e2e --selftest` | Windows x64 | 1 | fail | wall floor + Xrun 注入で両 cell fail（期待どおりの実証） |
| `mise run bench-frozen-e2e --test` | Windows x64 | 0 | not_run | smoke は report_only・gate verdict `not_run`（perf gate 非強制） |
| `mise run bench-frozen-e2e --test --baseline <0.01 pin>` | Windows x64 | 0 | not_run | smoke-fail 実演 — cells fail 診断を残し gate `not_run`・exit 0 |
| `mise run bench-frozen-e2e --test --baseline <bad-digest pin>` | Windows x64 | 1 | fail | correctness（digest_mismatch）は smoke でも hard-fail |
| `mise run bench-frozen-e2e --write-baseline …` | Windows x64 | 0 | written | candidate 出力のみ・gate なし |
| `mise run bench-frozen` | Windows x64 | 0 | pass | slice 1 lane 非回帰 |
| `mise run bench-smoke` | Windows x64 | 0 | pass | 既存 5 crate 契約不変（audio-test bench は対象外） |
| `mise run fmt-check` | Windows x64 | 0 | pass | `cargo fmt --all -- --check` |
| `mise run check ruviform-audio-test` | Windows x64 | 0 | pass | `--all-targets`（bench ターゲット含む） |
| `mise run lint ruviform-audio-test` | Windows x64 | 0 | pass | clippy `-D warnings` |
| `mise run test ruviform-audio-test` | Windows x64 | 0 | pass | 全テスト pass |
| `mise exec -- prek run --all-files` | Windows x64 | 0 | pass | hook 全量 |

## Measured pins（i9-9900K / x86_64）

| cell | measured_norm | max_norm_ratio (pinned) | errors | work_count | limiter |
| --- | --- | --- | --- | --- | --- |
| output/session | 2.64 | 3.97 | 0 | 229,376 | driver pump + backend callback machinery + Session plan.drive + fixture render + captured-output record |
| input/typed-rt | 2.92 | 4.38 | 0 | 229,376 | driver pump + capture offer/drain + input_writer copy + typed process (GainNode) + output_reader observe |

- 閾値ルール: `max_norm_ratio ≈ measured_norm × 1.5`（ceil 0.01）。
  per-callback / per-block コストが pin 時の ~1.5 倍に増えたら fail。
  判断経緯は `decision.tsv` d07。
- 絶対 ns 値は informational（cell の `median_ns`）。gate は norm
  （in-process ref kernel 中央値比）のみを見る — 機器速度相殺。
- rep は warmup 1 + timed 7、各 rep = 64 callback × 256 frame × 2ch。
  fixture = deterministic noise（seed 8829395791360・amp 0.5）。
  input 側 gain = 0.5。
- digest は観測出力の FNV-1a fingerprint（全 rep で不変のはず）。
  pin の `expected_digest` と一致しない場合は correctness 失敗
  （`digest_mismatch` — `--test` smoke でも hard-fail）。

## 計測経路（本番経路証明 — AC 対応）

- `output/session`: `PlaybackPipeline::open` → `Session::open` +
  `Session::start`（production GraphPlan closure → fixture render）。
  計時区内は `handle.pump_output_next()` が production `plan.drive`
  callback を発火。backend captured PCM を `expected_fixture_block`
  oracle と block ごとに比較。
- `input/typed-rt`: scripted input callback → `InputCapture` 経由で
  `InputPipeline::pump_block` が `input_writer` → planar copy →
  `rt.process`（GainNode × 0.5）→ `output_reader` の production
  handoff を実行。観測 planar PCM を `scale_expected` oracle と比較。
- oracle は `oracle.rs` の独立実装（`PcmFixture::fill`/`sample_at` を
  呼ばず fixture 仕様を再実装）— 計測対象の production コードと
  期待値を共有しない。

## Honest notes

- scripted backend の決定論的成功は OS-L1 証拠のみ。WASAPI/WinMM・
  実スレッド RT・GUI・plugin ABI の証拠ではない（callback 発火は
  手動 pump — 並行 callback 競合は本 lane の境界外）。
- `--selftest` は wall floor（`(pin + 1) × ref median`）と scripted
  Xrun の注入経路であり、実機退行時の検出感度は閾値設定
  （実測 × ~1.5）に依存する。
- pin 閾値は i9-9900K 実測由来。norm は ref kernel で機器速度を
  相殺するが backend/scheduler jitter の分布は機器差で揺れ得る —
  環境差は `measured_env` / 各 run の metadata で観測する。
- `--test` smoke では perf 系 cell 失敗（`norm_above_threshold`）は
  診断に留まり gate verdict `not_run`・exit 0。correctness 系
  （`digest_mismatch`・`pcm_mismatch`・`device_errors`・
  `capture_loss`・`pipeline_errors`・`rep_divergence`）は smoke
  でも gate `fail`・exit 1（正しさの欠陥は機器差ではない）。
- bench バイナリの cwd は package dir（cargo bench の規約）。
  `--baseline` / `--write-baseline` の相対パスは
  `crates/ruviform-audio-test/` 起点で解決される（`../../evidence/`
  表記を使うこと）。
