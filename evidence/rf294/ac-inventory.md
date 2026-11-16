# Issue #294 — 受入条件（AC）棚卸し表

- Issue: https://github.com/scarecrow173/ruviform/issues/294
- 作成: Close 判定 `COVERED` の条件処理（close-out 簿記）、2026-11-16
- 判定語彙: POLICIES.md の `Required/Deferred/NotApplicable` ×
  `Pass/Fail/NotRun/Unsupported`。実 OS/実機・GUI・実 plugin ABI の
  成功は一切主張しない — deterministic-backend（OS-L1）・証拠記録層・
  CI 配線の実績のみを verdict 根拠とする。
- AC 行番号は Issue 本文「### 受入条件」チェックリストの出現順（10 行）。
- 根拠列の run ID は GitHub Actions run、PR 番号は merge 済み slice のもの。

## Slice 対応関係

| Slice | PR | merge commit | 範囲 |
| --- | --- | --- | --- |
| daily candidate-tuple trigger | `scarecrow173/ruviform#500` | `85ac831a` | `decide.mjs --mode tuple`・daily-ci `decide-consumer-verify` job・consumer-verify `RUVIFORM_REV` pin |
| tracker 配線 | `scarecrow173/ruviform#534` | `036c60e4` | `workflow_run` trigger 拡張・ci-issue-tracker scope 別 reconcile・失敗追跡 |
| virtual-first evidence schema v2 | `scarecrow173/ruviform#639` | `c6e4f334` | `ci/consumer-verify/` Case schema v2 additive 拡張・`requirement`×`outcome` 直交・yml 全記録経路 schemaVersion 2 |
| consumer oracle lanes | `scarecrow173/ruviform#700` | `3d876600` | 両 verify job への rf628 oracle lane（probe→run→record 3 分岐・file 粒度 verdict・`resolved_ruviform_sha`/`oracle`/`fixture_version` 記録） |
| close-out 簿記（本 PR） | 本 PR | — | 本棚卸し・実 dispatch 実値の反映・stale owner 参照の candor 修正 |

## AC 棚卸し

