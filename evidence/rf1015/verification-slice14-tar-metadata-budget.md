# rf#1015 slice 14 verification — TAR 拡張 payload の metadata 予算

## 検証対象

`ARC-IDX-LIMIT` のうち、TAR 拡張 payload の確保前判定を検証した。従来の TAR metadata
予算の検証は `tar_metadata_gate` によるヘッダー計上の proptest であり、この判定を対象とする
旧 Kani harness はない。ZIP index の既存 proof は維持した。

| 役割 | SHA / 参照 |
| --- | --- |
| base SHA | `635bfafa1125aefc2f5b23ec5638c9ca6bddf3f6` |
| 検証対象 SHA / verified Sync Commit | `1b0f492efeb12bc0de39347ebef3c5ec6ffc9aea` |
| 初期 evidence sync commit | 本ファイルを含む後続 commit |
| 最終 head SHA | PR 本文で追跡 |

検証対象 SHA は、製品 core、in-src proof、接続回帰、selective lane を含む最後の commit である。
後続の docs/skill/evidence-only commit はこの SHA を更新しない。`runs.jsonl` の変異記録は
変異を含まない検証対象 SHA に、prek 記録は同 SHA 上の evidence-sync 作業ツリーに紐づける。

## Runtime Verification

Windows 11 x86_64（10.0.26300.9974）、rustc/cargo 1.99.1、mise 2026.11.2 で実行した。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt-check` | 0 | Pass |
| `mise run check ruviform-io-archive` | 0 | Pass |
| `mise run lint ruviform-io-archive` | 0 | Pass（`-D warnings`） |
| `mise run test ruviform-io-archive` | 0 | Pass。unit 22、extraction 12/14/17、formal archive 25、TAR reader/writer 44/15、ZIP reader/writer 34/10、doctest 10 |
| `mise run test-formal` | 0 | 変異の復元後に Pass。IPC 8、transport 5、archive 25、structured 35、WAV 9 |
| `mise exec -- prek run --all-files` | 0 | Pass。2026-11-27 17:35:07–17:35:15 UTC、evidence-sync 作業ツリー、自動修正なし |

`tar_metadata_extension_gate_connection_regression` は終端マーカーのない拡張レコードを
`ext_entry` で組み立てた。予算が `512 + payload_len` の場合と、空 payload で予算が 512 の場合は、
次のヘッダー読み取りで `Truncated` になった。予算が `512 + payload_len - 1` の場合は
`LimitExceeded` になり、診断には宣言長が実値、残量が比較上限として表示された。

製品差分の確認では、`read_extension` が判定結果を確保前に使い、受理時だけ計上値を更新することを確認した。
`advance` の BLOCK 計上、拒否時の `Failed` 遷移、payload の読み取り、padding の検査と予算からの除外、
PAX/GNU 拡張の統合、receipt・公開 API・RT 契約に差分はない。`tests/common/`、`Cargo.lock`、docs index は変更していない。

## Formal Verification

Kani は Windows ホスト上の WSL2 Ubuntu 24.04.4 LTS x86_64（kernel
`6.6.87.2-microsoft-standard-WSL2`）で実行した。WSL の rustc/cargo は 1.99.0、
Kani Rust Verifier は 0.68.0、CBMC は 6.11.0。

`mise run test-formal-kani arc-tar-metadata-budget` は filter `arc_tar_metadata_budget` により、
`tar::read::proofs::arc_tar_metadata_budget_decision_exactness` だけを選んだ。
保証種別は Bounded Exhaustive、対象は `read_extension` が呼ぶ `tar_metadata_extension_gate_decision` である。
入力は設定上限、計上済みバイト数、宣言長の全 `u64`。`#[kani::unwind(2)]`、loop・assume・stub・
`cfg(kani)` による製品実装の置換なし。production の到達可能域に限定せず、上限超過済みの場合も含める。

判定 oracle は残量との比較を再実装せず、`u128` の数学和と 0 バイト要求で受理条件を定義する。
残量の 0 飽和、等号の受理、受理時の非オーバーフローと数学和との一致、拒否時の計上値保持を検証した。
8 件の cover は境界の −1/等号/+1、上限超過済みでの 0/1 バイト要求、設定上限 0 での 0 バイト要求、
残量が正でも `u64::MAX` 要求が超過する場合、`u64::MAX` 要求が等号で収まる場合を区別する。
入力 byte stream の長さや trailing bytes、stream model は scalar proof には適用しない。

| canonical run | UTC（2026-11-27） | exit | verdict | verification time | lane wall |
| --- | --- | ---: | --- | ---: | ---: |
| 変異前 | 17:28:15–17:28:26 | 0 | `SUCCESSFUL`、0/11 failed、cover 8/8 | 1.2636979 s | 10.147 s |
| 受理条件の `<=` を `<` に変更 | 17:29:18–17:29:29 | 1 | 変異検出、1/11 failed、cover 4/8 | 0.7932049 s | 10.514 s |
| 復元後 | 17:29:53–17:30:09 | 0 | `SUCCESSFUL`、0/11 failed、cover 8/8 | 1.8670735 s | 16.617 s |

