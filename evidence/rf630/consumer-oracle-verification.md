# RF-VTEST AC7 consumer-oracle slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（AC7 consumer-oracle slice —
  fixture contract manifest + consumer oracle 対応表 + dep 非汚染
  evidence の ruviform 側完結 headless slice）
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints
  正本）
- 記録日: 2026-11-21（初回）・2026-11-24（re-pin — #842/#885 の
  README drift による pin 再照合。[re-pin（2026-11-24）](#re-pin2026-11-24)
  節に全記録）・2026-11-25（再評価・re-pin — sv#176 S1 の dev-dep
  採用による dep 非汚染再実測・#532 CLOSED による residual
  resolved 化・S1 headless leg 実走の反映。
  [再評価・re-pin（2026-11-25）](#再評価re-pin2026-11-25) 節に全記録）
- 本 slice は docs+evidence のみの変更 — `.rs`・`tests/`・
  `mise.toml`・`Cargo.toml`/`Cargo.lock`・`.github/**`・
  `docs/**/README.md`・他リポジトリは zero-diff。

## 実装概要

- `evidence/rf630/consumer-oracle/manifest.json`（schema
  `rf630-fixture-contract/1`）— 全 7 fixture class の descriptor
  identity・contract face を crate README の節への pin + README
  sha256 で宣言した正本表。contract 値は複写しない
  （`abi-validator/run.json` の schema `rf630-abi-validator-run/2`
  先例に倣う）。
- consumer oracle 対応表（manifest 内 `consumerOracleMap`）—
  sv#101→`PluginBindingSession`/`check_frame_base`（param
  binding）、sv#127→`ProcessBoundarySession`/`PrecisionProvenance`
  （process boundary+MIDI/transport+crash isolation）、
  sv#128→`PluginStatePort`/`StateCaptureQuality`（RFST v2 state
  quality）、sv#176→test_editor PD-01..07、al#127→NotApplicable。
- `docs/{specs,designs,test-specs,discovery}/issue-630-consumer-
  oracle.md` — doc chain 4 文書。
- `crates/ruviform-plugin-test-{vst3,clap}/README.md` — contract
  面補完（`test_instrument`/`test_spawn_fail` 節・fault 8 帯域表・
  editor event token 表・preset 面・CLAP `plugin_count()==6` と
  features 訂正）。#842/#885 の README 更新は本 slice 外の各
  slice 実施 — 本記録は pin 再照合のみ。

## 受入条件の照合（slice 確定版）

| 受入条件 | 結果 | 証拠 |
| --- | --- | --- |
| manifest が schema 名つき正本表として成立（全 7 class・README pin・二重正本化なし） | Passed | `manifest.json`（`rf630-fixture-contract/1`）+ 下記 cross-check で sha256/節 pin 全一致（初回 2026-11-21・re-pin 2026-11-24・再評価 2026-11-25 の各時点で実測 — 2026-11-25 は README drift なしで pin 据置を実測で証明） |
| 対応表が 5 consumer を網羅 | Passed | `consumerOracleMap` 5 行 — sv#101/#127/#128/#176 + al#127(NotApplicable) |
| verdict 表の正直な区別 | Passed | contractShape=`headless-contract-match`（synvelis 4 行）・consumerRun=`external-owner`・residuals は open 2 件（popup harness・consumer legs）+ resolved 2 件（clap-output-event-drain・#842 解消 / ac4-real-window-dcomp・#532 CLOSED — 2026-11-25 再評価で resolved 化）に Pass を付けていない |
| dep 非汚染の実測記録 | Passed | 下記 grep 実測 — 初回・re-pin（2026-11-24）時点では両 consumer で exit 1（非含有）。2026-11-25 再実測で synvelis は fixture 3 crate 含有（exit 0・`synvelis-app` `[dev-dependencies]` のみの consumer 自主採用・production DAG 非含有）・audio-lens は非含有（exit 1）— 「強制しない」の 2 半分（構造+実測）で正直に分離記録。AC7 の AudioLens 節は不変に成立 |
| doc chain + evidence + README が揃う | Passed | 本記録・doc chain 4 文書・manifest・両 crate README |
| lease 境界の遵守 | Passed | `git status` — 変更は manifest・本記録・spec doc のみ。`.rs`/`tests/`/`mise.toml`/`docs/**/README.md`/`Cargo.*`/`.github/**`/他 repo（synvelis・audio-lens `git status` 両方 clean）に差分なし |

## deterministic evidence

正規 lane（`mise.toml` タスク）経由。OS: Windows (OS-L1)。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Passed — `.rs` 非回帰の非破壊確認（本 slice は `.rs` を触らないため形式確認の意味）。2026-11-21 実施・2026-11-24 再実施・2026-11-25 再実施（再評価後の最終状態で exit 0） |
| `mise exec -- prek run --all-files` | 0 | Passed — markdown/構成ファイル衛生の canonical（trailing whitespace・EOF・yaml・toml・line ending・BOM・private key・illegal names・actionlint 全緑）。2026-11-21 実施・2026-11-24 再実施・2026-11-25 再実施（再評価後の最終状態で exit 0） |
| `mise exec -- node -e "JSON.parse(require('fs').readFileSync('evidence/rf630/consumer-oracle/manifest.json','utf8'))"` | 0 | Passed — manifest JSON 構文正当（2026-11-25 再評価の編集後最終状態で実施） |
| `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" ../synvelis/Cargo.lock ../audio-lens/Cargo.lock` | audio-lens: 1 / synvelis: 0（2026-11-25 再実測。初回・2026-11-24 re-pin 時点は両方 1） | Passed — **2026-11-25 再実測**: audio-lens `Cargo.lock` は fixture crate 名非含有（exit 1 = no match = 期待値。`ruviform-audio-test` は #628 dev-dep 経路で fixture ではない）、synvelis `Cargo.lock` は fixture 3 crate 名を含有（exit 0 — `ruviform-plugin-dock-test`・`ruviform-plugin-test-clap`・`ruviform-plugin-test-vst3`）。synvelis 側含有は sv#176 S1（synvelis PR #640）で `crates/synvelis-app/Cargo.toml` `[dev-dependencies]` にのみ追加された consumer 自主採用 — `[dependencies]`（production DAG）には fixture crate 非含有を同ファイルで確認。「強制しない」は ruviform production DAG 非依存（構造半分・維持）+ AudioLens 節（実測半分・不変）で成立を維持。詳細は [再評価・re-pin（2026-11-25）](#再評価re-pin2026-11-25) 節 |

grep 実測の対象（再評価時点・2026-11-25）:

- synvelis `Cargo.lock` @ `8585cda756231f4e33a50370b4e1f3cf42c53059`
  （ruviform resolved rev `20dc41f71c47674b3e51d1eb0921124b94f8bfaf`
  — fixture 3 crate を `synvelis-app` dev-dep 経路で含有。lock 内の
  fixture crate エントリは全て同 rev を指す）
- audio-lens `Cargo.lock` @ `7fd87add13e24858f6677d9c2dea135bf581ebe6`
  （ruviform resolved rev `e4c9f6f0715da3ac479f8c437d21a021327e51da`
  — fixture crate 非含有・resolved rev は初回測定から不変）

grep 実測の対象（re-pin 時点・2026-11-24）:

- synvelis `Cargo.lock` @ `726f80e76ae7bfed607bef8717cb50fab254fe19`
  （ruviform resolved rev `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`
  — 初回測定時と同一 rev を維持）
- audio-lens `Cargo.lock` @ `04be32dcfb802ccb4f3e40f11ebebe8b1d3f2f03`
  （ruviform resolved rev `e4c9f6f0715da3ac479f8c437d21a021327e51da`
  — 初回測定時と同一 rev を維持）

grep 実測の対象（初回・2026-11-21）:

- synvelis `Cargo.lock` @ `d68f0e54c09763b93eb556353098c7a92532dfcf`
  （ruviform resolved rev `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`）
- audio-lens `Cargo.lock` @ `fef857f684997449725e9f1bf38eca354a748d2c`
  （ruviform resolved rev `e4c9f6f0715da3ac479f8c437d21a021327e51da`）

## manifest ↔ crate README 整合照合

照合手順（再現可能）:

1. 両 README の sha256 を実測し manifest
   `authoritativeContract.sources[].readmeSha256` と突合する
   （`sha256sum <file>`）。
2. `fixtureClasses` が 7 class 全員を持つことを確認。
3. 各 `contractFaces.*.pin` を節 anchor（`## ` 見出しを kebab-case
   化）に解決し、class の ABI が持つ README 側に実在することを
   確認（vst3-only の `test_stateful_single` は VST3 README のみに
   解決）。
4. `consumerOracleMap` が 5 consumer 行を持つことを確認。

### 再評価（2026-11-25 — 現行記録）

- `crates/ruviform-plugin-test-vst3/README.md` =
  `4fa2ac27af528d3f82cd89b65f7d5c2bac4f9d9de2cbababafde86d7956ed00f`
  （266 行）→ MATCH（2026-11-24 re-pin 値と同一 — README drift なし。
  `sha256sum`/`wc -l` で再実測）
- `crates/ruviform-plugin-test-clap/README.md` =
  `8bdb571ff7a98e471ac9a72591594c95eee06bd4487551b63becd54053047024`
  （254 行）→ MATCH（同）
- `fixtureClasses` 7 class → MATCH
- pin anchor 解決 → missing pin なし（両 README の `## ` 見出しを
  再列挙して全 pin anchor の実在を確認 — `#what-it-provides`/
  `#test_gain-semantics`/`#test_fault-semantics`/`#test_editor-semantics`/
  `#test_stateful-semantics`/`#test_instrument-semantics`/
  `#test_spawn_fail-semantics`/`#realtime-contract` は両 README、
  `#test_stateful_single-semantics` は VST3 README のみに実在
  = class の ABI 構成と一致）
- `consumerOracleMap` 5 行 → MATCH

照合結果: PASS（exit 0）。README が 2026-11-24 pin から無 drift
のため declared face の再照合は変更不要 — 据置を実測値で証明した。

### re-pin 後（2026-11-24 — 当時記録）

- `crates/ruviform-plugin-test-vst3/README.md` =
  `4fa2ac27af528d3f82cd89b65f7d5c2bac4f9d9de2cbababafde86d7956ed00f`
  （266 行）→ MATCH
- `crates/ruviform-plugin-test-clap/README.md` =
  `8bdb571ff7a98e471ac9a72591594c95eee06bd4487551b63becd54053047024`
  （254 行）→ MATCH
- `fixtureClasses` 7 class → MATCH（`test_editor` に #885 の
  param/fault 面を `paramSurface`/`faultModes` として pin 追加 —
  下記 re-pin 節）
- pin anchor 解決 → missing pin なし（追加した
  `#test_editor-semantics` は両 README に実在）
- `consumerOracleMap` 5 行 → MATCH

照合結果: PASS（exit 0）。

### 初回 pin（2026-11-21 — 当時記録）

- `crates/ruviform-plugin-test-vst3/README.md` =
  `ded84ae15239d63f622f5490a09134ce6852341171d60119cc25c1552dcd0829`
  （260 行）→ MATCH（commit `7703132c` 時点の README）
- `crates/ruviform-plugin-test-clap/README.md` =
  `6a252f63fe0e321c1c238200786d0bde8380efc1b9e6254054ba2d299ddebdc0`
  （248 行）→ MATCH（同）
- `fixtureClasses` 7 class・pin anchor 解決・`consumerOracleMap`
  5 行 → 全て MATCH。

照合結果: PASS（exit 0）。#842（`116c1cb2`・CLAP README 更新）と
#885（`dc32b8f0`・VST3 README 更新）の merge で初回 pin が drift
化したため 2026-11-24 に再照合（上記）。

## 再評価・re-pin（2026-11-25）

3 つの外部変化で manifest が stale 化したため再検証・re-pin した
（docs+evidence のみ・`.rs`/`tests/`/`mise.toml`/`Cargo.*` は
zero-diff）:

1. **dep 非汚染**: sv#176 S1（synvelis PR #640・merge commit
   `71275d85747c05741787e8b20f0f7f3c4e51f287`）で synvelis が
   fixture 3 crate を `synvelis-app` dev-dep 採用し、現行 synvelis
   `Cargo.lock` に含有 — 旧記録「両 consumer 非含有」は撤回し、
   2 半分構造で正直に分離記録。
2. **`residuals.ac4-real-window-dcomp`**: `scarecrow173/ruviform#532`
   は CLOSED（2026-11-21T11:35:15Z — `gh issue view 532 --repo
   scarecrow173/ruviform --json state,closedAt` で実測、state=
   CLOSED）— `gate` → `resolved`。
3. **sv#176 headless legs**: S1 headless real-ABI probe は実走済み
   — `consumerRun` の `disposition` は `external-owner` 維持で
   `note` に実走内訳を追記。

### 実測方法と再現手順

- consumer lock grep: `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" ../synvelis/Cargo.lock ../audio-lens/Cargo.lock`
  （ruviform repo 直下から consumer repo はワークスペース兄弟
  `G:/repositry/audio-products/{synvelis,audio-lens}` を read-only
  参照）
- dev-dep 経路の確認: `crates/synvelis-app/Cargo.toml` で fixture
  crate が `[dev-dependencies]`（41-53 行・コメントに sv#176 S1
  由来と artifact 解決の経路が明記）にのみ存在し、
  `[dependencies]`（11-39 行・production DAG）には非含有である
  ことを目視で確認
- oracle anchor 再 grep: 下記のとおり synvelis 現行 HEAD で全件
- README sha256/行数: `sha256sum` / `wc -l`（初回・re-pin と同一
  基準）

### consumer lock grep（2026-11-25 実測）

| repo | 対象 SHA | ruviform resolved rev | exit | 結果 |
| --- | --- | --- | --- | --- |
| synvelis | `8585cda756231f4e33a50370b4e1f3cf42c53059` | `20dc41f71c47674b3e51d1eb0921124b94f8bfaf` | 0 | fixture 3 crate 名を含有（`ruviform-plugin-dock-test`/`ruviform-plugin-test-clap`/`ruviform-plugin-test-vst3` — lock 内エントリは全て同 rev）。**ただし経路は `synvelis-app` の `[dev-dependencies]` のみ** — production DAG 非含有 |
| audio-lens | `7fd87add13e24858f6677d9c2dea135bf581ebe6` | `e4c9f6f0715da3ac479f8c437d21a021327e51da` | 1 | fixture crate 名非含有（期待値どおり。`ruviform-audio-test` は #628 dev-dep 経路で fixture ではない） |

解釈: AC7「Ruviform Test Plugin 自体が AudioLens への新依存を
強制しない」の AudioLens 節は不変に成立する。synvelis の含有は
ruviform による強制ではなく consumer 側の自主採用 — sv#176 S1
の headless probe が fixture cdylib の実バイナリを必要とし、
dev-dep edge がそれを `target/<profile>/deps/` にビルドする経路
（synvelis 側 `evidence/sv176/s1-headless-probe.md` がこの動機を
明記）。ruviform 側の構造半分（production crate は fixture crate
に非依存・test-only 宣言）は変化していない。manifest の
`dependencyNonContamination` をこの分離記録に更新した。

### oracle anchor 再 grep（2026-11-25 実測）

synvelis @ `8585cda756231f4e33a50370b4e1f3cf42c53059`（現行 HEAD）
で全 anchor の存在を grep 実測 — 差異なし:

- `crates/synvelis-player/src/binding.rs`: `PluginBindingSession`
  （L351）・`check_frame_base`（L120）
- `crates/synvelis-player/src/process_boundary.rs`:
  `ProcessBoundarySession`・`PrecisionProvenance`（L768）
- `crates/synvelis-player/src/plugin_state.rs`: `PluginStatePort`・
  `PluginStateHost`
- `crates/synvelis-project/src/plugin_state.rs`:
  `StateCaptureQuality`（L45）
- `evidence/sv176/pd-01-07-case-definitions.md`: `PD-01..07`・
  `PluginArtifactEvidence` / `evidence/sv176/acceptance-ledger.md`
  （存在確認）

manifest の全 `oracleAnchors[].repoShaVerified` を `726f80e7…` から
`8585cda7…` へ更新した（6 箇所）。

### README sha256/行数の実測（2026-11-25）

ruviform HEAD `1009a94fb0ad6032a65e2d72b1cbe56b19f9b593` で再実測:

- vst3 README: `4fa2ac27…` / 266 行 — **2026-11-24 pin と同一
  （drift なし）**
- clap README: `8bdb571f…` / 254 行 — **同**

drift がないため `authoritativeContract.sources[]` の pin は据置
— 据置を実測値で証明した。manifest↔README 整合照合（本ファイル
上記節の 4 手順）を再現し PASS（exit 0）。

### residuals disposition 変更の根拠

`ac4-real-window-dcomp`: 第一根拠は ac-inventory AC4 行の Pass 判定
（`evidence/rf630/ac-inventory.md` — VST3 leg 32 case
（`plugin-dock-real-scenario.jsonl` @ `41b8a6ec`）+ CLAP leg 14
case（`plugin-dock-real-clap.jsonl` @ `41b8a6ec`）全 pass・class
A/B 修正実走 + class C 確定機序の `expected_outcome` 記録で DComp
既知 Fail の再現・解消判定に使用可能）。第二根拠は
`scarecrow173/ruviform#532` の CLOSED（2026-11-21T11:35:15Z — 修正
Issue 自体の着地）。`clap-output-event-drain` の先例どおり entry
は追跡のため残し、`disposition: "resolved"` +
`resolvedEvidence` で ac-inventory AC4 行を指す。
`popup-real-hwnd-harness`（rf#631 real-dock lane）と
`consumer-run-legs`（各 consumer Issue）の `external-owner` は
変更していない — AC7 の実質残りである consumer 製品 Host 実走
legs は sv#101/#127/#128/#176 所有の gate として残る。

### sv#176 S1 実走内訳の出典

全て synvelis 側 evidence の read-only 実測（synvelis @ `8585cda7`）:

- `evidence/sv176/s1-headless-probe.md` — HEADLESS_MATRIX verdict 表:
  `artifact`/`host-lifecycle`/`session-lifecycle`/`host-terminate`/
  `open-or-degrade` = pass（実 VST3 test_gain バイナリ
  `ruviform_plugin_test_vst3-1ac2c390d2654dc3.dll` sha256
  `c12df015…` in-process）、`view-mount`/`command-receipt` = pass
  （fake driver 行 — 実バイナリ行とは区別）、`editor-attach` =
  unsupported（`createView` null — test_gain は editor 面を持た
  ない defined refusal）、`dock-pixels` = not_run（VisualCapture
  不在）
- `evidence/sv176/s1-legs/*.jsonl`（5 ファイル）— product/CLAP legs:
  `clap-artifact`/`clap-host-lifecycle` = pass（実 CLAP バイナリ）、
  `product-dock-test-gain-open`/`product-dock-test-editor-attach`/
  `product-dock-clap-test-gain-open` = unsupported（いずれも
  defined refusal — `Ops(Unsupported)`/`Failed{PluginFailed}` 系で
  fake pass ではない。pass 相当と読み違えない）
- `evidence/sv176/s1-headless-probe.jsonl` — `run_all` lane 文書
  （canonical jsonl）
- `crates/synvelis-app/tests/plugin_dock_probe.rs` — 7 `#[test]`
  （`s1-headless-probe.md` 記録では 7 passed / 0 failed・exit 0）
- `evidence/sv176/acceptance-ledger.md` — 行 1/7（Real OS/Real
  Plugin PD-01..07 legs）は S1 後も `NotRun` のまま — S1 は
  in-process real-ABI の headless 結果であり、Native Dock/実 OS
  受入に昇格していないことが ledger 自身に明記されている

manifest sv#176 行の `consumerRun.note` にこの内訳を追記した。
`disposition` は `external-owner` 維持（real-OS PD-01..07 legs は
sv#176 + rf#631 責務のまま）で、`contractShape`
（`headless-contract-match`）は変更不要 — 実走は consumerRun 側の
記録であり Pass/Fail の consumer 判定は付けない。

### re-pin 履歴（freeze SHA）

- `repo.shaAtFreeze`/`repo.branch` を HEAD 値
  （`1009a94fb0ad6032a65e2d72b1cbe56b19f9b593`・
  `docs/630-ac7-consumer-oracle-reeval`）へ更新 — schema の意図は
  「freeze 時点の正本表」（2026-11-24 re-pin 時の判断と同一根拠）。
- 直前 freeze: `5ea61d83ddfe6eba4849a05b5c9c4962478ef464`・
  `docs/630-consumer-oracle-manifest-repin`（2026-11-24 re-pin）
- 初回 freeze: `812555d8`・`docs/630-consumer-oracle`（2026-11-21）
  — git 履歴と 2026-11-24 節に記録

### 実行した検証コマンド（2026-11-25）

- `mise exec -- node -e "JSON.parse(require('fs').readFileSync('evidence/rf630/consumer-oracle/manifest.json','utf8'))"` → exit 0（manifest JSON 構文正当）
- manifest↔README 整合照合手順の再現（上記）→ PASS
- consumer lock grep 再実測（上記表）→ 記録どおり
- `mise run fmt-check` → 実施（結果は deterministic evidence 表に統合記録）
- `mise exec -- prek run --all-files` → 実施（同）
- `mise run test ruviform-plugin-test-vst3` / `mise run test ruviform-plugin-test-clap` → **NotRun** — 発動条件（調査中に fixture の実挙動と manifest declares の乖離 = コード側 bug の疑いを発見）に非該当。今回の乖離は全て「文書の未更新」（外部状態変化の未反映）であり、fixture 実挙動の回帰は確認されなかった。docs+evidence slice のため cargo 系は原則不要

### 受入条件の照合（2026-11-25 再評価）

| 項目 | 結果 | 根拠 |
| --- | --- | --- |
| dep 非汚染の再実測・分離記録 | Pass | 上記 grep 表 + `synvelis-app/Cargo.toml` 確認 — AudioLens 節不変・synvelis dev-dep 自主採用・production DAG 非依存の 3 点を分離記録 |
| `ac4-real-window-dcomp` → resolved | Pass | #532 CLOSED 実測 + ac-inventory AC4 行 Pass — entry は追跡のため残存 |
| sv#176 `consumerRun.note` 内訳 | Pass | 出典は全て synvelis 側実測 evidence。fake driver 行と実バイナリ行を区別・product legs の unsupported を pass 相当と読み違えない表現 |
| re-pin（freeze SHA・anchors・README pin） | Pass | HEAD `1009a94f`・anchors 6 件再 grep 差異なし・README 無 drift を実測で証明 |
| lease 境界 | Pass | 変更は manifest・本記録・spec doc の 3 ファイルのみ（`git status`）。`.rs`/`tests/`/`mise.toml`/`Cargo.*`/`.github/**`/docs index/doc chain 兄弟/両 crate README/他 repo は zero-diff |
| AC7 確定判定 | NotRun（継続） | consumer 実走 legs（sv#101/#127/#128/#176 所有の gate）が残るため Issue は開いたまま — 本 slice は ruviform 側証拠の再検証・re-pin のみで gate を更新しない |

## re-pin（2026-11-24）

#842・#885 の README 変更で `readmeSha256`/`readmeLines` pin が
stale 化したため、manifest を現行 HEAD
（`5ea61d83ddfe6eba4849a05b5c9c4962478ef464`・branch
`docs/630-consumer-oracle-manifest-repin`）の README に再 pin し、
declared face を README 本文と再照合した。docs+evidence のみ・
`.rs`/`tests/`/`mise.toml`/`Cargo.*` は zero-diff。

### 実測方法

- sha256: `sha256sum crates/ruviform-plugin-test-{vst3,clap}/README.md`
- 行数: `wc -l`（改行コード数 — 両 README は末尾改行あり。
  初回 pin の 260/248 も `wc -l` 基準であることを commit
  `7703132c` 時点の README で実測確認したため、同一基準で再測）
- 測定値: vst3 `4fa2ac27…`/266 行（初回 `ded84ae1…`/260 行）、
  clap `8bdb571f…`/254 行（初回 `6a252f63…`/248 行）

### declared face 再照合の結果（README が正本）

| 面 | 判定 | 根拠 |
| --- | --- | --- |
| `test_editor`（vst3） | face 追加 | #885 で「Parameterless pass-through」→ shared stepped `Fault` param carrier（`fault.rs` 正本の in-band armed-fault law verbatim・editor-session crash/hang oracle）。manifest に `paramSurface`/`faultModes` を pin 追加。CLAP `test_editor` は parameterless のまま（README CLAP 節確認）— ABI 非対称は `eventTokens` 先例どおり `declares` 本文に「CLAP: 対応面なし」と記す |
| `test_fault`（clap） `faultModes` declare | 文言更新 | #842 で `OutputEvents::void` discard → `drain_output_reports` canonical drain。「honest 境界」の内容を `bad_time` stamp = host-observable 到達・`invalid` NaN = counted defined skip・push = best-effort へ更新 |
| `test_fault.faultModes` と `test_editor` の整合 | 矛盾なし | test_editor declares が「test_fault の law verbatim・`fault.rs` 正本」と参照する形 — test_fault 側 declare は自 class の面を記すのみで共有に矛盾しない |
| `residuals[0]` `clap-output-event-drain` | `resolved` へ | #842（`116c1cb2`）で実装済み — open 残件から外す。追跡のため entry は残し `disposition: "resolved"` + `resolvedEvidence: evidence/rf630/clap-output-drain-verification.md` を記す（`policyVocabulary.dispositions` に `resolved` を追加 — 構造不変で schema は `rf630-fixture-contract/1` のまま） |
| `consumerOracleMap` 5 行 `matched` | sv#176 のみ文言更新 | sv#101/#127/#128 の `matched` は pin 更新後も成立（crash isolation 面・state capability 対照に変化なし）。sv#176: PD schema dims 対応は不変 — #885 の Fault carrier は PD-06 crash/hang leg が消費しうる面だが消費宣言は anchor 実測外のため `contractFacesUsed` は不変・`matched` に「pin 追加・消費は sv#176 判断」と記す |
| `repo.shaAtFreeze`/`repo.branch` | HEAD 値へ更新 | schema の意図は「freeze 時点の正本表」— pin が現行 README を指す以上 freeze SHA も現行 HEAD でなければ表内で自己矛盾する。元 freeze（`812555d8`・`docs/630-consumer-oracle`）は本節と git 履歴に記録を残す |
| `oracleAnchors[].repoShaVerified` | 現行 HEAD 値へ更新 | synvelis `726f80e76ae7bfed607bef8717cb50fab254fe19` で全 anchor の存在を grep 実測（`PluginBindingSession`/`check_frame_base`・`ProcessBoundarySession`/`PrecisionProvenance`・`PluginStatePort`/`PluginStateHost`・`StateCaptureQuality`・`evidence/sv176/` 両ファイルの `PD-01..07`/`PluginArtifactEvidence`）— 差異なし |
| `dependencyNonContamination` | 再実測・pin 更新 | dep grep 再実施で exit 1（非含有）。両 consumer HEAD が初回測定から前進したため `consumerPinsAtMeasurement` を実測値へ更新（ruviform resolved rev は両者とも不変） |
| `readmeLines` 数え方 | `wc -l` 踏襲 | 初回 pin（260/248）が `wc -l` 基準であることを `7703132c` 時点 README で実測確認。末尾改行ありのため `wc -l` = 行数 |

### oracle anchor の現状（read-only 実測・re-pin 時点）

synvelis @ `726f80e76ae7bfed607bef8717cb50fab254fe19` で全 anchor
の存在を grep 確認（差異なし）:

- `crates/synvelis-player/src/binding.rs`:
  `PluginBindingSession`・`check_frame_base`
- `crates/synvelis-player/src/process_boundary.rs`:
  `ProcessBoundarySession`・`PrecisionProvenance`
- `crates/synvelis-player/src/plugin_state.rs`:
  `PluginStatePort`・`PluginStateHost`
- `crates/synvelis-project/src/plugin_state.rs`:
  `StateCaptureQuality`
- `evidence/sv176/pd-01-07-case-definitions.md`: `PD-01..07`・
  `PluginArtifactEvidence` / `evidence/sv176/acceptance-ledger.md`

### 実行した検証コマンド（re-pin）

- `mise exec -- prek run --all-files` → exit 0（上記 deterministic
  evidence 表に統合記録）
- `mise run fmt-check` → exit 0（同）
- 上記 manifest↔README 照合手順の再現 → PASS
- `mise run test ruviform-plugin-test-{vst3,clap}` → **NotRun** —
  発動条件（declared face が fixture の実挙動と乖離する変更の
  発見）に非該当。#885/#842 の README 記述は fixture 実装と一致
  しており、文書の未更新ではなく実挙動の回帰は確認されなかった。
  docs+evidence slice のため cargo 系は原則不要（Spec 条件分岐）

## oracle anchor の現状（初回・2026-11-21 記録）

探索記録どおり全 anchor が存在（consumer SHA は上記初回表）:

- `crates/synvelis-player/src/binding.rs`: `PluginBindingSession`・
  `check_frame_base` — dispatch が (binding, event) 全件を staging
  前に検証する binding oracle。
- `crates/synvelis-player/src/process_boundary.rs`:
  `ProcessBoundarySession`・`PrecisionProvenance` — 4 generation
  pin で delivery context を固定する boundary oracle。
- `crates/synvelis-player/src/plugin_state.rs` +
  `crates/synvelis-project/src/plugin_state.rs`:
  `StateCaptureQuality`（BestEffort/Coordinated/Quiescent）。
- `evidence/sv176/{pd-01-07-case-definitions,acceptance-ledger}.md`:
  PD-01..07 schema dims + lane 分離。freeze どおり Real OS/Real
  Plugin 行は `NotRun`（`editor-attach` 行の upstream
  Unsupported は #630 側 slice で fixture は解消済み）。
- `scarecrow173/audio-lens#127` 本文: 「#630実Plugin Test
  SuiteはAudioLensが現在その機能を製品提供しない限り
  NotApplicable」の宣言を確認 → NotApplicable 記録のみ。

## verdict 表（本 slice の照合結果）

| consumer | contractShape | consumerRun | 備考 |
| --- | --- | --- | --- |
| `scarecrow173/synvelis#101` | headless-contract-match — param id/domain/normalized・frame offset・last-wins が test_gain pin で宣言済み | external-owner（sv#101 の AC/lane） | 実走 leg は consumer Issue 責務 |
| `scarecrow173/synvelis#127` | headless-contract-match — param/PCM・MIDI/transport・fault mode pin で宣言済み。fixture 側 crash isolation oracle は #630 L1 e2e で done | external-owner（sv#127 の AC/lane） | 実走 leg は consumer Issue 責務 |
| `scarecrow173/synvelis#128` | headless-contract-match — RFST v2 layout・provenance・typed rejection が pin 済み。test_fault/test_spawn_fail は capability 対照 | external-owner（sv#128 の AC/lane） | 実走 leg は consumer Issue 責務 |
| `scarecrow173/synvelis#176` | headless-contract-match — 実 child HWND・popup・event token・CLAP floating が pin 済み。#885 の Fault carrier 面も pin 追加（消費は sv#176 判断） | external-owner（sv#176 + ruviform#631） | PD-01..07 Real OS/Real Plugin 行は sv176 ledger でも NotRun。S1 headless real-ABI legs は実走済み（2026-11-25 再評価で note 追記 — in-process であり Native Dock 受入ではない） |
| `scarecrow173/audio-lens#127` | external-owner（照合対象なし） | external-owner — NotApplicable | AL は plugin host 非提供 |

## 残件（本 slice 外 — external-owner/gate/resolved）

- ~~CLAP output-event の host 側観測経路（`OutputEvents::void` →
  drain 配線）~~ — **resolved**: `scarecrow173/ruviform#842`
  （`116c1cb2`）で `drain_output_reports` canonical drain として
  実装済み。解消 evidence:
  `evidence/rf630/clap-output-drain-verification.md`。
- editor popup の real-HWND host harness / L1 editor 面 —
  external-owner・`scarecrow173/ruviform#631` real-dock lane 責務。
- ~~AC4 実 Window/DComp 既知 Fail 再現・解消判定 —
  gate・`scarecrow173/ruviform#532` 着地待ち（既知 soft）~~ —
  **resolved**（2026-11-25 再評価）: #532 は CLOSED
  （2026-11-21T11:35:15Z・`gh issue view` 実測）。ac-inventory
  AC4 行は Pass 判定済み（VST3 leg 32 case + CLAP leg 14 case
  全 pass・class A/B 修正実走 + class C 確定機序の
  `expected_outcome` 記録で再現・解消判定に使用可能）。解消
  evidence: `evidence/rf630/ac-inventory.md`（AC4 行）。
- consumer 実走 legs — external-owner・各 consumer Issue
  （sv#101/#127/#128/#176）の AC/lane 責務。al#127 は
  NotApplicable で実走 leg なし。sv#176 S1 で headless real-ABI
  legs は実走済み（`evidence/sv176/s1-headless-probe.md` ほか）
  が in-process であり Native Dock/実 OS の PD-01..07 受入判定
  ではない — real-OS legs は引き続き sv#176 + rf#631 責務の
  external-owner。

## 観測境界（honest boundary）

- contractShape の `headless-contract-match` は「consumer oracle
  が要求する contract 面が正本に宣言済み」の照合であり、
  consumer 製品 Host での実走成功を意味しない — 実走 legs は
  全て external-owner のまま残す。
- dep 非汚染は consumer lock grep の実測半分 + 「test-only 宣言で
  production 非依存」の構造半分の 2 半分 — consumer が fixture
  を取り込む判断を妨げるものではない（Ask-first 事項は
  consumer 側判断）。実際に synvelis は sv#176 S1 で dev-dep
  経路の自主採用を行った（2026-11-25 再実測）—「強制しない」
  は「consumer が採用しない」ではなく「ruviform production DAG
  が fixture を要求しない」構造の性質であり、AudioLens 節
  （al#127 NotApplicable・lock 非含有）は実測半分でも不変に
  成立する。
- `test_editor` の `paramSurface`/`faultModes` pin は fixture の
  capability 宣言であり、sv#176 の PD case が当該面を消費する
  宣言ではない（消費判断は consumer Issue の責務）。
- docs index（`docs/**/README.md`）は未同期のまま —
  Coordinator が統合時に整備する（worker の lease 対象外）。
