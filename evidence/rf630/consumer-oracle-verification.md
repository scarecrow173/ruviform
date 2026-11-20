# RF-VTEST AC7 consumer-oracle slice — 検証記録

- Issue: `scarecrow173/ruviform#630`（AC7 consumer-oracle slice —
  fixture contract manifest + consumer oracle 対応表 + dep 非汚染
  evidence の ruviform 側完結 headless slice）
- OS: Windows (OS-L1) / toolchain: mise 管理 rust（workspace.lints
  正本）
- 記録日: 2026-11-21
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
  features 訂正）。

## 受入条件の照合（slice 確定版）

| 受入条件 | 結果 | 証拠 |
| --- | --- | --- |
| manifest が schema 名つき正本表として成立（全 7 class・README pin・二重正本化なし） | Passed | `manifest.json`（`rf630-fixture-contract/1`）+ 下記 cross-check で sha256/節 pin 全一致 |
| 対応表が 5 consumer を網羅 | Passed | `consumerOracleMap` 5 行 — sv#101/#127/#128/#176 + al#127(NotApplicable) |
| verdict 表の正直な区別 | Passed | contractShape=`headless-contract-match`（synvelis 4 行）・consumerRun=`external-owner`・residuals 4 件（output-event drain・popup harness・AC4 gate・consumer legs）に Pass を付けていない |
| dep 非汚染の実測記録 | Passed | 下記 grep 実測 — exit 1（非含有）を両 consumer で確認 |
| doc chain + evidence + README が揃う | Passed | 本記録・doc chain 4 文書・manifest・両 crate README diff |
| lease 境界の遵守 | Passed | `git status` — 変更は両 crate README + 新規 docs/evidence のみ。`.rs`/`tests/`/`mise.toml`/`docs/**/README.md`/`Cargo.*`/`.github/**`/他 repo（synvelis・audio-lens `git status` 両方 clean）に差分なし |

## deterministic evidence

正規 lane（`mise.toml` タスク）経由。OS: Windows (OS-L1)。

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | Passed — `.rs` 非回帰の非破壊確認（本 slice は `.rs` を触らないため形式確認の意味） |
| `mise exec -- prek run --all-files` | 0 | Passed — markdown/構成ファイル衛生の canonical（trailing whitespace・EOF・yaml・toml・line ending・BOM・private key・illegal names・actionlint 全緑） |
| `grep -n "ruviform-plugin-test\|ruviform-plugin-dock-test" ../synvelis/Cargo.lock ../audio-lens/Cargo.lock` | 1 | Passed — 両 consumer `Cargo.lock` に test fixture crate 名が非含有（exit 1 = no match = 期待値）。audio-lens `Cargo.lock:67` の `ruviform-audio-test` は #628 dev-dep 経路で fixture ではない |

grep 実測の対象（2026-11-21 時点）:

- synvelis `Cargo.lock` @ `d68f0e54c09763b93eb556353098c7a92532dfcf`
  （ruviform resolved rev `cd4213a1d665600f6adfefa0c3a90f2be9b4e4ce`）
- audio-lens `Cargo.lock` @ `fef857f684997449725e9f1bf38eca354a748d2c`
  （ruviform resolved rev `e4c9f6f0715da3ac479f8c437d21a021327e51da`）

## manifest ↔ crate README 整合照合

照合手順（再現可能）:

1. 両 README の sha256 を実測し manifest
   `authoritativeContract.sources[].readmeSha256` と突合する。
   - `crates/ruviform-plugin-test-vst3/README.md` =
     `ded84ae15239d63f622f5490a09134ce6852341171d60119cc25c1552dcd0829` → MATCH
   - `crates/ruviform-plugin-test-clap/README.md` =
     `6a252f63fe0e321c1c238200786d0bde8380efc1b9e6254054ba2d299ddebdc0` → MATCH
2. `fixtureClasses` が 7 class 全員を持つことを確認 → MATCH。
3. 各 `contractFaces.*.pin` を節 anchor（`## ` 見出しを kebab-case
   化）に解決し、class の ABI が持つ README 側に実在することを
   確認 → missing pin なし（vst3-only の `test_stateful_single`
   は VST3 README のみに解決）。
4. `consumerOracleMap` が 5 consumer 行を持つ → MATCH。

照合結果: PASS（exit 0）。

## oracle anchor の現状（read-only 実測）

探索記録どおり全 anchor が存在（consumer SHA は上記表）:

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
| `scarecrow173/synvelis#176` | headless-contract-match — 実 child HWND・popup・event token・CLAP floating が pin 済み | external-owner（sv#176 + ruviform#631） | PD-01..07 Real OS/Real Plugin 行は sv176 ledger でも NotRun |
| `scarecrow173/audio-lens#127` | external-owner（照合対象なし） | external-owner — NotApplicable | AL は plugin host 非提供 |

## 残件（本 slice 外 — external-owner/gate）

- CLAP output-event の host 側観測経路（`OutputEvents::void` →
  drain 配線）— external-owner・後続 slice
  （`scarecrow173/ruviform#253` の VST3 report wire 再利用見込み）。
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
- docs index（`docs/**/README.md`）は未同期のまま —
  Coordinator が統合時に整備する（worker の lease 対象外）。
