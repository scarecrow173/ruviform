# rf1015 slice 18 — WAV-DECL-GATE lazy/pure diagnostic boundary 検証証跡

## Sync state

| 項目 | 値 |
| --- | --- |
| base SHA | `979128b4c400e7b1ed96f7a1a80479db83cdda49` |
| 検証対象 SHA / verified Sync Commit | `eee229d86507cd017c3c1691b4878255385f9e98` |
| 対象 group | `WAV-DECL-GATE`（直接依存する `WAV-HEADER` serialization non-regression を含む） |
| canonical selective lane | `mise run test-formal-kani wav-decl-gate` |

`eee229d8` は production decision core、Kani proof、regression test、selective lane を含む最後の製品/proof/mise 変更 commit である。後続 commit は docs/skill/evidence の同期に限定し、verified Sync Commit を動かさない。最終 PR head SHA は evidence が自分自身を固定点として書けないため PR 本文で記録する。

## 境界と保証

production core は `wire_declaration_decision` であり、`SampleFormat`、正の channel count、normalized 済みの正の rate から accept 時の `channels`/`block_align`/`byte_rate` または `Unsupported`/`InvalidValue` rejection class を返す。core は `io::Error`、`Error` payload、`format!`、allocation、Vec、header serialization、`SampleRate::new` normalization、環境アクセスを持たない。

`Wire::new` は scalar 抽出、core 呼び出し、rejection class の既存 `ErrorKind` への写像、accept 値への speaker mask/extensible 付与だけを行う thin shell である。unsupported rejection の production diagnostic は shell に残り、source の `io::ErrorKind::InvalidInput` と「対象 sample format が `wav_capabilities` の wire set にない」という meaning を維持する。`unsupported_sample_format` の `cfg(not(kani))`/`cfg(kani)` 二重実装は除去した。

Kani Pass に含めるのは core の membership exactness、channel/block_align width exactness、distinct block_align representative × byte-rate boundary ±1、rejection class exactness、および直接依存する fmt/header serialization non-regressionである。shell-core 接続、`ErrorKind` 写像、diagnostic payload meaning は deterministic regression/proptest の責務であり、Kani Pass に含めない。diagnostic text、`io::Error` 表現、Vec/header write、`SampleRate::new` normalization、SampleFormat canonical inventory、symbolic rate interior、任意 sink/任意 layout serialization は未証明領域として残る。channel/rate が正であることは production invariant に由来する入力域であり、overflow を除外する仮定ではない。手動 `%10` inventory は D-5 未解決の residual risk であり、将来 variant の恒久的 exhaustive proof と主張しない。

## 実行環境とコスト

canonical verdict は WSL2 (`Ubuntu`) 経路の mise lane で取得した。補助計測（canonical ではない）は同一 selector を WSL2 内で `/usr/bin/time -v` 付きに実行した結果である。

| 項目 | 値 |
| --- | --- |
| OS/Arch | WSL2 Ubuntu, Linux x86_64 |
| Kani | Kani Rust Verifier 0.68.0 (cargo plugin) |
| CBMC | 6.11.0 |
| canonical lane wall clock | 58 s |
| canonical harness verification time 合計 | 39.723 s（fmt/header 33.938 s が dominant cost） |
| auxiliary CPU time | user 50.79 s + system 4.50 s = 55.29 s（0.92 CPU-min） |
| auxiliary wall clock | 57.69 s |
| auxiliary peak memory | maximum resident set size 1,233,948 KiB（GNU `/usr/bin/time -v`、compile を含む） |
| 事前予算 | 20 CPU-min / 4 GiB |
| 判定 | 予算内 |

実行 harness は次の 6 件で、filter は `--harness wav_decl_gate` の substring selectionである。

| harness | verdict | verification time | cover |
| --- | --- | ---: | --- |
| `wav::proofs::wav_decl_gate_fmt_and_header_layout` | `SUCCESSFUL`、0/3897 failed（311 unreachable） | 33.93782 s | cover なし |
| `wav::proofs::wav_decl_gate_byte_rate_boundary_large_align` | `SUCCESSFUL`、0/181 failed（2 unreachable） | 1.3031628 s | 1/1 satisfied |
| `wav::proofs::wav_decl_gate_byte_rate_boundary_mid_align` | `SUCCESSFUL`、0/181 failed（2 unreachable） | 1.2767345 s | 1/1 satisfied |
| `wav::proofs::wav_decl_gate_byte_rate_boundary_small_align` | `SUCCESSFUL`、0/189 failed（2 unreachable） | 1.6473343 s | 2/2 satisfied |
| `wav::proofs::wav_decl_gate_width_exactness` | `SUCCESSFUL`、0/44 failed（1 unreachable） | 0.39271986 s | 4/4 satisfied |
| `wav::proofs::wav_decl_gate_membership_exactness` | `SUCCESSFUL`、0/113 failed（2 unreachable） | 1.1656963 s | 2/2 satisfied |

