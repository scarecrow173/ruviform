# rf1015 slice 1 verification — Property ledger / legacy exception design

## 基本情報

- 作業日: 2026-11-26（JST）。実行時刻は UTC で記録する。
- 対象 Issue: `scarecrow173/ruviform#1015`（slice 1。Issue を close しない）。
- base SHA / 検証対象 SHA: `ab6dc99fe8935ac09082d05e2176b10e729ba263`（作業開始時・最終確認時とも `origin/main` と同一）。
- 作業 SHA: 本 branch は base SHA 上の docs/evidence-only diff として検証した。commit SHA は evidence を含むため自己参照できず、PR の head SHA が最終的な編集 SHA になる。製品検証の verified sync commit はこの slice では進めない。
- OS/Arch: Windows 11 x86_64（MSYS2/MinGW shell）。`rustc 1.99.1 (58cf8f9c3 2026-10-13)`、`cargo 1.99.1 (5f94df478 2026-08-27)`、`mise 2026.11.2 windows-x64`。
- 変更区分: 製品コード、proof harness、proptest、Stub、Bound、`cfg(kani)` 挙動、`--no-unwinding-checks` invocation、mise lane、CI workflow は変更していない。

## 読んだ正本・監査対象

1. `G:/repositry/audio-products/.devin/dispatch-specs/rf1015-verification-redesign.explore.json` — scope、前提、verification tasks、residual work、uncertainty。
2. `gh issue view 1015 -R scarecrow173/ruviform --json body --jq .body` — live Issue本文。本 slice の AC は Issue 全体 AC であり、slice 1 だけで完了にしない前提を確認した。
3. `POLICIES.md` — §3 の verdict/requirement state、§5 のローカル検証と統合検証の分離、§6 の形式的検証方法論。
4. `AGENTS.md` — リポジトリ作業契約、検証入口、lockfile/docs index ルール。
5. `.claude/skills/japanese-tech-writing/SKILL.md` — 日本語文書の推敲基準。
6. `docs/specs/formal-verification.md` — rf#806 inventory、lane、Bound、未証明領域、legacy exception。
7. `.agents/skills/verify-ruviform/SKILL.md` — formal lane、evidence、sync commit、選択再実行手順。
8. `evidence/rf806/verification.md` / `evidence/rf806/runs.jsonl` — 旧 verdict、resource-caused `not_run`、mutation、コスト envelope。読み取り専用として参照し、改変していない。
9. 対象 test/harness/product reader: `formal_ipc.rs`、`formal_transport.rs`、`formal_archive.rs`、`formal_structured.rs`、`src/input.rs`、`formal_wav.rs`、`src/wav.rs`、`src/convert.rs`。
10. `mise.toml` — `test-formal`/`test-formal-kani` lane の現行 invocation。編集はしていない。

## coverage 確認方法と結果

手順: 各 formal test fileで `#[test]` に続く `fn` を proptest、`#[kani::proof]` に続く `fn` を Kani proofとして数え、in-src proof は `src/input.rs`、`src/wav.rs`、`src/convert.rs` を追加した。抽出した全 93 件の識別子が `evidence/rf1015/property-ledger.md` に verbatim で参照されることを Python で照合した。

結果:

| 対象 | proptest | Kani proof |
| --- | ---: | ---: |
| `crates/ruviform-ui/tests/formal_ipc.rs` | 7 | 3 |
| `crates/ruviform-ui-test/tests/formal_transport.rs` | 4 | 2 |
| `crates/ruviform-io-archive/tests/formal_archive.rs` | 18 | 3 |
| `crates/ruviform-io-structured/tests/formal_structured.rs` | 35 | 2 |
| `crates/ruviform-io-structured/src/input.rs` | — | 1 |
| `crates/ruviform-io-wav/tests/formal_wav.rs` | 9 | 2 |
| `crates/ruviform-io-wav/src/convert.rs` | — | 1 |
| `crates/ruviform-io-wav/src/wav.rs` | — | 6 |
| **合計** | **73** | **20** |

照合結果は missing proptest refs / missing Kani refs とも空だった。重複する test/proof 名（例: archive の `tar_declared_size_gate`、transport の `write_read_roundtrip`）は file 単位で数え、ledger では `pt:`/`kp:` を区別した。1 proofが複数 groupを補う場合は主 groupだけを counting 上の所有とし、他 groupでは共有参照とした。

## 5件の意思決定

詳細な options、根拠、保証差、独立根拠、費用、implementation slice は `property-ledger.md` §4。結論だけをここへ同期する。

1. structured `--no-unwinding-checks`: crate全体 invocationは撤廃する。最終形は CSV limit decisionを `csv_core` DFA/reader shellから分離した pure decision coreで証明する。移行中の proof単位限定は許容するが、例外存続は採用しない。既存3 proofは legacy-exception resultであり、通常 Proof Passに計上しない。
2. WAV `cfg(kani)` diagnostic substitution: lazy/pure diagnostic boundaryを採用し、置換は撤廃する。kind判定と診断付与を分離し、診断文面はwire契約に含めず未証明領域として明示する。verified stubbingとproperty testへの全面downgradeは採用しない。
3. wire/archive 8 harness: 現行形の予算拡大ではなく、pure decision core抽出とProperty単位分割を採用する。代表 important Propertyは `W-IPC-OP`、`W-FRM-READCLASS`、`ARC-IDX-LIMIT`、`ARC-PAYLOAD-LIMIT`。複合 parser shellが支配的な部分だけ downgrade/retireを検討する。
4. `read_frame` EOF: 製品 rustdocを正とする。length prefix 1〜3 byteの部分 EOFは clean EOFではなく `UnexpectedEof` とする契約差分を後続製品 slice へ分離する。この sliceでは実装・oracleを変更しない。現行Passは旧oracleに対するものであり、新契約の保証へ転用しない。
5. `SampleFormat` non_exhaustive: audio-core側の public canonical variant inventory（`SampleFormat::ALL`相当）と exhaustive probeを正本にし、variant追加時は compile errorで検出する。WAVの `%10`列挙は後続 sliceでその inventoryへ移行し、公開API追加の承認と命名は後続 sliceで確定する。

