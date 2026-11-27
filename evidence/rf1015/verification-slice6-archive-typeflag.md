# rf#1015 slice 6 verification — Archive `ARC-TAR-STRUCT`

## 対象と SHA の役割

- base SHA: `f6f77fc05799d4af960c35d6d82eccd758e5c370`（branch 開始時の `origin/main`）。
- 検証対象 SHA / verified Sync Commit: `9729366a37ebf10174132f829c5cdf07b0c2c980`。
  TAR typeflag decision core、in-src proof、旧 parser-facing harness 退役、
  `arc-tar-struct` selective lane を含む最後の製品/proof/mise 変更 commit である。
- evidence sync commit: 本書・ledger・spec・skill・`runs.jsonl` を含む後続の
  docs/evidence-only commit。evidence は自分自身を含む commit hash を固定点として
  書けないため、最終 head SHA は PR 本文で追跡する。

## 実装した再設計

`TarArchiveReader` は header typeflag byte だけを受け取る crate-private な
`tar_typeflag_class` を entry dispatch の前に呼び、local/global PAX、GNU
longname/longlink、surface kind、unsupported を一意に分類する。surface 側では
`TarSurfacedKind` から link target を読むかどうかと公開 `EntryKind` を決める。
core は allocation、I/O、`Read`、TAR header parser、checksum 計算、PAX/GNU
extension payload、Vec buffering、診断構築を持たない。公開 API、default 値、
TAR header/PAX/GNU/extension/receipt/padding/streaming の observable behavior は
変更していない。global PAX は従来どおり grammar-validated だが effective fields
へ適用しない。

新しい proof は production core と同じ `src/tar/read.rs` の
`#[cfg(kani)] mod proofs` にある `arc_tar_struct_typeflag_exactness` である。
typeflag byte を symbolic にし、assume なし、`#[kani::unwind(2)]`、Unwinding
Assertions 有効で、production core を independent oracle と比較して全 256 値の
exactness を検証する。cover property は 0/`0`/`7`/`5`/`1`/`2`/`x`/`X`/`g`/`L`/`K`/
unsupported 代表の 12 経路到達を確認する。`cfg(kani)` は proof placement のため
だけであり、製品実装を置き換えていない。

parser-core 接続は `tar_typeflag_connection_regression` が担保する。raw typeflag
byte が observable entry kind/link target へ届くこと、local PAX と GNU extension
fold が内部で処理されること、global PAX が effective fields を上書きしないこと、
未知 typeflag が `ArchiveErrorKind::Unsupported` になることを確認する。

旧 parser-facing `tar_typeflag_classification` harness は残さず退役した。これは
失敗隠しではなく、concrete TAR skeleton と header parser shell を含む verdict
未発行 proof を、pure decision core exactness proof へ redesign したものである。
rf806 では約15 CPU-min で約21.8 GiBまで膨張して verdict を返せなかった。この
経緯は履歴として残し、新保証が parser-core 接続と extension folding を証明しない
保証差を明示する。

## 環境

- canonical lane runner OS: Windows 11 x86_64。
- Kani execution environment: WSL2 Ubuntu。
- Kani Rust Verifier 0.68.0、CBMC 6.11.0。
- canonical verdict は mise lane。直接 `cargo kani` と `/usr/bin/time` は補助証拠
  （canonical = mise）である。

## 実行した正規検証

| lane | command | 時刻（UTC） | exit | verdict | 記録 |
| --- | --- | --- | ---: | --- | --- |
| selective Kani pre-mutation | `mise run test-formal-kani arc-tar-struct` | 04:27:50–04:27:58 | 0 | Pass | 1/1 harness、0/83 failed、cover 12/12、verification time 0.22770266 s |
| selective Kani mutation | `mise run test-formal-kani arc-tar-struct` | 04:28:18–04:28:24 | 1 | Fail（検出成功） | mutant で 1/83 failed、cover 11/12、verification time 0.19747403 s |
| selective Kani post-revert | `mise run test-formal-kani arc-tar-struct` | 04:28:36–04:28:43 | 0 | Pass | 1/1 harness、0/83 failed、cover 12/12、verification time 0.18786699 s |
| proptest post-revert | `mise run test-formal` | 04:28:56–04:29:37 | 0 | Pass | `formal_ipc` 7/7、`formal_transport` 4/4、`formal_archive` 19/19、`formal_structured` 35/35、`formal_wav` 9/9 |
| usage negative smoke | `mise run test-formal-kani invalid-selector` | 04:30:11–04:30:12 | 2 | Pass（期待どおり usage error） | unknown selector は environment detection/Kani 実行前に停止 |
| fmt | `mise run fmt-check` | 04:33:44–04:33:56 | 0 | Pass | rustfmt diff なし |
| check | `mise run check ruviform-io-archive` | 04:33:56–04:33:58 | 0 | Pass | crate check pass |
| lint | `mise run lint ruviform-io-archive` | 04:33:58–04:34:00 | 0 | Pass | clippy `-D warnings` pass |
| crate regression | `mise run test ruviform-io-archive` | 04:34:00–04:34:04 | 0 | Pass | unit/integration/formal archive/doctest pass。`formal_archive` 19/19 を含む |