代表的な unreachable は rejected arm の不可達性や serialization matrix の dead branchであり、 unwinding assertion failure ではない。`fmt_and_header` は serialization の concrete matrixが支配コストで、pure scalar gate 自体は各 0.4–1.7 s に収まった。

## Mutation / Able to Fail

transient mutant は production core の byte-rate 計算を `rate * block_align + 1` へ 1 箇所だけ変更する off-by-one fault とした。commit には含めず、実行後に byte-for-byte で復元した（`cmp` 一致）。

- mutation run: `mise run test-formal-kani wav-decl-gate`、exit 1、lane verdict `fail`
- 検出: 5/6 harness が semantic assertion failure。代表反例は `u64::from(scalars.byte_rate) == product`、exact boundary acceptance、membership output equality。unwinding assertion failure の副産物ではない
- `wav_decl_gate_fmt_and_header_layout` は mutation を通さない serialization non-regressionとして `SUCCESSFUL` のままだった
- 完全復元後: 同 selective lane exit 0 / Pass、`mise run test-formal` exit 0、`mise run test ruviform-io-wav` exit 0

## 正規検証

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-io-wav` | 0 | Pass |
| `mise run lint ruviform-io-wav` | 0 | Pass |
| `mise run test ruviform-io-wav` | 0 | Pass（unit 7、contract 19、formal 10、roundtrip 9、doctest 3） |
| `mise run test-formal` | 0 | Pass（formal_wav 10/10 を含む） |
| `mise run test-formal-kani wav-decl-gate` | 0 | Pass（6/6 harness） |
| `mise run test-formal-kani invalid-selector` | 2 | 期待どおり Kani 実行前の usage error |
| controlled mutation → selective Kani | 1 | 検出成功（semantic failure） |
| post-revert selective Kani / test-formal / crate test | 0 / 0 / 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass |

## Consumer impact assessment（repo 内 baseline）

最終 PR head に対する consumer verdict は、SHA 関係を壊さないため repo 内 evidence に後追い commit せず、GitHub Actions artifact/Job Summary と PR 本文を正本とする。ここには checkout 時点の影響調査だけを記録する。

| consumer | checkout SHA | crate reference / exact symbol |
| --- | --- | --- |
| audio-lens | `0f1a1aac86336bff98df8bab1162e23bf5c36672` | `ruviform-io-wav` / `ruviform_io_wav` / `WavOutputSpec` / `make_encoder` / `open_muxer` / `wav_capabilities` の direct hit なし |
| synvelis | `a5e29228e6a08940e7396325daab138295a5e894` | crate reference: workspace/root `Cargo.toml`、`synvelis-render`、`synvelis-player`。exact symbol: `synvelis-render/src/preflight.rs` の `wav_capabilities`、`synvelis-render/src/export.rs` の `WavOutputSpec`/`make_encoder`/`open_muxer`/`wav_capabilities`、`synvelis-player/src/publish.rs` と複数 test の `WavOutputSpec`/`make_encoder`/`open_muxer` |

既知 baseline run 46144010070 は最終 PR head の candidate evidence ではない。Synvelis `test-rt` linker bus error が最終 candidate runで再現した場合も、ruviform 製品回帰・consumer 既存問題・インフラ/flaky を根拠なく混同せず、unknown または該当 lane 未達として分類する。

## 残る未証明領域

- shell-core 接続、`Wire::new` の rejection class → `ErrorKind` 写像、diagnostic payload meaning は Kani ではなく deterministic/proptest regression。
- diagnostic exact text、`io::Error` 表現、Vec/header write、`SampleRate::new` normalization、symbolic rate interior、任意 sink/任意 layout serialization。
- SampleFormat canonical inventory（D-5）。`%10` inventory は現行 10 variant に対する証明であり、将来 variant の恒久的 exhaustive proofではない。
- `open_muxer` Err-direction composite proof は退役済みのまま。byte-rate overflow decision は production core、shell `?` 伝播は regression coverage が担う。