| # | 受入条件（Issue 転記） | verdict | 理由 | 根拠 |
| --- | --- | --- | --- | --- |
| 1 | **現行Virtual-firstのOwner/受入整合:** 共通Fixture #628、OS仮想Endpoint #629、正規ABI Test Plugin #630、GUI Runner #631のCase/Version/Artifact Hash、3repo SHA tuple、Windows利用L2/L3と3OS L1の独立Oracleを候補SHAに結合し、AL f64/CLI・SV PCM/RT/Project/Plugin/GUIを独立判定する。通常のDaily Full CIと専用opt-in環境を分離 | **Deferred**（owner 付き） | #628 共通 Fixture の oracle lane 結合は run `43907984482` で実走 Pass（AC6 行）。残る #629（OS 仮想 Endpoint）・#630（正規 ABI Test Plugin）・#631（GUI Runner）の Case/Version/Artifact Hash 結合は各 gate が未実装/未実走で NotRun — owner は当該 Issue。opt-in 専用 lane 分離は workflow 側に opt-in 経路なしのまま owner 領域へ委譲 | consumer-verify run `43907984482`。`scarecrow173/ruviform#629`/`#630`/`#631`（OPEN — 各 virtual-first gate owner） |
| 2 | **Virtual-first移行（2026-11-15現行POLICIES適用）:** 新共通Fixtureを各Consumerの独立OS-L1とWindows仮想L2/L3に紐付け、候補SHA tuple・Feature/Case/Device/Plugin version・Evidence URL/Runを収集。既存CI変更検知Daily・手動Full CIを再利用しWindows実GUI/OS仮想Endpointの専用Laneは別途記録 | **Deferred**（owner 付き） | 証拠 schema v2（Case/requirement/outcome/resolved SHA/fixture version）は PR `scarecrow173/ruviform#639` で導入済み、#628 fixture は oracle lane 経由で独立 OS-L1 に紐付済み。Windows 仮想 L2/L3（Device/Plugin/GUI）への紐付は #629〜#631 の実装待ちで NotRun — AC1 と同一 owner 群 | `evidence/rf294/verification.md`・`verification-consumer-oracle-lanes.md`。`scarecrow173/ruviform#629`/`#630`/`#631` |
| 3 | PR #500/#534のDaily SHA変更検知/Workflow Run追跡基盤とconsumer-verify run 42937524742の3repo SHA一致・AL/SV独立結果を証拠として保持 | **Pass** | Issue AC 行は checked。PR #500（merge `85ac831a`）で tuple 変更検知、PR #534（merge `036c60e4`）で workflow_run/reconcile 追跡を統合済み。run `42937524742` の 3repo SHA 一致・AL/SV 別集計は Issue 本文に証拠として保持済み（1 tuple の実走例であり全候補の Pass ではない旨も明記済み） | PR `scarecrow173/ruviform#500`・`#534`。consumer-verify run `42937524742`（AL 19 lanes・SV 21 lanes、shaMatch=true） |
| 4 | 変更時Daily・手動Full CIの代表SHA tuple（Ruviform/AL/SV）で実行/重複回避/既成功skip/失敗retry/credential/Artifactを検証する | **Pass** | daily tuple trigger が実稼働: daily-ci schedule run `43783844680` @ `cbcc8343` が `decide-consumer-verify` で tuple 変更を検知し consumer-verify dispatch run `43783883894`（workflow_dispatch・success）を発火。重複回避・skip・retry の決定論理は `ci/daily/decide.mjs --mode tuple` + fixture tests（PR #500 時点 175 件）で pin、失敗追跡は ci-issue-tracker（PR #534）が担う。credential 経路は `CROSS_REPO_DEPLOY_KEY` 有無で SSH/anonymous HTTPS を分岐済み | daily-ci run `43783844680`（schedule・success）→ consumer-verify dispatch run `43783883894`（workflow_dispatch・success）。`.github/workflows/daily-ci.yml`・`.github/workflows/consumer-verify.yml` |
| 5 | Windows/macOS/Linuxの既存CI Native Build/Link/Headless/CLI/RT/必要Featureを、それぞれの対象リポジトリrevisionと実行URLで参照し、OS別未実行・skipをPassに変換しない | **Pass** | consumer-verify 証拠行は lane 単位で consumer 名・resolved SHA・run URL・outcome を記録し、`ubuntu` headless で実施不能な lane（macos-os・gui-real-window・plugin-native-dock）は owner・理由付き `unsupported`/`not_run` 固定行として分離 — skip を pass にしない構造が schema v2 と yml contract test で pin 済み | `evidence/rf294/verification.md`（schema v2 test 82 pass）。`.github/workflows/consumer-verify.yml` の capability-absent 固定行（`audiolens-macos-os`/`synvelis-macos-os` 等）。run `43907984482` artifact の lane 行 |
| 6 | AL f64/長尺/CLI/Export・SV Project/RT/Plugin Model等の**Headless独立Oracle**を同候補tupleで判定し、片方のPassを他方へ流用しない | **Pass**（file 粒度・headless OS-L1 範囲） | consumer-verify run `43907984482`（対象 SHA `615cf159`・oracle lane 配線 PR `scarecrow173/ruviform#700` 含む）で両 consumer の rf628 oracle lane が同一 `resolved_ruviform_sha`・同一 `fixture_version` で実走 Pass。AL `audiolens-rf628-oracle`=pass（7 test）、SV `synvelis-rf628-oracle`/`synvelis-rf628-oracle-rt-alloc`=pass×2 — consumer・oracle 別 lane 行で別集計、片方の結果の流用なし。per-case（test fn 粒度）emitter は consumer repo 側の後続 gate として残る | consumer-verify run `43907984482`（workflow_dispatch・全 job success）。artifact `consumer-evidence-audiolens.json`（`fixture_version=ruviform-audio-test-0.1.0@615cf159...`・outcome=pass）。artifact `consumer-evidence-synvelis.json`（outcome=pass×2）。詳細は `evidence/rf628/ac-inventory.md` AC6 行 |
| 7 | リリースがL2/L3を要求する実用途が発生した場合のみ、そのOS/Format/Deviceの追加受入Gateを別Ownerへ連結。既存CI Workflowを新設・重複運用しない | **Pass** | consumer-verify.yml は workflow_dispatch + daily tuple trigger のみで新規 workflow を増設していない。Windows L2/L3 相当 lane（gui-real-window・plugin-native-dock・macos-os）は `unsupported` 固定行として owner Issue へ連結済み（#631/#630/#388/#389・consumer 側 owner）。opt-in/実機 lane は別 owner gate として外部化され、本 workflow の pass 条件に混ざらない | `.github/workflows/consumer-verify.yml`（capability-absent 固定行・dispatch 専用設計）。`docs/development-environment.md` の lane 説明 |
| 8 | **Windows L2/L3の必須範囲:** WindowsでAudioLens/Synvelisが提供するGUI/Audio・MIDI/Plugin機能のL2/L3を、各製品の実行証拠と候補SHA tupleに結合する。実行しない/使わない機能はNotRequiredを明示し、未実施の必要機能をCI Headless成功でPassにしない | **Deferred**（owner 付き） | Windows L2/L3 の実走自体は headless ubuntu CI では Unsupported/NotRun のまま正直に維持（AC5/AC10）。実走 gate の owner は RF #629（OS 仮想 Endpoint）・#630（正規 ABI Test Plugin）・#631（実 Window Runner）・#388/#389（Native Dock、#532 DComp が open Blocker）・consumer 側 SV#176 — 本 Issue 側の記録行は owner 参照済みで残件は各 gate の実装・実走待ち | `.github/workflows/consumer-verify.yml` の `unsupported`/`not_run` 固定行。`scarecrow173/ruviform#629`/`#630`/`#631`/`#388`/`#389`/`#532`・`scarecrow173/synvelis#176`（全て OPEN） |
| 9 | **macOS/Linux L1のみ必須:** 既存CIのNative Build/Link/Headless/必要Feature/RTの証拠を保持し、実GUI/実Device/実Pluginは使用開始時に追加する | **Pass** | 両 consumer lane は ubuntu headless の既存 mise task（check/test/test-rt/UI headless 系）を resolved SHA 付きで実走し証拠 artifact へ記録。macOS 実測・実 GUI/実 Device/実 Plugin は `macos-os`/`gui-real-window`/`plugin-native-dock` 固定行で `unsupported` 分離 — 「L1 実走 + L2/L3 は未実施の正直記録」の構造が維持されている | run `43907984482` の両 verify job（AL/SV 各 lane 実走）。`consumer-verify.yml` の `macos-os` 固定行（正式受入は別途 macOS 実測必須と明記） |
| 10 | **受入の真偽:** Windowsの使用機能に必要な実走はNotRun/FailのままPass扱いしない。未使用OS/Format/機器はNotRequired/Deferredを区別し、CI Workflowは増設しない | **Pass** | schema v2 が `requirement`（Required/Deferred/NotApplicable）と `outcome`（pass/fail/not_run/unsupported/skipped）を別軸で強制し、全終端経路で証拠 JSON を生成（`not run` 全 lane は workflow 失敗＝証拠ゼロを緑にしない）。lane 理由文字列に owner・再実行条件を記録する candor 運用を本棚卸しでも継続（stale owner 参照を #313/#314→#631・AL#166→#127・SV#175→#176/#177 へ修正済み） | `ci/consumer-verify/`（schema v2・evidence.mjs finalize）。`evidence/rf294/verification.md`・本 PR の candor 修正差分 |

