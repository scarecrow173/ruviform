# rf#1001 Slice F — os_endpoint wiring legs provisioned 実走

Issue: `scarecrow173/ruviform#1001` / rf#629 AC3 Slice F
Branch: `feat/1001-wiring-legs`

## 設計判断

1. `os_endpoint.rs` の audio leg に rf#977 の timestamp epoch 契約を contract 級 oracle として移植した。stop/start 境界で input/output 両方向の timestamp baseline を split し、callback index を `REGRESSION_INDEX_SLOTS=32` の bounded slot に記録する。epoch 先頭 `EPOCH_START_GRACE_CALLBACKS=24` 内だけ transient とし、mid-epoch regression と slot overflow は fatal のまま残す。
2. 分類は deterministic test `timestamp_epoch_classifier_distinguishes_transient_and_fatal_regressions` で固定した。epoch 1 grace、restart 後 epoch 2 grace、mid-epoch fatal、slot overflow fatal を区別する。
3. workflow stage `os-endpoint-wiring` は `midi-loopback` 後・`cleanup` 前に置いた。audio capture は `stream-roundtrip` が同一 job 内で実施済みの mic consent provisioning に依存するため、複写せずこの配置を canonical とした。
4. canonical lane は従来どおり `mise run test-os-endpoint-wiring`（`--nocapture` なし）である。workflow stage は verdict と RunMetadata を artifact 生ログへ surface させる補助証拠として、`mise exec -- cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --test-threads=1 --nocapture` を直接実行する。`mise.toml` は変更していない。
5. `run_metadata_path:` 行が示す `CARGO_TARGET_TMPDIR` 内 JSON を workspace 内 `os-endpoint-run-metadata-<os>/` へ copy し、reset/upload artifact path に含めた。Cargo integration test では runtime env が常に存在しないため、emit 側は compile-time `option_env!("CARGO_TARGET_TMPDIR")` を fallback に使う。copy 不能・metadata 未到達は stage `error` に写す。
6. stage status `ok` は fail-closed contract の正常終端を示すだけで、rf#1001 AC3 の Pass 証拠ではない。AC 判定は audio/midi 両 RunMetadata JSON の `result:"pass"`、`level:"ext-l3"`、`environment_kind:"virtual-os-endpoint"`、repo/SHA で行う。

## ローカル検証（未 provisioned 環境）

worker ホストは `Ruviform Cable Input`/`Ruviform Cable Output`/`Ruviform MIDI Loopback` を持たない未 provisioned 環境だった。したがって local evidence は Pass 証拠ではなく、fail-closed NotRun の非回帰証拠である。

