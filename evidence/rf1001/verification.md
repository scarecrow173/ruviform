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

この節は workflow dispatch artifact の取得後に確定する。判定は run conclusion ではなく、次の artifact を読む。

- `vaudio-probe-windows-2025.json` の `os-endpoint-wiring` stage record
- `stage-os-endpoint-wiring-windows-2025.txt`
- `os-endpoint-run-metadata-windows-2025/run-metadata-rf629-audio-virtual-endpoint-render-capture.json`
- `os-endpoint-run-metadata-windows-2025/run-metadata-rf629-midi-virtual-endpoint-loopback.json`

pass 条件: 両 leg `result:"pass"`、`level:"ext-l3"`、`environment_kind:"virtual-os-endpoint"`、`repo:"scarecrow173/ruviform"`、SHA が dispatch した branch head と一致し、stage log に対応する `verdict: Pass` 行があること。

## 受入条件メモ

- 未 provisioned fail-closed: ローカルで両 leg NotRun を観測（上表）。
- provisioned Pass: branch-ref workflow artifact で確定（このファイルに追記）。
- canonical lane: `mise run test-os-endpoint-wiring` exit 0 を確認。

## 残課題

- merge 後 main ref dispatch は Coordinator の統合検証範囲。
- docs index は生成物であり本 Worker では編集しない。
