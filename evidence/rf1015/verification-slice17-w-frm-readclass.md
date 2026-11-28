# rf#1015 slice 17 verification — Wire `W-FRM-READCLASS` read classification

## 対象と SHA の役割

- branch: `feat/1015-wire-archive-proof`。
- base SHA: `7f2053866e47399e6a86a7368b6d167fcd9654f1`（着手時・最終検証前の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `40ff06c4fc7f6426dbf924393ecb35e57b8f38b5`。
  partial-prefix EOF 分類修正、production read-classification core、in-src proof、
  deterministic regression、`w-frm-readclass` selective lane を含む最後の製品/proof/mise 変更 commitである。
- evidence sync commit: 本書・ledger・spec・skill・`runs.jsonl` を含む後続の docs/skill/evidence-only commit。
  最終 head SHAは PR 本文で追跡する。
- 検証はすべてローカル検証であり、merge 後 main SHA の統合検証ではない。

## 実装した再設計

`read_frame` は製品 rustdocどおり、length prefix の 1 byte目を読む前に終わった clean EOFだけを
`FrameRead::Eof` とする。1〜3 byteの partial length prefixは frame途中の EOFなので
`io::ErrorKind::UnexpectedEof` とする。complete prefix後の body truncationは従来どおり
`UnexpectedEof`、complete bodyの invalid UTF-8は従来どおり `InvalidData` とする。

prefix shellは進捗を保持して 0〜4 byteを読む。最終分類は crate-private な
`frame_read_classification(prefix_len, prefix, body_complete, body_utf8_valid)` が返す。
この coreは allocation、I/O、`Read`/`Write`、Vec growth、UTF-8 validation、environment access、
診断構築を持たない。UTF-8 correctnessそのものは Rust標準ライブラリの責務とし、proofは
classifierが valid/invalid UTF-8を `Frame`/`InvalidData` へ一意に対応づけることだけを検証する。

public signature、`FrameRead` variant、`MAX_INTENT_BYTES`、write cap、wire byte format、
`write_frame`、TooLarge時の drain/stream alignment、body errorの error kindは変えていない。
partial prefixの `UnexpectedEof` には新しい error objectが必要だが、旧実装ではこの経路が errorを返さず
`Eof` だったため、変更済みの stable message契約はない。

proofは production implementationと同じ `src/transport.rs` の `#[cfg(kani)] mod proofs` にある。
proof placementのための cfgであり、製品挙動を置き換えていない。modelや test doubleは対象にしていない。

| harness | 対象と bound | 確認した領域 | 未証明領域 |
| --- | --- | --- | --- |
| `w_frm_readclass_fact_exactness` | symbolic `usize` prefix read length、symbolic `[u8;4]` prefix、symbolic body complete/UTF-8 validity bool、`#[kani::unwind(2)]`、assumeなし | production classifierと独立 truth-table oracleの exactness。coverは prefix 0/1/2/3、truncated body、invalid UTF-8、valid in-cap frame、cap+1 TooLargeの 8/8 satisfied | shellが factsを作る接続、実 I/O、UTF-8 validation本体、drain、Vec growth |
| `w_frm_readclass_window_exactness_len32` | stream length 0〜32、symbolic `[u8;4]` prefix、symbolic UTF-8 validity bool。`stream_len <= 32` は入力域分割の `assume`。`#[kani::unwind(2)]` | byte window abstractionで prefix 0/1/2/3、complete prefix、declared bodyに対する short/exact/extra byte、invalid UTF-8、in-cap/TooLargeの代表到達。cover 9/9 satisfied。extra byteは当該 frameの errorにしない | 32 byte超の window、実 body byteの UTF-8 validation、cursor/drainの実進行、shell-core接続 |

`stream_len <= 32` は production reachable domainの先頭 windowを表す入力域分割であり、充足可能である。
coverは要求した分岐へ到達した。32 byte超、実 body byte transport、oversize drainの実 byte転送、
malicious infinite reader、実 OS pipe/TCP、shell-core接続は証明済みにしない。

Unwinding Assertionsは無効化していない。`--no-unwinding-checks`、verified/unverified stub、
`cfg(kani)` による製品実装置換、反例を消す目的の assume/bound縮小は使っていない。

旧 legacy `read_frame_contract` harnessは削除した。rf806では Vec/Cursor/UTF-8/I/O shellを含めて
約44 CPU-min・5.9 GiBで verdictを返せなかった。新保証と旧 harnessを二重計上しない。
`W-FRM-OVERSIZE` の drain必要量や実 byte転送はこの sliceの対象外である。

## shell-core接続と proptest oracle

`formal_transport.rs` の oracleを製品 rustdoc契約へ更新した。empty inputだけが `Eof`、
1〜3 byteの partial prefixは `UnexpectedEof`、complete prefix後の body truncationは
`UnexpectedEof`、complete bodyの invalid UTF-8は `InvalidData` とする。

deterministic `read_frame_classification_connection_regression` は prefix 0/1/2/3 byte、
body truncation、invalid UTF-8、連続した2 frameの extra byteが次 frameの入力として残ること、
TooLarge drain後の整列を直接確認する。これらの接続条件は Kani未証明であり、proof Passに含めない。

