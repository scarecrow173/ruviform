# Issue #629 WASAPI streaming verifier — 実装実験記録（2026-11-16）

`docs/tasks/issue-629-wasapi-endpoint.md` の 4th slice（
`tests/virtual_endpoint.rs` 新設 + `endpoint_probe` 拡張 +
Test Runtime 結線の headless 範囲）実装で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-629-wasapi-endpoint.md`](../ir/issue-629-wasapi-endpoint.md)、
イベントログは
[`issue-629-wasapi-endpoint-exp.jsonl`](issue-629-wasapi-endpoint-exp.jsonl)
を参照。

## 試行 1: feature gate と依存境界の確定

- **目的**: verifier の gate 方法と `ruviform-audio-test` との結線
  方針を決める。
- **観測**: `real_input_e2e.rs` の `#![cfg(feature = "real-device")]`
  + `#[ignore]` 先例がそのまま使える。Coordinator 回答 (A) により
  `ruviform-audio-device -> ruviform-audio-test` の dev-dep 追加は
  不承認（2026-11-15 承認済み依存変更リスト外）— oracle は test
  file 内の独立 deterministic generator（`signal_sample`: 440 Hz
  sine + 100 ms 毎の impulse marker burst）で自足させ、#628
  `PcmFixture`/`RunMetadata` との IF は docs に記録して後続
  provisioned-runner slice へ hand-off した。
- **結論**: 新規 feature flag も新規 dependency も追加しない。
  `Cargo.toml`/`Cargo.lock` は無変更。

## 試行 2: pair 選定ヒューリスティックの同期

- **目的**: probe と verifier で「同じ pair candidate」を選ぶ規則を
  一致させる。
- **観測**: example は test から import できないため、
  `VIRTUAL_NAME_HINTS`・`is_virtual_name`・新設の
  `DIRECTION_TOKENS`/`virtual_stem` を両 file に複写し、相互参照の
  sync コメントを付けた。stem は「小文字化 → 英数字以外で分割 →
  方向語（input/output/render/capture/in/out）除去 → 空白 join」で、
  "CABLE Input"↔"CABLE Output" や Voicemeeter の逆方向命名を同一
  stem に正規化する。probe 側は port view（cpal の `Device::name()`）
  をそのまま使うため、verifier が見る候補と表が一致する。
- **結論**: name match はあくまで candidate — `virtual-os-endpoint`
  の証明にも Pass の根拠にもしない（両者の出力文言で明記）。

## 試行 3: oracle の独立性と SRC 耐性

- **目的**: bit-exact を要求せず、仮想 cable 内部 SRC（VB-CABLE 系
  48 kHz）を通っても成立する Expected/Observed 照合を設計する。
- **観測**: 信号は 440 Hz sine（0.20）+ 8 frame 0.70 impulse
  burst/100 ms。sine 単体は検出閾値 0.5 を超えないため、marker
  検出（`|x|>0.5` + 20 ms refractory）は impulse のみを拾う。
  oracle は 3 系統: (a) marker 数 ≥3 かつ平均間隔が 100 ms ±10%、
  (b) 観測 RMS/期待 RMS が [0.2, 3.0]（SRC は unity-gain 仮定）、
  (c) Goertzel による 440 Hz 成分が全 power の 25% 以上 — impulse
  だけの破損経路を弾く。いずれも閾値は provisional で、provisioned
  runner の実測で締める余地を残した。
- **結論**: bit-exact・位相厳密比較は行わない。capture は channel 0
  のみ pre-allocated buffer に `try_lock` 付きで bounded copy
  （溢れ・lock 失敗は counter で可視化、RT 経路で alloc/block/panic
  なし）。

## 試行 4: 初回コンパイル・lint での機械的修正

- **観測**: `cargo check` で 6 件（`f64::from(u64)` 非対応、
  usize/u64 不一致、`&Box<dyn Trait>`→`&dyn Trait` の引数不一致、
  Goertzel の unused assignment）、続く `lint-audio-real-input`
  （`--all-targets --features real-device -D warnings`）で 7 件
  （`field_reassign_with_default`→struct literal 化、
  `map_unwrap_or`、`ignored_unit_patterns`、`redundant_closure`、
  `doc_lazy_continuation`、`needless_raw_string_hashes`、
  `doc_markdown`）。全て機械的に修正し、以降 lint は clean。
- **結論**: pedantic lint 下では `Probe::new` の sentinel 初期化は
  `..Self::default()` 付き struct literal が正解。`#[expect]` は
  cast 系 2 関数のみに限定使用。

## 試行 5: 実機 probe 再実行と verifier の fail-closed 確認

- **観測**: 本ホスト（Windows 11 26300, Audient iD4 + NVIDIA HDA、
  仮想 endpoint 未 provisioning）で probe を再実行 — 65 endpoint
  列挙、port view 5 output/2 input、pair candidates `none —
  NOT_PROVISIONED`。driver signature は `Win32_PnPSignedDriver` で
  1 件観測（"NVIDIA Virtual Audio Device" が 'virtual audio' hint
  に一致、catalog:absent — 情報記録であって仮想 endpoint の証明
  ではない）。permission は `ConsentStore\microphone=Allow`・
  `NonPackaged=Allow`・`DeviceAccess\Global\{guid}=absent`。
  verifier 直接実行は `verdict: NotRun — NOT_PROVISIONED` で exit 0。
- **結論**: 未 provisioning 環境での fail-closed が実機で確認できた。
  Pass 経路（open→stream→oracle→lifecycle）は未実走のまま —
  provisioned runner 待ち。`--nocapture` なしでは libtest が
  passing test の出力を capture するため、evidence 採取時は
  `--nocapture` を付ける運用を evidence/docs に明記した。
