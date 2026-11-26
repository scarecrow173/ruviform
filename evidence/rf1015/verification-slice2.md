# rf1015 slice 2 verification — `W-IPC-OP` pure decision core

## 基本情報

- 作業日: 2026-11-26。実行時刻は UTC で記録する。
- 対象 Issue: `scarecrow173/ruviform#1015`（slice 2。Issue は close しない）。
- branch: `test/1015-valid-op-proof-core`。
- 作業開始時 base SHA: `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`。
- 最終検証前に取り込んだ `origin/main`: `0fb97d1ba89e1fee154cdbbb72ceb2db15fdcd8c`。差分は `crates/ruviform-audio-device/src/wasapi_input.rs` のみで、本 slice の編集範囲と重ならなかった。最終検証はこの SHA へ fast-forward した branch 上の branch diff に対するローカル検証であり、merge 後 main SHA の統合検証ではない。
- 検証対象 SHA / verified Sync Commit: `5024268f56339c91ba0487fe8d87198a8d529cab`。selective Kani、proptest、crate regression を実行した working tree の製品/proof/mise 内容は、この commit で固定したものと一致する。mutation run は同じ内容へ長さ上限 `+1` の transient mutant を載せて実行し、revert 済みである。`15a9273a04120aa144b21802d5ddecc767aefb46` はその親実装 commitである。
- 初期 evidence sync commit: `8cb8a6d58933d04415cfe47f92381a913c547d52`。この commit以降は docs/skill/evidence-only 差分でなければならず、製品/proof/mise 差分が加わった場合は再検証なしに verified Sync Commitを進めない。
- 最終 head SHA: evidence は自分自身を含む commit hashを固定点として書けないため、PR #1027 の head と本文で追跡する。最終 headと `5024268f` の差分が docs/skill/evidence-onlyであることが、本記録の検証を最終 headへ適用する条件である。
- OS/Arch: Windows 11 x86_64（MSYS2/MinGW shell）+ WSL2 Ubuntu 24.04.4 LTS x86_64、kernel `6.6.87.2-microsoft-standard-WSL2`。
- tool version: Windows 側 `rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`、`mise 2026.11.2 windows-x64`。WSL2 側 `rustc 1.99.0 (b940084d7 2026-09-28)`、Kani Rust Verifier 0.68.0、CBMC 6.11.0、solver は Kani default の CaDiCaL。
- 変更区分: `valid_op` の内部 refactor、byte-core proof、selective Kani entry、doc/evidence sync。公開 signature、`MAX_OP_LEN`、accepted language、`parse_intent` の error code と優先順位は変えていない。

## 実装と証明境界

`valid_op(&str)` は `op.as_bytes()` を `valid_op_bytes(&[u8])` へ渡す thin wrapper になった。`valid_op_bytes` は crate-private で、allocation、UTF-8/`char` 走査、I/O、診断構築を持たない。長さ 0 と `MAX_OP_LEN` 超を先に拒否し、残りを index ではなく byte 値を読む単純な loop で判定する。

| harness | 対象と bound | 確認した領域 | 未証明領域 |
| --- | --- | --- | --- |
| `w_ipc_op_exactness_len0_16` | 製品 `valid_op_bytes` と独立 reference predicate `valid_op_spec` の一致。symbolic byte slice 長 0〜16、`#[kani::unwind(20)]`。`len <= 16` は入力域分割の assume | 長 0〜16 の全 byte 列。cover は len 0、len 16、valid len 16、invalid len 16 の 4/4 satisfied | 17〜126 byte、`MAX_OP_LEN` boundary、128 byte 超 |
| `w_ipc_op_boundary_len127_valid` | concrete 長 127 valid witness、`#[kani::unwind(131)]` | `MAX_OP_LEN - 1` の valid 入力を受理すること | 長 127 の他の grammar 変種 |
| `w_ipc_op_boundary_len128_valid` | concrete 長 128 valid witness、`#[kani::unwind(132)]` | `MAX_OP_LEN` の valid 入力を受理すること | 長 128 の他の grammar 変種 |
| `w_ipc_op_boundary_len129_over_cap` | concrete 長 129、`#[kani::unwind(132)]` | 他の grammar 条件を満たす長 129 入力を cap で拒否すること | 129 byte を超える任意長 |

