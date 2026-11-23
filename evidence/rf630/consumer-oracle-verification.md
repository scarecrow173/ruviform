# RF-VTEST AC7 consumer-oracle slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（AC7 consumer-oracle slice —
  fixture contract manifest + consumer oracle 対応表 + dep 非汚染
  evidence の ruviform 側完結 headless slice）
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints
  正本）
- 記録日: 2026-11-21（初回）・2026-11-24（re-pin — #842/#885 の
  README drift による pin 再照合。[re-pin（2026-11-24）](#re-pin2026-11-24)
  節に全記録）
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
| manifest が schema 名つき正本表として成立（全 7 class・README pin・二重正本化なし） | Passed | `manifest.json`（`rf630-fixture-contract/1`）+ 下記 cross-check で sha256/節 pin 全一致（初回 2026-11-21・re-pin 2026-11-24 の両時点で実測） |
| 対応表が 5 consumer を網羅 | Passed | `consumerOracleMap` 5 行 — sv#101/#127/#128/#176 + al#127(NotApplicable) |
| verdict 表の正直な区別 | Passed | contractShape=`headless-contract-match`（synvelis 4 行）・consumerRun=`external-owner`・residuals は open 3 件（popup harness・AC4 gate・consumer legs）+ resolved 1 件（clap-output-event-drain・#842 解消）に Pass を付けていない |
| dep 非汚染の実測記録 | Passed | 下記 grep 実測 — 初回・re-pin 両時点で exit 1（非含有）を両 consumer で確認 |
| doc chain + evidence + README が揃う | Passed | 本記録・doc chain 4 文書・manifest・両 crate README |
| lease 境界の遵守 | Passed | `git status` — 変更は manifest・本記録・spec doc のみ。`.rs`/`tests/`/`mise.toml`/`docs/**/README.md`/`Cargo.*`/`.github/**`/他 repo（synvelis・audio-lens `git status` 両方 clean）に差分なし |

## deterministic evidence

正規 lane（`mise.toml` タスク）経由。OS: Windows (OS-L1)。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Passed — `.rs` 非回帰の非破壊確認（本 slice は `.rs` を触らないため形式確認の意味）。2026-11-21 実施・2026-11-24 再実施 |
| `mise exec -- prek run --all-files` | 0 | Passed — markdown/構成ファイル衛生の canonical（trailing whitespace・EOF・yaml・toml・line ending・BOM・private key・illegal names・actionlint 全緑）。2026-11-21 実施・2026-11-24 再実施 |
| `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" ../synvelis/Cargo.lock ../audio-lens/Cargo.lock` | 1 | Passed — 両 consumer `Cargo.lock` に test fixture crate 名が非含有（exit 1 = no match = 期待値）。audio-lens `Cargo.lock` の `ruviform-audio-test` は #628 dev-dep 経路で fixture ではない。初回 2026-11-21・re-pin 2026-11-24 ともに exit 1 |

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

### re-pin 後（2026-11-24 — 現行記録）

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
| `scarecrow173/synvelis#176` | headless-contract-match — 実 child HWND・popup・event token・CLAP floating が pin 済み。#885 の Fault carrier 面も pin 追加（消費は sv#176 判断） | external-owner（sv#176 + ruviform#631） | PD-01..07 Real OS/Real Plugin 行は sv176 ledger でも NotRun |
| `scarecrow173/audio-lens#127` | external-owner（照合対象なし） | external-owner — NotApplicable | AL は plugin host 非提供 |

## 残件（本 slice 外 — external-owner/gate/resolved）

- ~~CLAP output-event の host 側観測経路（`OutputEvents::void` →
  drain 配線）~~ — **resolved**: `scarecrow173/ruviform#842`
  （`116c1cb2`）で `drain_output_reports` canonical drain として
  実装済み。解消 evidence:
  `evidence/rf630/clap-output-drain-verification.md`。
- editor popup の real-HWND host harness / L1 editor 面 —
  external-owner・`scarecrow173/ruviform#631` real-dock lane 責務。
- AC4 実 Window/DComp 既知 Fail 再現・解消判定 —
  gate・`scarecrow173/ruviform#532` 着地待ち（既知 soft）。
- consumer 実走 legs — external-owner・各 consumer Issue
  （sv#101/#127/#128/#176）の AC/lane 責務。al#127 は
  NotApplicable で実走 leg なし。

## 観測境界（honest boundary）

- contractShape の `headless-contract-match` は「consumer oracle
  が要求する contract 面が正本に宣言済み」の照合であり、
  consumer 製品 Host での実走成功を意味しない — 実走 legs は
  全て external-owner のまま残す。
- dep 非汚染は「現状非含有」の実測 + 「test-only 宣言で
  production 非依存」の構造記録の 2 半分 — consumer が今後
  fixture を取り込む判断を妨げるものではない（Ask-first 事項は
  consumer 側判断）。
- `test_editor` の `paramSurface`/`faultModes` pin は fixture の
  capability 宣言であり、sv#176 の PD case が当該面を消費する
  宣言ではない（消費判断は consumer Issue の責務）。
- docs index（`docs/**/README.md`）は未同期のまま —
  Coordinator が統合時に整備する（worker の lease 対象外）。
