# window-variant loop の残存失敗記録（superseded）

exact-attribution pivot **前**の window 維持版 oracle で実行した
40-iter dual-binary loop（`loop3.sh`、iters 31-40 は concurrent
`cargo build` 負荷併走）の失敗記録。生ログは exact-attribution 版
の再実行で `logs3/`・`logs4/` が上書きされたため、本ファイルは
観測時の panic 出力を転記した記録である（生ログではない）。

## 失敗一覧（3 run / 80 binary-runs）

| iter | binary | crate | test | exit | panic |
| --- | --- | --- | --- | --- | --- |
| 8  | a | clap | `real_test_plugin_l1_instrument_cc_and_offset_gating` | 101 | `the full-volume phase never delivered`（l1_real_test_plugin_e2e.rs:928） |
| 19 | a | clap | `real_test_plugin_l1_instrument_transport_clicks` | 101 | `click phases barely delivered — [0, 3, 1, 2]`（同:1046） |
| 21 | b | vst3 | `real_test_plugin_l1_instrument_cc_and_offset_gating` | 101 | `the full-volume phase never delivered`（同:965） |

## 解釈

- いずれも content assert の失敗ではなく、matched-only 計上に
  よる `verified >= 2` 最低到達 gate の割れ。
- 原因: matched ⇒ `source = call − 2` の厳密帰属があるのに、
  window（cc: calls 12..=15 の 4 call、transport: phase 毎
  ~5 call）だけを assert 対象にしていたため、substitution
  burst 下で matched sample が gate を下回った。
- 対応: spec が許容した exact source-phase attribution へ pivot
  — cc は source phase で silence/sounding を assert（run を
  16→22 call に延長）、transport は 12 call/phase ×4 = 48 call
  で source phase 毎の click/silence を assert。gate 値・
  assert 内容は不変、assert 対象は全 matched block に拡大。
- 結果: 同一 `loop3.sh` 40/40 green（`logs3/`・`logs4/`）。