各 run で Unwinding Assertions は有効。変異は `zero demand or an in-budget mathematical sum must
be accepted, including equality` の semantic assertion で検出し、unwinding failure ではなかった。
変異は commit せず元の `<=` に戻した。`git diff --exit-code HEAD -- crates/ruviform-io-archive/src/tar/read.rs`
が exit 0 であることを確認し、同じ selective lane と `mise run test-formal` を再実行した。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run test-formal-kani arc-idx-limit` | 0 | Pass。ZIP index、0/19 failed、cover 8/8、0.40871486 s |
| `mise run test-formal-kani arc-payload-limit` | 0 | Pass。TAR declared size、0/16 failed、cover 5/5、0.19302124 s |
| `mise run test-formal-kani arc-tar-measured-total` | 0 | Pass。0/58 failed、cover 14/14、1.4755414 s |
| `mise run test-formal-kani arc-tar-entry-progress` | 0 | Pass。0/43 failed、cover 14/14、1.5927713 s |
| `mise run test-formal-kani arc-tar-struct` | 0 | Pass。0/83 failed、cover 12/12、0.3933676 s |
| `mise run test-formal-kani invalid-selector` | 2 | Windows ホスト。期待どおり usage を表示し、環境探索前に終了 |

### コストと予算

事前予算は 5 CPU-min / 1 GiB。2026-11-27 17:32:30–17:32:37 UTC に、
mise の WSL2 分岐を複製し、同じ target・filter を使う
`cargo kani -j1 --output-format terse -p ruviform-io-archive --harness arc_tar_metadata_budget`
にだけ `/usr/bin/time -v` を付けた。これは補助証拠（canonical = mise）である。exit 0、
compile を含む wall 5.27 s、user 1.34 s + sys 0.55 s = 1.89 CPU-s、max RSS 105,112 KiB で、予算内だった。
canonical lane の wall と Kani が報告した verification time は上表の値を使う。

同じ TAR 判定の旧コストは未計測であり、改善率は出さない。rf806 の旧 ZIP parser-facing harness は
約 87 CPU-min / 7.3 GiB（verdict 未発行）であり、TAR scalar core と同一の保証境界ではない。

## Consumer Impact

各 repository の下記 SHA に対し、`crates/`、`Cargo.toml`、`Cargo.lock` を個別に検索した。
コマンドは `git -C <repo> grep -n -E 'ruviform-io-archive|ruviform_io_archive' <SHA> -- <scope>` と
`git -C <repo> grep -n -E 'TarArchiveReader|ArchiveReadLimits' <SHA> -- <scope>`。

| repository | SHA | crate reference（3 scope） | exact symbol（3 scope） |
| --- | --- | --- | --- |
| audio-lens | `70e979044adb1a829bcd5ae4baed54629653f714` | 0 / 0 / 0 | 0 / 0 / 0 |
| synvelis | `ab678e935fbdf7ec61781d448fdb067ec5de8d8e` | 0 / 0 / 0 | 0 / 0 / 0 |

全検索が exit 1（該当なし）で、代表一致はない。この slice の consumer 独立回帰は
NotApplicable とし、Issue 全体の consumer 受入条件は完了扱いにしない。

## 未証明領域と未実施の検証

新しい proof と既存 ZIP index proof の和集合は `ARC-IDX-LIMIT` 全体の証明ではない。
`advance` の BLOCK 計上、複数 entry と TAR metadata のストリーム全体での計上、
shell-core/parser-core 接続、TAR ヘッダー解析、PAX/GNU folding、path/link gate、padding 方針、
ZIP EOCD/CD/local-header 解析と整合性・overlap audit、decompression、CRC は対象外とした。

`advance` には、512 バイトを加算してから上限と比較する既存の算術リスクがある。
既定の 16 MiB では加算結果が表現範囲を超える前に予算超過を検出するが、公開 `max_metadata_bytes` は `u64::MAX` を許すため、
計上値が `u64::MAX - 511` に達した状態での次のヘッダー計上はオーバーフローし得る。
これはコード経路と算術からの評価であり、実ストリームでは再現していない。挙動・設定を変更せず、
BLOCK 計上とストリーム全体の計上を検証する後続 slice の候補に残す。

| lane | 判定 | 理由・後続 |
| --- | --- | --- |
| selector なしの `test-formal-kani` | NotRun | verdict 未発行の既知 harness を含む。本 slice は選択実行を要求する |
| native Kani 分岐 | NotRun | Windows ホストでは WSL2 分岐を実行した。両分岐が同じ filter を渡すことは差分で確認した |
| Workspace full / Feature Matrix / MSRV / 3 OS | NotRun | ローカル受入の対象外。統合検証は Daily CI の担当 |
| GUI-L2 / EXT-L3 / consumer 独立回帰 | NotApplicable | UI・外部実体の変更なし。consumer は上記の検索で直接利用なし |
| 手動 CI dispatch | NotRun | 通常 slice の受入で要求されない |

ローカル結果は変更範囲の証拠であり、merge 後 main の対象 SHA に対する CI の代替ではない。
docs index は Coordinator が統合時に整備する。
