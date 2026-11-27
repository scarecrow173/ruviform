# rf#1015 slice 5 verification — Archive `ARC-PAYLOAD-LIMIT`

## 対象と SHA の役割

- base SHA: `7c26d01c6b228b11652aec12c44855208cab086a`（branch 開始時の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `7a00436aa3ed1e85c11a00b1f05d93a626833a2e`。
  TAR declared-size decision core、in-src proof、旧 parser-facing harness 退役、
  `arc-payload-limit` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit: 本書・ledger・spec・skill・`runs.jsonl` を含む後続の
  docs/evidence-only commit。evidence は自分自身を含む commit hash を固定点として
  書けないため、最終 head SHA は PR 本文で追跡する。

## 実装した再設計

`TarArchiveReader` は PAX size override を fold した effective declared size と
`max_entry_uncompressed_bytes` だけを受け取る crate-private な
`tar_declared_size_gate_decision` を payload byte を読む前に呼ぶ。core は
allocation、I/O、`Read`、TAR header parser、PAX/GNU extension folding、Vec buffering、
診断構築を持たない。判定は従来どおり declared size が limit より大きい場合だけ
reject、等しい場合は accept である。shell は従来どおり同じ
`ArchiveErrorKind::LimitExceeded`、limit name、actual/max を構築し、path/name/link
limit と entries limit に対する判定順も変えていない。公開 API、default 値、TAR
header/PAX/GNU/extension/typeflag/receipt/padding/streaming の observable behavior は
変更していない。

新しい proof は production core と同じ `src/tar/read.rs` の
`#[cfg(kani)] mod proofs` にある `arc_payload_limit_declared_exactness` である。
declared size と limit を symbolic にし、assume なし、`#[kani::unwind(2)]`、
Unwinding Assertions 有効で exactness を検証する。cover property は declared size の
boundary-1/boundary/boundary+1、in-cap acceptance、over-cap rejection を覆う。
`cfg(kani)` は proof placement のためだけであり、製品実装を置き換えていない。

旧 parser-facing `tar_declared_size_gate` harness は残さず退役した。これは失敗隠し
ではなく、concrete TAR skeleton と header parser shell を含む verdict 未発行 proof を、
pure decision core exactness proof へ redesign したものである。rf806 では約19 CPU-min
で約21.8 GiBまで膨張して verdict を返せず、declared-size boundary mutant
`>`→`>=` が proptest に検出されない boundary density gap も残っていた。これらの
経緯は履歴として残し、新保証が parser-core 接続を証明しない保証差を明示する。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠
  （canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani arc-payload-limit` | 03:11:34–03:11:42 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.12518229 s |
| selective Kani mutation | `mise run test-formal-kani arc-payload-limit` | 03:11:53–03:11:59 | 1 | Fail（検出成功） | mutant で 1/16 failed、cover 4/5、verification time 0.06499593 s |
| selective Kani post-revert | `mise run test-formal-kani arc-payload-limit` | 03:12:13–03:12:19 | 0 | Pass | 1/1 harness、0/16 failed、cover 5/5、verification time 0.08520902 s |
| fmt | `mise run fmt-check` | 03:13:09–03:13:20 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 03:13:20–03:13:23 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 03:13:23–03:13:26 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 03:13:26–03:13:39 | 0 | Pass | unit/integration/formal archive/doctest pass |
| proptest post-revert | `mise run test-formal` | 03:13:39–03:14:23 | 0 | Pass | `formal_ipc` 7/7、`formal_transport` 4/4、`formal_archive` 18/18、`formal_structured` 35/35、`formal_wav` 9/9 |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 03:14:36–03:14:37 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |

`mise run test ruviform-io-archive` では parser-core 接続を担保する既存 proptest
`tar_declared_size_gate` を含む `formal_archive` 18/18 が pass した。coverage 不足は
確認されず、新しい regression file や mutation 副産物は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-payload-limit` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_payload_limit`
を実行した。出力では `tar::read::proofs::arc_payload_limit_declared_exactness` だけが
checking 対象になり、他 proof を巻き込まなかった。post-revert run は `SUCCESSFUL`、
0/16 failed、cover 5/5 satisfied であり、Unwinding Assertions を無効化していない。

canonical lane は pre-mutation run が verification time 0.12518229 s / wall 約8 s、
post-revert run が verification time 0.08520902 s / wall 約6 sだった。補助計測
（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 3.29 s、user CPU 0.48 s + system CPU
0.35 s、max RSS 106,480 KiBだった。compile を含む補助 wall clock と Kani の
verification time は混同しない。事前予算 5 CPU-min / 1 GiB に対し、verification
time・CPU 時間・観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof
Pass の根拠にはせず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の declared-size gate 比較
`declared > max_entry_uncompressed_bytes` を `>=` へ変える 1 箇所だけである。
transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-payload-limit`、
  2026-11-27T03:11:53Z–03:11:59Z、exit 1。
