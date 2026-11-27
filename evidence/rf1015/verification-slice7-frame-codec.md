# rf#1015 slice 7 verification — Wire `W-FRM-CODEC` length-prefix/write-cap

## 対象と SHA の役割

- branch: `test/1015-frame-codec-proof`。
- 作業開始時の `origin/main`: `823245f57d130436c1f51824f601af13e22006ef`。
- 最終検証前に取り込んだ base SHA: `84abc76f4674ba587482197d314005054d7bda6f`。
  rf#1040 の merge で main が進んだため、branch はこの SHA へ rebase した。
- 検証対象 SHA / verified Sync Commit: `82d260dfa0b98d9cf325dc1b085090f2bb2c6d0f`。
  frame length-prefix/write-cap decision core、in-src proof、旧 shell roundtrip harness 退役、
  `w-frm-codec` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit: 本書・ledger・spec・skill・`runs.jsonl` を含む後続の
  docs/evidence-only commit。evidence は自分自身を含む commit hash を固定点として
  書けないため、最終 head SHA は PR 本文で追跡する。
- 検証はすべてローカル検証であり、merge 後 main SHA の統合検証ではない。

## 実装した再設計

`write_frame` と `read_frame` は crate-private な length-prefix/write-cap decision core
を使う。write 側は body length から accept/reject と accepted length を計算し、
accept 時だけ u32le length prefix を encode する。read 側は complete な 4 byte prefix から
declared length と in-cap/TooLarge を計算する。cap 判定は共有し、write/read に重複させない。

core は allocation、I/O、`Read`/`Write`、Vec growth、UTF-8 validation、environment access、
診断構築を持たない。公開 API、`MAX_INTENT_BYTES`、error kind、zero/short prefix の EOF 分類、
body truncation、UTF-8 error、oversize drain、flush、transport selection、error message の
observable behavior は変えていない。D-4 の partial-prefix EOF 契約も変更していない。

proof は production core と同じ `src/transport.rs` の `#[cfg(kani)] mod proofs` にある。
`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

| harness | 対象と bound | 確認した領域 | 未証明領域 |
| --- | --- | --- | --- |
| `w_frm_codec_write_length_exactness` | symbolic `usize` body length、`#[kani::unwind(2)]`、assumeなし | accept/reject、accepted length、u32le prefix byte exactness。coverは len 0/cap-1/cap/cap+1 の 4/4 satisfied | body byte transport、write shell接続、UTF-8 validation、実 writer |
| `w_frm_codec_prefix_exactness` | symbolic `[u8; 4]` prefix、`#[kani::unwind(2)]`、assumeなし | declared length decode、in-cap/TooLarge exactness。coverは len 0/cap-1/cap/cap+1 の 4/4 satisfied | EOF/truncation 分類、oversize drainの実 I/O、UTF-8 validation |
| `w_frm_codec_length_roundtrip` | symbolic `usize` body length、`#[kani::unwind(2)]`、assumeなし | accepted lengthの encode→decode roundtrip。coverは len 0/cap-1/cap/cap+1 の 4/4 satisfied | body bytesそのものの roundtrip、Vec/Read/Write shell、実 OS transport |

Unwinding Assertions は無効化していない。`--no-unwinding-checks`、verified/unverified stub、
`cfg(kani)` による製品実装置換、入力域を縮める assume は使っていない。

旧 shell 向け `write_read_roundtrip` harness は残さず退役した。これは失敗隠しではなく、
16 byte bodyを Vec/Cursor/UTF-8 shellごと検査しようとして verdict を返せなかった proofを、
length-prefix/write-cap exactness proofへ redesign したものである。rf806 では約90 CPU-min・
5.4 GiBで verdict を返せなかった。この経緯は履歴として残し、body byte transportと I/O shellを
新保証に含めない。`read_frame_contract` は `W-FRM-READCLASS` / `W-FRM-OVERSIZE` の legacy
状態を表す harnessなので、この slice では削除・修正・証明済み扱いをしていない。