Unwinding Assertions は無効化していない。`--no-unwinding-checks`、verified/unverified stub、`cfg(kani)` による製品実装置換、入力域を黙って縮める assume は使っていない。`len <= 16` は型上到達不能入力の排除ではなく domain partition であり、17〜126 byte を未証明領域として残す。assume の充足可能性と必要経路到達性は、len 0/16 と valid/invalid len 16 の 4 件の cover witness で確認した。

boundary matrix は長さごとに 3 harness へ分割した。当初の単一 harness では 127/128/129 の最大 unroll が 1 回の solver invocation に結合し、共有 WSL2 host 上で完了する場合と数分経っても verdict を返さない場合があった。分割後は各 harness が独立し、len 129 は cap の early rejection で loop に入らないため非 mutant では 0.1 s 程度で終わる。len 129 harness の unwind を 132 にしているのは、cap を緩める mutant が loop 全体を実行したうえで semantic assertion failure になることを確認するためであり、bound 超過を隠すためではない。

旧 `valid_op_soundness` harness は退役させた。旧 harness は 8 byte の valid UTF-8 `&str` に対する soundness だけを対象とし、verdict を返せない cost envelope だった。新保証は 0〜16 byte の受理/拒否 exactness と 127/128/129 boundary に再構成される。arbitrary Unicode 入力、`&str` wrapper の接続、大域的な totalness は既存 proptest が担う。

## 実行した正規検証

canonical command は mise/prek 経由。すべてローカル検証であり、merge 後 main SHA の統合検証ではない。機械可読版は `runs.jsonl`。

| lane / command | 開始（UTC） | 終了（UTC） | exit code | verdict | 結果 |
| --- | --- | --- | ---: | --- | --- |
| `mise run test-formal-kani w-ipc-op` | 2026-11-26T06:00:37Z | 2026-11-26T06:02:13Z | 0 | Pass | post-merge/post-revert canonical proof run。4/4 harness successful |
| `mise run test-formal-kani invalid-selector` | 2026-11-26T06:02:13Z | 2026-11-26T06:02:13Z | 2 | Pass（usage validation） | Kani 実行前に `usage: mise run test-formal-kani [w-ipc-op]` で停止。期待どおり exit 2 |
| `mise run fmt-check` | 2026-11-26T06:02:13Z | 2026-11-26T06:02:27Z | 0 | Pass | `cargo fmt --all -- --check` が差分なし |
| `mise run check ruviform-ui` | 2026-11-26T06:02:27Z | 2026-11-26T06:02:29Z | 0 | Pass | `cargo check --locked --all-targets -p ruviform-ui` passed |
| `mise run lint ruviform-ui` | 2026-11-26T06:02:29Z | 2026-11-26T06:02:32Z | 0 | Pass | clippy `-D warnings` passed |
| `mise run test ruviform-ui` | 2026-11-26T06:02:32Z | 2026-11-26T06:02:36Z | 0 | Pass | unit/integration/doc tests passed。lib 105、integration test 群を含む全 test result ok |
| `mise run test-formal` | 2026-11-26T06:02:36Z | 2026-11-26T06:02:45Z | 0 | Pass | post-revert run。`formal_ipc` 7、`formal_transport` 4、`formal_archive` 18、`formal_structured` 35、`formal_wav` 9、計 73/73 passed |
| `mise exec -- prek run --all-files` | 2026-11-26T07:09:02Z | 2026-11-26T07:09:06Z | 0 | Pass | 初期 evidence sync commit `8cb8a6d` での exact run。全 configured hook passed。auto-fix 差分は残っていない |