- 検出: `arc_payload_limit_declared_exactness` が `1 of 16 failed`。失敗は boundary
  equality で `Accept` を期待する semantic assertion であり、unwinding assertion
  failure ではない。verification time 0.06499593 s。
- revert: 該当行を `>` へ戻し、`rg` で `if declared > max_entry_uncompressed_bytes`
  を確認した。mutant を示す未 revert diff はない。
- revert後再検証: selective Kani が 03:12:13Z–03:12:19Z に exit 0 / pass、
  `test-formal` が 03:13:39Z–03:14:23Z に exit 0 / pass。

## consumer impact assessment

2026-11-27T03:16:07Z に audio-lens/synvelis の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に `TarArchiveReader`、`ArchiveReadLimits`、
`ruviform-io-archive`、`ruviform_io_archive` を検索し、`rg` exit 1（no matches）
だった。本変更は crate-internal な decision core 抽出であり、公開 API・limit 契約・
error kind/limit name/actual/max・observable behavior を変えないため、consumer 独立
回帰は NotApplicable と判断する。この判断は Issue 全体の consumer 回帰 AC を
完了扱いにしない。

## 手動確認

- selective filter が `arc_payload_limit_declared_exactness` だけを選ぶことを lane
  出力で確認した。
- `w-ipc-op` / `arc-idx-limit` / `sd-csv-limits` / `sd-byte-budget` selector の
  command/filter は変更していない。native/WSL2 の両分岐に同じ selector semantics を
  追加したことを diff で確認した。
- selector なし aggregate invocation、wire/archive/WAV/structured invocation、target
  dir 方針は変更していない。
- equality case が accept のままであることを proof の boundary assertion と既存
  proptest `tar_declared_size_gate` で確認した。
- `Cargo.lock` に差分はない。
- docs index（`docs/**/README.md`）は編集していない。統合時に Coordinator が整備する。
- mutation 実験の副産物や不要な regression file は残していない。

## 未実施 lane / 後続

- selector なし `mise run test-formal-kani`: NotRun。verdict を返せない既知の
  wire/archive harness を含む default aggregate を本 slice の Required evidence にしない。
- Workspace full lane、Feature Matrix、MSRV、3OS 横断、GUI-L2/EXT-L3、手動 CI
  dispatch: NotApplicable/NotRun。本 slice の指定外であり、merge 後 main SHA の統合
  検証は Daily CI が担う。
- consumer repo test run: NotApplicable。直接利用なし、public behavior unchanged。
- 残件: `W-FRM-*`、他 wire/archive Property、WAV diagnostic boundary、read_frame
  EOF、SampleFormat inventory、consumer 回帰の統合判断。`ARC-PAYLOAD-LIMIT` のうち
  TAR measured total streaming gate、ZIP entry size gate、decompression output の
  Kani proof も後続である。

## 残る未証明領域とリスク

- parser が core を呼ぶ接続条件は Kani 未証明。既存 proptest/integration coverage
  に依存する。
- TAR header parse、checksum 計算、PAX/GNU override、任意 malformed stream、payload
  streaming、measured total uncompressed budget、ZIP entry size gate、decompression
  correctness は proof boundary に入れていない。
- 上記を Pass に含めず、ledger/spec/skill へ同期した。