shell-core接続は deterministic `frame_length_boundary_connection_regression` を追加して確認する。
exact-cap write/read acceptance、cap+1 pre-write rejection、u32le byte order、read-side TooLarge
classificationを直接確認する。既存 proptest `write_read_roundtrip` と
`write_frame_over_cap_refused` も維持した。接続条件は Kani 未証明である。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu 24.04.4 LTS x86_64、kernel `6.6.87.2-microsoft-standard-WSL2`。
- Windows 側 tool: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`、mise 2026.11.2 windows-x64。
- WSL2 側 tool: `rustc 1.99.0 (b940084d7 2026-09-28)`、Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani w-frm-codec` | 05:57:14–05:57:31 | 0 | Pass | 3/3 harness successful、各 cover 4/4。verification timeは roundtrip 0.11586958 s、prefix 0.12648582 s、write 0.22829914 s |
| selective Kani mutation | `mise run test-formal-kani w-frm-codec` | 05:57:42–05:57:57 | 1 | Fail（検出成功） | mutant で 3/3 harnessが semantic assertion failure、各 cover 3/4 |
| selective Kani post-revert | `mise run test-formal-kani w-frm-codec` | 05:58:09–05:58:25 | 0 | Pass | 3/3 harness successful、各 cover 4/4。verification timeは roundtrip 0.08742989 s、prefix 0.13833326 s、write 0.26267588 s |
| proptest post-revert | `mise run test-formal` | 05:58:49–05:59:21 | 0 | Pass | `formal_ipc` 7/7、`formal_transport` 5/5、`formal_archive` 19/19、`formal_structured` 35/35、`formal_wav` 9/9。追加した deterministic regressionを含む |
| fmt | `mise run fmt-check` | 06:00:02–06:00:13 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-ui-test` | 06:00:13–06:00:17 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-ui-test` | 06:00:17–06:00:22 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-ui-test` | 06:00:22–06:00:53 | 0 | Pass | unit/integration/formal transport/doctest pass。`formal_transport` 5/5 を含む |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 06:00:53–06:00:53 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |

## selective Kani とコスト

canonical command は `mise run test-formal-kani w-frm-codec` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-ui-test --harness w_frm_codec`
を実行した。出力では次の 3 harnessだけが checking 対象になり、legacy `read_frame_contract`
や他 crate の proofを巻き込まなかった。

- `transport::proofs::w_frm_codec_length_roundtrip`
- `transport::proofs::w_frm_codec_prefix_exactness`
- `transport::proofs::w_frm_codec_write_length_exactness`

post-revert run はすべて `SUCCESSFUL`、0 failed、cover 4/4 satisfied であり、
Unwinding Assertions を無効化していない。verification time 合計は 0.48843903 s、
canonical lane wall clock は約16 sだった。

補助計測（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 9.76 s、user CPU 1.96 s + system CPU 1.06 s、
max RSS 105,292 KiBだった。compile を含む補助 wall clock と Kani の verification time は
混同しない。事前予算 5 CPU-min / 1 GiB に対し、verification time・CPU 時間・観測メモリの
いずれも十分以内である。支配要因は symbolic `usize` の bit-vector 比較であり、
旧 shell proofの Vec/Read/Write/UTF-8 loop展開ではない。完了時間や exit 0 を Proof Pass の
根拠にはせず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の cap 判定 `len <= MAX_INTENT_BYTES` を
`len < MAX_INTENT_BYTES` へ変える 1 箇所だけである。transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani w-frm-codec`、2026-11-27T05:57:42Z–05:57:57Z、exit 1。
- 検出: 3 harnessとも exact-cap acceptanceに関する semantic assertion failureになった。
  roundtripは `body_len > MAX_INTENT_BYTES`、prefixは `declared as usize > MAX_INTENT_BYTES`、
  writeは `write length decision mismatch` が失敗した。unwinding assertion failureではない。
  各 harnessは 1 failed、cover 3/4 satisfiedだった。
- revert: 該当行を `len <= MAX_INTENT_BYTES` へ戻し、`git diff --exit-code -- crates/ruviform-ui-test/src/transport.rs` が差分なしを返した。
- revert後再検証: selective Kani が 05:58:09Z–05:58:25Z に exit 0 / pass、
  `test-formal` が 05:58:49Z–05:59:21Z に exit 0 / pass。

mutation の fail は lane の Fail ではなく「検出成功」として記録する。mutation 実験の副産物や
不要な regression file は残していない。

## consumer impact assessment

2026-11-27T05:59:35Z–05:59:36Z に audio-lens/synvelis の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に `ruviform-ui-test`、`ruviform_ui_test::transport`、`write_frame`、
`read_frame`、`FrameRead`、`MAX_INTENT_BYTES` を検索した。

audio-lens では `ruviform-ui-test` の依存と scenario 基盤への言及はあるが、
transport symbol の直接利用は見つからなかった。synvelis では
`crates/synvelis-app/src/uitest.rs` の `#[cfg(test)]` で `write_frame`、`read_frame`、
`FrameRead` を使う。`mcp-poc` の `read_frame` は別関数である。公開 API、limit、error behavior、
frame byte formatを変えていないため、consumer 独立回帰はこの slice では NotApplicable と判断する。
この判断は Issue 全体の consumer 回帰 AC を完了扱いにしない。