補助証拠（canonical = mise）として、作業中に direct `cargo kani` を実行して harness ごとの完了時間と filter 選択を確認した。direct run は canonical verdict には使っていない。

## Kani verdict、コスト、環境

post-merge/post-revert canonical run の harness filter は `w_ipc_op`、実行された harness は `ipc::proofs::w_ipc_op_boundary_len129_over_cap`、`ipc::proofs::w_ipc_op_boundary_len128_valid`、`ipc::proofs::w_ipc_op_boundary_len127_valid`、`ipc::proofs::w_ipc_op_exactness_len0_16` の 4 件だけだった。filter が他 proof を巻き込まないことを出力で確認した。

| harness | verdict | checks | verification time |
| --- | --- | --- | ---: |
| `w_ipc_op_boundary_len129_over_cap` | `SUCCESSFUL` | 0 of 90 failed（1 unreachable） | 0.111670546 s |
| `w_ipc_op_boundary_len128_valid` | `SUCCESSFUL` | 0 of 90 failed（1 unreachable） | 21.240137 s |
| `w_ipc_op_boundary_len127_valid` | `SUCCESSFUL` | 0 of 90 failed（1 unreachable） | 24.873161 s |
| `w_ipc_op_exactness_len0_16` | `SUCCESSFUL` | 0 of 146 failed（1 unreachable）、cover 4/4 satisfied | 24.536709 s |
| 合計 | Pass | 4/4 harness | 70.761677546 s |

wall clock は mise lane全体で約 96 s だった。verification time は compile 時間を含まない Kani 報告値であり、wall clock は compile・harness生成・process起動を含む。どちらも事前予算 5 CPU-min（300 s）以内である。終了理由は各 harness の CBMC `VERIFICATION:- SUCCESSFUL` であり、Timeout/OOM/exit 0/CI green を Proof Pass に転用していない。

peak memory は補助証拠（canonical = mise）である。同一 proof code に対する canonical run の実行中、WSL2 側で `/proc/<pid>/status` の `VmHWM` を 0.2 s 間隔で観測した。観測値は exactness 189,680 KiB、len127 229,488 KiB、len128 231,740 KiB、lane observed max 231,740 KiB（約 0.221 GiB）だった。len129 は 0.1 s 程度で終わるため sampling に乗らず、個別 peak は unsupported とする。len129 が cap early rejection で loop に入らないこと、他 harness の観測値が 1 GiB 予算の 4 分の 1 未満であることから、lane 全体の観測上限は 1 GiB 以内と記録する。memory monitor は一時 script であり、repo へ残していない。

## Mutation-fail と revert

実施した mutant は 1 箇所だけである。

```diff
-    if op.is_empty() || op.len() > MAX_OP_LEN {
+    if op.is_empty() || op.len() > MAX_OP_LEN + 1 {
```

mutation は fast-forward 前の base `87a7ba8f` 上の同一 branch diff で実施した。2026-11-26T05:54:43Z から 2026-11-26T05:55:32Z まで `mise run test-formal-kani w-ipc-op` を実行し、exit 1、lane verdict `fail` を観測した。`w_ipc_op_boundary_len129_over_cap` で `assertion failed: !valid_op_bytes(&OP_129)` が `1 of 91 failed` となり、長 129 を誤って受理する mutant を semantic assertion failure として検出した。verification time は 8.459146 s だった。他の 3 harness は成功したが、len 0〜16 の exactness は cap mutation が domain 外なので検出しない設計であり、boundary proof が owner である。

mutation 直後に元の predicate へ revert し、`MAX_OP_LEN + 1` が `src/ipc.rs` に残っていないことを確認した。revert 後かつ最新 main 取込後の selective Kani は exit 0、4/4 harness successful、`mise run test-formal` も exit 0 で緑へ戻った。mutant は commit していない。`formal_ipc.proptest-regressions` に mutation 由来の差分は残っていない。