## 残件 → owner 対応表（本 Issue Close 後の追跡先）

| 残件 | 状態 | owner |
| --- | --- | --- |
| per-case（test fn 粒度）verdict emitter — consumer test が機械可読 verdict を出力しないため file 粒度 lane 行に留まる | Deferred | `scarecrow173/audio-lens#127` / `scarecrow173/synvelis#115`（consumer repo 側 emitter gate） |
| Windows 実 OS lane（GUI-L2 実 Window・OS 仮想 Endpoint・実 ABI Test Plugin・実 Native Dock 画素） | NotRun/Unsupported | `scarecrow173/ruviform#629`（OS 仮想 Endpoint）・`#630`（正規 ABI Test Plugin）・`#631`（実 Window Runner）・`#388`/`#389`（Native Dock、`#532` DComp が open Blocker） |
| consumer 側通常 View/Domain headless fixture・GUI contract | Deferred（not_run） | `scarecrow173/audio-lens#127` / `scarecrow173/synvelis#176`・`#177` |
| SV 側製品 Oracle の umbrella（PCM/RT/Project/Recording/MIDI domain） | Deferred | `scarecrow173/synvelis#28`・`#115`・`#126`（MIDI domain 集計の umbrella 委譲先）・`#176`/`#177` |
| Cross-Repo 互換契約・将来候補 SHA tuple の継続採否運用 | 継続運用 | `scarecrow173/ruviform#219`（長期横断 Owner） |
| opt-in 専用環境と通常 Daily Full CI の分離運用 | Deferred | virtual-first gate 群（#629〜#631）の実装と併せて lane 化を判断 |
| `consumer-verify-tuple` contract v1→v2 bump | Deferred | `ci/daily`・`daily-ci.yml`・docs 連動の後続 slice（`evidence/rf294/verification-consumer-oracle-lanes.md` §3 記録どおり継続 Deferred） |

## candor 修正（本 PR で実施）

- `.github/workflows/consumer-verify.yml` — lane reason の stale owner 参照を修正:
  `#313/#314`（CLOSED）→ `#631`、`AL #166`（CLOSED）→ `AL #127`、
  `SV #175-177`（#175 CLOSED）→ `SV #176/#177`。
  `plugin-native-dock` 行には正規 ABI Test Plugin owner として `RF #630` を追加
  （lane が「実 plugin binary」を要求するのに対し virtual-first 方針の当該 owner
  が未記載だったため — Issue #294 本文の virtual-first owner 整理に基づく）。
- `docs/development-environment.md` — consumer-verify 節の
  `AL #166/SV #175` → `AL #127/SV #176/#177`（owner candor のみ、説明文は不変）。
- `evidence/rf294/verification.md`・`verification-consumer-oracle-lanes.md` —
  §3「実 runner dispatch: NotRun」を run `43907984482` の実値で更新。

## 検証実行記録

- `evidence/rf294/verification.md`（schema v2 slice・`mise run test-ci-consumer-verify` 82 pass ほか）
- `evidence/rf294/verification-consumer-oracle-lanes.md`（oracle lanes slice・84 pass ほか）
- 本 PR は docs/証拠簿記のみ。`.github/workflows/consumer-verify.yml` の変更は
  lane reason の文字列修正のみで CI トリガ・job 構造・lane 実行内容は不変。
