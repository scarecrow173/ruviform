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
| 3 | `mise run bench-frozen --test`（CI smoke 経路） | 1 rep・`gate_mode:"report_only"`・6/6 cells pass・exit 0。hard-fail しない。記録: `test-smoke.jsonl` / `rf807-test.log` |
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
| `mise run bench-frozen --test` | Windows x64 | 0 | pass | smoke は report_only で hard-fail しない |
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
  smoke では report_only（CI bench 契約を変えないため）。