なお、len129 harnessが `#[kani::unwind(8)]` だった初期版でも mutant は unwinding assertion failureで停止したが、それは semantic violation ではなく harness bound の副産物だった。検出能力を property failure として示すため、len129 harnessを 132 へ調整して mutation を再実施した。非 mutant では cap early rejection により loopへ入らないため、この調整は proof の成功条件を弱めない。

## consumer impact assessment

2026-11-26 に consumer repo を再確認した。

```sh
rg -n "\b(valid_op|parse_intent)\b" \
  G:/repositry/audio-products/audio-lens/crates \
  G:/repositry/audio-products/synvelis/crates
```

一致は 0 件だった。`valid_op` の公開 signature、accepted language、`MAX_OP_LEN`、`parse_intent` の error behavior も変えていないため、audio-lens/synvelis の独立回帰はこの slice では `NotApplicable` と判断する。この判断は Issue 全体の consumer 回帰 AC を完了扱いにしない。

## 実行しなかった lane と理由

| lane / 検証 | state / verdict | 理由 |
| --- | --- | --- |
| selector なし `mise run test-formal-kani` | Required ではない / NotRun | 現行 default lane には verdict を返せない既知の wire/archive harness と structured legacy exception が残る。本 slice の証拠にしない dispatch 指示どおり実行しなかった |
| Workspace full lane、Feature Matrix、MSRV、3OS 横断 | NotApplicable / NotRun | 変更範囲に比例した検証原則。対象は `ruviform-ui` と selective Kani lane |
| GUI-L2 / EXT-L3 | NotApplicable | UI 実行面・外部実機面を変更していない |
| 手動 CI dispatch | NotApplicable | 通常 slice は CI 非ブロッキング。merge 後 main SHA の必須 CI が統合検証の証拠 |
| consumer 独立回帰 | NotApplicable | 直接利用 0 件、public behavior unchanged |

## sync state

| 対象 | state | 根拠 |
| --- | --- | --- |
| `W-IPC-OP` ledger row | `synced` | assurance kind、target、入力域、bound、assume、未証明領域、旧保証差、verdict、選択再実行、mutation、実測コストを更新 |
| `docs/specs/formal-verification.md` | `synced` | 新 proof 境界、selective entry、旧 `valid_op_soundness` の退役、mutation 結果を同期。rf#806 inventory は削除していない |
| `.agents/skills/verify-ruviform/SKILL.md` | `synced` | selective entry、in-src proof placement、unknown selector、mutation 責務を同期。英語中心の現行スタイルを維持 |
| `mise.toml` | `synced` | `w-ipc-op` selector だけを追加し、selector なし aggregate と structured legacy exception の挙動は拡大していない |
| 他 Property group | `design_only` / `not_synced` / `deferred` | この slice の成果として昇格していない |
| `evidence/rf806/` | `synced`（read-only 参照） | 改変していない |
| docs index | `not_synced`（許容） | dispatch 契約どおり `docs/**/README.md` は編集・再生成していない。Coordinator が統合時に整備する |

## 残留リスク・未解決事項

- 17〜126 byte interior と 129 byte を超える任意長は Kani では証明していない。proptest の補完を維持する必要がある。
- boundary matrix は length cap の concrete witness であり、大入力の個別 grammar 変種を尽くしていない。個別 grammar 条件は 0〜16 byte exactness に依存する。
- `valid_op` wrapper の接続は proptest と compile/type boundary で確認する範囲にとどまり、独立した Kani connection proof はない。wrapper は `as_bytes()` を渡すだけの thin wrapper である。
- 他の wire/archive harness、structured legacy exception、WAV diagnostic boundary、read_frame EOF、SampleFormat inventory は後続 slice に残る。
- 共有 WSL2 host では別 workload による負荷変動がある。verification time は budget 判定の実測値として記録するが、性能回帰の閾値として使わない。