`mise run test ruviform-io-archive` では parser-core 接続 regression
`tar_typeflag_connection_regression` を含む `formal_archive` 19/19 が pass した。
coverage 不足は確認されず、mutation 副産物は残していない。

## selective Kani とコスト

canonical command は `mise run test-formal-kani arc-tar-struct` であり、WSL2 内では
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_tar_struct`
を実行した。出力では `tar::read::proofs::arc_tar_struct_typeflag_exactness` だけが
checking 対象になり、同じファイル内の `arc_payload_limit_declared_exactness` など他
proof を巻き込まなかった。post-revert run は `SUCCESSFUL`、0/83 failed、cover
12/12 satisfied であり、Unwinding Assertions を無効化していない。

canonical lane は pre-mutation run が verification time 0.22770266 s / wall 約8 s、
post-revert run が verification time 0.18786699 s / wall 約7 sだった。補助計測
（canonical = mise）では、post-revert後の direct `cargo kani` に対する
`/usr/bin/time -v` が exit 0、wall clock 3.55 s、user CPU 0.57 s + system CPU
0.51 s、max RSS 106,148 KiBだった。compile を含む補助 wall clock と Kani の
verification time は混同しない。事前予算 5 CPU-min / 1 GiB に対し、verification
time・CPU 時間・観測メモリのいずれも十分以内である。完了時間や exit 0 を Proof
Pass の根拠にはせず、`SUCCESSFUL` verdict、失敗数、cover 到達を証拠にした。

## Mutation-fail と revert

実施した mutant は decision core の `TYPE_SYMLINK` 分類を
`TarSurfacedKind::SymbolicLink` から `TarSurfacedKind::File` へ変える 1 箇所だけ
である。transient mutant は commitしていない。

- mutation run: `mise run test-formal-kani arc-tar-struct`、
  2026-11-27T04:28:18Z–04:28:24Z、exit 1。
- 検出: `arc_tar_struct_typeflag_exactness` が `1 of 83 failed`。失敗は production
  core と independent oracle の exactness semantic assertion であり、unwinding
  assertion failure ではない。verification time 0.19747403 s。
- revert: 該当行を `TarSurfacedKind::SymbolicLink` へ戻し、`rg` で
  `TYPE_SYMLINK => TarTypeflagClass::Surface(TarSurfacedKind::SymbolicLink)`
  を確認した。mutant を示す未 revert diff はない。
- revert後再検証: selective Kani が 04:28:36Z–04:28:43Z に exit 0 / pass、
  `test-formal` が 04:28:56Z–04:29:37Z に exit 0 / pass。

## consumer impact assessment

2026-11-27T04:36:12Z に audio-lens/synvelis の `crates/`、`Cargo.toml`、
`Cargo.lock` を対象に `TarArchiveReader`、`ArchiveReadLimits`、
`ruviform-io-archive`、`ruviform_io_archive` を検索し、両 repo で `rg` exit 1
（no matches）だった。本変更は crate-internal な decision core 抽出であり、公開
API・error kind・observable behavior を変えないため、consumer 独立回帰は
NotApplicable と判断する。この判断は Issue 全体の consumer 回帰 AC を完了扱いに
しない。

## 手動確認

- selective filter が `arc_tar_struct_typeflag_exactness` だけを選ぶことを lane
  出力で確認した。
- `w-ipc-op` / `arc-idx-limit` / `arc-payload-limit` / `sd-csv-limits` /
  `sd-byte-budget` selector の command/filter は変更していない。native/WSL2 の両分岐に
  同じ selector semantics を追加したことを diff で確認した。
- selector なし aggregate invocation、wire/archive/WAV/structured invocation、target
  dir 方針は変更していない。
- 0 と `0` の両方が File、`7` が File、`x`/`X` が local PAX、`g` が global PAX、
  `L`/`K` が GNU extension、未知値が Unsupported であることを proof oracleと cover
  で確認した。
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
  EOF、SampleFormat inventory、consumer 回帰の統合判断。

## 残る未証明領域とリスク

- parser が core を呼ぶ接続条件は Kani 未証明。connection regression と既存
  proptest/integration coverage に依存する。
- TAR header parse、checksum 計算、PAX/GNU extension folding、複数 header、任意
  malformed stream、数値 field 全体、payload streaming、receipt accounting は proof
  boundary に入れていない。
- 上記を Pass に含めず、ledger/spec/skill へ同期した。