## 手動確認

- selective filter が 3 件の `w_frm_codec_*` だけを選ぶことを lane 出力で確認した。
- native/WSL2 の両分岐に同じ selector semantics を追加したことを `mise.toml` diff で確認した。
- selector なし aggregate invocation、wire/archive/WAV/structured invocation、WSL target dir 強制、
  WAV `-j1`、既存 selector の command/filter は変更していない。
- exact-cap acceptance、cap+1 pre-write rejection、u32le byte order、read-side TooLarge、
  EOF/truncation/UTF-8/drain/flush/transport selection の observable behavior を変えていないことを
  既存 test と deterministic regressionで確認した。D-4 の partial-prefix EOF は変更していない。
- `Cargo.lock` に差分はない。
- docs index（`docs/**/README.md`）は編集していない。統合時に Coordinator が整備する。
- mutation 実験の副産物や不要な regression file は残していない。

## sync state

| 対象 | state | 根拠 |
| --- | --- | --- |
| `W-FRM-CODEC` ledger row | `synced` | assurance kind、target、入力域、bound、assume、未証明領域、旧保証差、verdict、選択再実行、mutation、実測コストを更新 |
| `docs/specs/formal-verification.md` | `synced` | 新 proof 境界、selective entry、旧 shell harness の退役、I/O shell/body transportの未証明領域、mutation 結果を同期 |
| `.agents/skills/verify-ruviform/SKILL.md` | `synced` | selective entry、in-src proof placement、filter、mutation 責務を同期。英語中心の現行スタイルを維持 |
| `mise.toml` | `synced` | `w-frm-codec` selective entryだけを追加し、aggregate laneと既存 selectorを維持 |
| 他 Property group | `design_only` / `not_synced` / `deferred` | この slice の成果として昇格していない |
| `evidence/rf806/` | `synced`（read-only 参照） | 改変していない |
| docs index | `not_synced`（許容） | dispatch 契約どおり編集・再生成していない。Coordinator が統合時に整備する |

## 未実施 lane / 後続

- selector なし `mise run test-formal-kani`: NotRun。verdict を返せない既知の wire/archive harnessを
  含む default aggregate を本 slice の Required evidence にしない。
- Workspace full lane、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI dispatch:
  NotApplicable/NotRun。本 slice の指定外であり、merge 後 main SHA の統合検証は Daily CI が担う。
- consumer repo test run: NotApplicable。audio-lens は transport symbol直接利用なし、synvelis は
  test-only利用であり、public behavior unchanged。
- 残件: `W-FRM-CODEC` の body byte transport shell、`W-FRM-READCLASS`、`W-FRM-OVERSIZE`、
  `W-IPC-ENVELOPE`、`W-IPC-SERDE`、Archive残部、WAV diagnostic boundary、read_frame EOF、
  SampleFormat inventory、consumer 回帰の統合判断。

## 残る未証明領域とリスク

- shell が core を呼ぶ接続条件は Kani 未証明。deterministic regressionと既存 proptestに依存する。
- frame body bytesの輸送、UTF-8 validation、EOF/truncation 分類、oversize drain、Vec/Read/Write shell、
  実 OS pipe/TCP、malicious infinite readerは proof boundaryに入れていない。
- 上記を Pass に含めず、ledger/spec/skill へ同期した。