## 環境

- canonical lane runner OS: Windows 11 x86_64（10.0.26300.9974）。
- Kani execution environment: WSL2 Ubuntu 24.04.4 LTS x86_64、kernel `6.6.87.2-microsoft-standard-WSL2`。
- Windows側 tool: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`、mise 2026.11.2 windows-x64。
- WSL2側 tool: `rustc 1.99.0 (b940084d7 2026-09-28)`、Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdictは mise lane。direct `cargo kani` と `/usr/bin/time` は補助証拠（canonical = mise）である。

## 実行した正規検証

UTCは 2026-11-28。wallは mise lane全体である。

| lane | command | 時刻 | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani w-frm-readclass` | 06:49:03–06:49:30 | 0 | Pass | 2/2 harness successful、cover 9/9と8/8 |
| selective Kani mutation | `mise run test-formal-kani w-frm-readclass` | 07:02:30–07:02:59 | 1 | Fail（検出成功） | 両 harnessが semantic assertion failure |
| fmt | `mise run fmt-check` | 06:50:50–06:51:02 | 0 | Pass | rustfmt diffなし |
| check | `mise run check ruviform-ui-test` | 06:51:03–06:51:05 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-ui-test` | 06:51:05–06:51:13 | 0 | Pass | clippy `-D warnings` pass |
| crate regression pre-evidence | `mise run test ruviform-ui-test` | 06:51:13–06:52:00 | 0 | Pass | `formal_transport` 6/6を含む |
| crate regression post-revert final tree | `mise run test ruviform-ui-test` | 07:43:59–07:45:12 | 0 | Pass | mutation完全revert後の final tree（`767c5228ad63cf85db0b8796e9cc47b3fff6f240`）で `formal_transport` 6/6を含む全対象 pass |
| proptest post-revert | `mise run test-formal` | 06:52:00–06:52:24 | 0 | Pass | `formal_transport` 6/6を含む全 lane pass |
| selective Kani post-revert | `mise run test-formal-kani w-frm-readclass` | 07:03:09–07:03:27 | 0 | Pass | 2/2 harness successful、cover 9/9と8/8 |
| existing proof non-regression | `mise run test-formal-kani w-frm-codec` | 06:52:43–06:52:57 | 0 | Pass | 3/3 harness successful、各 cover 4/4 |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 06:52:57 | 2 | Pass（期待どおり usage error） | Kani実行前に停止 |

## selective Kani とコスト

canonical commandは `mise run test-formal-kani w-frm-readclass` であり、WSL2内では
`cargo kani -j1 --output-format terse -p ruviform-ui-test --harness w_frm_readclass` を実行した。
出力では次の 2 harnessだけが checking対象になり、退役した `read_frame_contract`、
`w_frm_codec_*`、他 crateの proofを巻き込まなかった。

- `transport::proofs::w_frm_readclass_window_exactness_len32`
- `transport::proofs::w_frm_readclass_fact_exactness`

post-revert runはすべて `SUCCESSFUL`、0 failed、cover 9/9と8/8 satisfiedであり、Unwinding Assertionsを
無効化していない。verification timeは window 0.43533978 s、fact 0.39517504 s、合計 0.83051482 s、
canonical lane wall clockは約19 sだった。

補助計測（canonical = mise）では、post-revert後の direct `cargo kani` に対する `/usr/bin/time -v` が
exit 0、wall clock 8.45 s、user CPU 1.51 s + system CPU 0.97 s、max RSS 105,620 KiBだった。
compileを含む補助 wall clockと Kaniの verification timeは混同しない。事前予算 12 CPU-min / 3 GiBに対し、
verification time・CPU時間・観測メモリのいずれも十分以内である。dominant costは symbolic `[u8;4]` prefixと
stream length 0〜32の bit-vector比較であり、旧 shell proofの Vec/Read/Write/UTF-8 loop展開ではない。

## Mutation-fail と revert

実施した mutantは `frame_read_classification` の partial prefix分類を
`FrameReadClassification::UnexpectedEof` から `FrameReadClassification::Eof` へ変える 1箇所だけである。
制御した mutation runを 2026-11-28T07:02:30Z–07:02:59Z に再取得した。transient mutantは commitしていない。

- mutation run: `mise run test-formal-kani w-frm-readclass`、2026-11-28T07:02:30Z–07:02:59Z、exit 1。
- 検出: `w_frm_readclass_window_exactness_len32` と `w_frm_readclass_fact_exactness` の両方で
  `assertion failed: actual == expected`。unwinding assertion failureではなく semantic violationである。
  coverはそれぞれ 6/9、5/8 satisfiedだった。
- revert: 該当行を `UnexpectedEof` へ戻した。revert後の製品/proof/mise差分は verified Sync Commitに含まれる。
- revert後再検証: selective Kaniが 07:03:09Z–07:03:27Zに exit 0 / pass（verification time 0.952017 s / 0.3605135 s）、`test-formal` も revert後に再実行して exit 0 / pass。さらに mutation完全revert後の final tree `767c5228ad63cf85db0b8796e9cc47b3fff6f240` で `mise run test ruviform-ui-test` を 07:43:59Z–07:45:12Zに実行し、exit 0 / pass（`formal_transport` 6/6を含む）を確認した。

mutationの failは lane regressionではなく検出成功として記録する。mutation実験の副産物は残していない。

## consumer impact assessment

2026-11-28に最新 checkoutで crate referenceと exact symbol利用を分けて検索した。
audio-lens HEADは `886828500b85452774af2be7114437ebb226976b`、synvelis HEADは
`3a7e3ed9b17aebb236acde9bdce125abc721cbe0` だった。

- audio-lens: `ruviform-ui-test` の manifest/lockfile/scenario基盤参照はあるが、
  `ruviform_ui_test::transport`、`read_frame`、`write_frame`、`FrameRead`、`MAX_INTENT_BYTES` の
  direct useは見つからなかった。
- synvelis: `crates/synvelis-app/src/uitest.rs` の `#[cfg(test)]` で
  `ruviform_ui_test::transport::{FrameRead, read_frame, write_frame}` を直接利用する。
  `read_all_frames` は complete frame列の終端で `FrameRead::Eof` を期待する正常系であり、partial prefixを
  作らない。`crates/mcp-poc/src/main.rs` の `read_frame` は別関数なので混同しない。