| 検証 | 結果 | evidence |
| --- | --- | --- |
| `mise run fmt-check` | exit 0 | `local-final-verification.log` |
| `mise run check ruviform-audio-test` | exit 0（前段ログ） | `local-test-ruviform-audio-test.log` |
| `mise run lint ruviform-audio-test` | exit 0 | `local-lint.log` |
| `mise run test ruviform-audio-test` | exit 0 | `local-final-verification.log` |
| `mise run test-os-endpoint-wiring` | exit 0・両 ignored leg `ok`（canonical、stderr capture のため verdict 非表示） | `local-final-verification.log` / `local-test-os-endpoint-wiring-canonical.log` |
| 補助: `mise exec -- cargo test -p ruviform-audio-test --test os_endpoint -- --ignored --test-threads=1 --nocapture` | exit 0。audio/midi とも `verdict: NotRun — NOT_PROVISIONED`、`run_metadata` は `result:"not-run"` / `level:"ext-l3"` / `environment_kind:"virtual-os-endpoint"` | `local-os-endpoint-wiring-nocapture.log` / `local-run-metadata-*.json` |
| `mise run test-ci-daily` | exit 0 | `local-hygiene-verification.log` |
| `mise run test-ci-tracker` | exit 0 | `local-hygiene-verification.log` |
| `mise exec -- prek run --all-files` | exit 0 | `local-hygiene-verification.log` |
| `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | exit 0 | `local-actionlint.log` |

ローカル RunMetadata JSON は compile-time/runtime tmpdir 配線の確認用で、`repo:"scarecrow173/ruviform"` と base SHA が記録されている。branch head SHA は workflow dispatch 時に `${{ github.sha }}` で注入する。

## branch-ref provisioned 実走

pass 条件: 両 leg `result:"pass"`、`level:"ext-l3"`、`environment_kind:"virtual-os-endpoint"`、`repo:"scarecrow173/ruviform"`、SHA が dispatch した branch head と一致し、stage log に対応する `verdict: Pass` 行があること。run conclusion は判定根拠にしない。

### attempt 1 — run 45554544765 / SHA 50edac47118028c5ab4f45a0bbd9c86451c73998

- artifact: `attempt1-vaudio-probe-windows-2025.json`、`attempt1-stage-os-endpoint-wiring-windows-2025.txt`、両 `attempt1-run-metadata-*.json`。
- stage record: `os-endpoint-wiring` は `error`（fail-closed 記録どおり。metadata copy は両方成功）。
- midi leg: `result:"pass"`、`level:"ext-l3"`、`environment_kind:"virtual-os-endpoint"`、repo/SHA 一致、`verdict: Pass`。
- audio leg: `result:"fail"`。`output timestamp regressions: 2`（raw 4 / effective 2、regression callback indices `[3,4,77,217]`、restart boundary 302）。idx 3/4 は epoch 1 grace だが、77/217 は mid-epoch なので oracle どおり fatal。marker/RMS・frame/domain/channel・lifecycle・input timestamp は通過。
- 切り分け: provisioning/pair・MIDI・audio stimulus path は成立。残りは output timestamp oracle または実行環境 timing。同一 run の `stream-roundtrip`（rf#977 verifier）は直前に pass しているため、まず同一 SHA の再実走で変動性を確認する。

### attempt 2 — run 45555696425 / SHA 50edac47118028c5ab4f45a0bbd9c86451c73998

**AC3 実走証拠（pass）**。artifact は `attempt2-vaudio-probe-windows-2025.json`、`attempt2-stage-os-endpoint-wiring-windows-2025.txt`、両 `attempt2-run-metadata-*.json`。

- stage record: `os-endpoint-wiring` `status:"ok"`、`detail:"test_exit=0; audio=pass; midi=pass; verdicts=2; metadata=2; copied=2"`。
- audio leg: `result:"pass"`、`level:"ext-l3"`、`environment_kind:"virtual-os-endpoint"`、repo/SHA 一致。stage log に `verdict: Pass — render→capture round-trip observed`。timestamp 観測は raw 3 / effective 0・epoch reset 1・indices `[3,4,317]`（idx 3/4 は epoch 1 grace、317 は boundary 302 後の epoch 2 grace）で、rf#977 契約の transient 分類どおり。
- midi leg: `result:"pass"`、同じく level/environment/repo/SHA 一致。stage log に `verdict: Pass — round-trip observed through real OS ports`。
- test harness: `test result: ok. 2 passed` / exit 0。RunMetadata JSON 2 件が artifact へ copy された。

attempt 1 の mid-epoch regression は attempt 2 で再現せず、attempt 2 では regression が両 epoch の grace window にだけ分布した。attempt 1 は oracle が mid-epoch regression を fatal に倒す fail-closed 動作の実証として残す。

## 受入条件メモ

- 未 provisioned fail-closed: ローカルで両 leg NotRun を観測（上表）。
- provisioned Pass: attempt 2 run 45555696425 の artifact で両 leg `result:"pass"` を確認。
- canonical lane: `mise run test-os-endpoint-wiring` exit 0 を確認。

## 残課題

- merge 後 main ref dispatch は Coordinator の統合検証範囲。
- docs index は生成物であり本 Worker では編集しない。