## 実行した正規検証

canonical command は mise/prek経由。実行記録の機械可読版は `runs.jsonl`。すべてローカル検証であり、merge後 main SHAの統合検証ではない。

| lane / command | 開始（UTC） | 終了（UTC） | exit code | verdict | 結果 |
| --- | --- | --- | ---: | --- | --- |
| `mise run test-formal` | 2026-11-25T21:05:27Z | 2026-11-25T21:06:47Z | 0 | Pass | baseline。73/73 proptest passed。 |
| `mise run fmt-check` | 2026-11-25T21:12:26Z | 2026-11-25T21:12:40Z | 0 | Pass | `cargo fmt --all -- --check` が差分なし。 |
| `mise exec -- prek run --all-files` | 2026-11-26T03:17:22Z | 2026-11-26T03:17:28Z | 0 | Pass | evidence を staging へ含めた最終実行。全 configured hook pass。auto-fix差分は残っていない。 |
| `mise run test-formal` | 2026-11-25T21:13:25Z | 2026-11-25T21:13:33Z | 0 | Pass | 最終確認。`formal_ipc` 7、`formal_transport` 4、`formal_archive` 18、`formal_structured` 35、`formal_wav` 9、計73/73 passed。 |

## 実行しなかった laneと理由

| lane / 検証 | state / verdict | 理由 |
| --- | --- | --- |
| `mise run test-formal-kani` | Requiredではない / NotRun | slice 1は proof実装を変更しない。現行 wire/archive invocationが verdictを返せない既知問題であり、環境・長時間コストは後続 sliceの設計対象。dispatchでも実行しない明示指示。 |
| Workspace full lane、Feature Matrix、MSRV、3OS横断 | NotApplicable | docs/evidence-only slice。変更範囲に比例した検証原則（`POLICIES.md` §5）により必須化しない。 |
| GUI-L2 / EXT-L3 | NotApplicable | UI実行面・外部実機面を変更していない。 |
| Mutation実走 | NotRun / 後続 slice | mutation matrixは ledger §6で設計したが、製品実装を一時変更して実走するのはこの sliceの範囲外。rf806の既存 mutation証跡は改変せず参照した。 |
| consumer独立回帰（audio-lens / synvelis） | NotApplicable / 後続 slice | 共通実装変更がない。製品変更を伴う後続 sliceで必要な独立回帰を設計する。 |
| 手動 CI dispatch | NotApplicable | ユーザー指定どおり通常 sliceは CI非ブロッキング。手動 dispatchを受入ゲートにしない。 |

## sync state

| 対象 | state | 根拠 |
| --- | --- | --- |
| Property ledger | `synced`（この sliceの設計成果物として） | 73/73 proptestと20/20 Kani proofの参照照合が空 missing。 |
| `docs/specs/formal-verification.md` | `synced` | ledger参照、5件の設計方向、legacy exception会計、選択再実行 `not_synced` を追加。既存 inventoryは削除していない。 |
| `.agents/skills/verify-ruviform/SKILL.md` | `synced` | 英語スタイルを保ち、ledger導線・選択再実行・Proof Pass会計の最小限だけ追加。 |
| mise lane / 選択再実行入口 | `not_synced` | `mise.toml`を編集できず、Property group単位の入口は未実装。未実装を syncedと記録していない。 |
| 製品コード / proof harness / proptest | `deferred` | 実装変更は後続 slice。特に `read_frame` EOFは製品契約差分の承認と実装が必要。 |
| `evidence/rf806/` | `synced`（read-only参照） | 改変していない。旧 verdictと SHAを現行 verdictへ転用していない。 |
| docs index | `not_synced`（許容） | dispatchの共通契約どおり `docs/**/README.md`は編集・再生成していない。Coordinatorが統合時に整備する。 |

## 残留リスク・未解決事項

- 現行 structured 3 proofは legacy exception下の結果であり、この sliceで Proof Passへ昇格していない。DFA/reader境界再設計と再検証が必要。
- WAVのcfg差分は design decisionのみで未実装。diagnostic payload文面は今後も契約に含めない限り未証明領域である。
- wire/archive 8 harnessは依然 verdict未取得。ledgerの予算は rf806 envelopeを baselineにした事前予算であり、実測ではない。
- `read_frame` EOFについて製品 rustdocを正と判断したが、実装変更と公開挙動の承認は後続 slice。旧oracleに対する proptest Passを新契約の証拠に使わない。
- 選択再実行 runnerは未実装。group単位の予算記録・選択実行は ledger設計どおり後続 sliceで実装する。
- `origin/main` `ab6dc99f` の必須 CI証跡は統合 gateであり、このローカル証拠で代替しない。rf#1021 trackerはこの sliceの作業停止理由にしなかったが、merge判断では Coordinatorが確認する。
- 探索 JSONが Open Issue全件の逐語精読ではないことは前提どおり。別 Issueの hard dependencyはこの sliceの作業中には発見しなかった。

## slice 2 verification

`W-IPC-OP` pure decision core sliceの検証記録は companion文書 `evidence/rf1015/verification-slice2.md` にある。本書の slice 1 記録は変更していない。