- 両 repoとも `MAX_INTENT_BYTES` の direct useは見つからなかった。

今回は observable error classificationを変えるため、consumer regressionを NotApplicableにしない。
最終 PR head SHAに対する `consumer-verify.yml` candidate runを取得し、run URL、candidate ruviform SHA、
両 consumer resolved SHA、lane verdict、NotRun/fail行と理由を PR本文へ記録する。evidenceは自分自身の
commit hashを固定点として書けないため、最終 runの詳細は PR本文と `runs.jsonl` の固定点外記録で追跡する。
consumer repoのコード変更、pin更新、Issue起票はこの sliceでは行わない。

## 手動確認

- clean EOF/partial prefix/body truncation/invalid UTF-8/TooLarge drain後整列、write側非変更、
  extra byteが次 frame入力として残ることを deterministic regressionで確認した。
- selective filterが 2件の `w_frm_readclass_*` だけを選ぶことを lane出力で確認した。
- native/WSL2両分岐に同じ selector semanticsを追加した。selectorなし aggregate、WSL target dir、
  WAV `-j1`、既存 selectorの意味は変えていない。
- `Cargo.lock`に差分はない。docs index（`docs/**/README.md`）は編集していない。
- mutation副産物や transient mutantは残していない。

## sync state

| 対象 | state | 根拠 |
| --- | --- | --- |
| `W-FRM-READCLASS` ledger row | `synced` | assurance kind、target、入力域、bound、assume、未証明領域、旧保証差、verdict、選択再実行、mutation、実測コストを更新 |
| `docs/specs/formal-verification.md` | `synced` | 新 proof境界、selective entry、旧 harness退役、未証明領域、mutation、verified Sync Commitを同期 |
| `.agents/skills/verify-ruviform/SKILL.md` | `synced` | Coordinatorの lease拡張後に selective entry、in-src proof placement、filter、mutation責務だけを同期。skill全体の再設計はしていない |
| `mise.toml` | `synced` | `w-frm-readclass` selective entryだけを追加し、aggregate laneと既存 selectorを維持 |
| 他 Property group | `design_only` / `not_synced` / `deferred` | この sliceの成果として昇格していない |
| `evidence/rf806/` | `synced`（read-only参照） | 改変していない |
| docs index | `not_synced`（許容） | dispatch契約どおり編集・再生成していない。Coordinatorが統合時に整備する |

## 未実施 lane / 後続

- selectorなし `mise run test-formal-kani`: NotRun。verdictを返せない既知legacy harnessを含む default aggregateを
  本 sliceの Required evidenceにしない。
- Workspace full lane、Feature Matrix、MSRV、3OS横断、GUI-L2/EXT-L3: NotRun。本 sliceの指定外であり、
  main統合検証は merge後の対象SHAに対するCI証拠である。
- 残件: `W-IPC-SERDE`、`W-FRM-OVERSIZE` の独立proof、frame body transport shell、実 OS pipe/TCP、
  malicious infinite reader、Archive残部の上位合成、WAV diagnostic boundary、SampleFormat inventory、
  Issue closeと Issue全体ACの最終判定。

## 残る未証明領域とリスク

- shellが coreを呼ぶ接続条件は Kani未証明。deterministic regressionと既存 proptestに依存する。
- UTF-8 validation本体、frame body bytesの輸送、oversize drain、Vec/Read/Write shell、実 OS pipe/TCP、
  malicious infinite readerは proof boundaryに入れていない。
- consumer-verify candidate runの結果は PR本文で最終追跡する。Ubuntu中心の candidate evidenceであり、
  3OS/main統合検証の代用ではない。
