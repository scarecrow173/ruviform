---
id: "DESIGN-034XAEEhHWzlfr8ZBpkhgL"
type: "design"
status: "approved"
title: "Issue #627 POLICIES整合・着地済み改訂の照合記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-627-policies-revision-proposal.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  derived-by:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
  defers: []
  deferred-by: []
---

# Issue #627 POLICIES整合・着地済み改訂の照合記録

## Context

Issue #627 の当初スコープは「三 repo の `POLICIES.md` に対する仮想優先改訂**案**の確定とレビュー可能化」だった。しかし作業中に、三 repo すべてで同一趣旨の改訂が **既に main へ merge 済み** であることが判明した。本書はその事実を踏まえ、役割を「改訂案の提示」から「**着地済み改訂と #627 契約の post-landing 照合記録**」へ転換したものである。`POLICIES.md` 本文は本スライスでも一切変更しない。

本書の目的は3つ:

1. 三 repo の現行 `POLICIES.md` を旧版（Effective: 2026-11-13）からの差分として行単位で整理し、変更箇所・非変更箇所・repo 間残差分を記録する。
2. 適用済み改訂が `docs/designs/issue-627-virtual-first-verification-contract-design.md` の契約要件をどこまで満たし、何が残件かを対応付ける。
3. 旧物理 Required の Issue 群について、仮想環境へのゲート移管先と「仮想 Pass を物理 Pass に読み替えない」扱いを台帳として固定する。

## 比較 baseline（読んだ版の記録）

| repo | 旧 baseline（改訂前） | 新 baseline（改訂後・本照合の対象） | 取得方法 |
| --- | --- | --- | --- |
| ruviform | `POLICIES.md` @ `70dfb2a5`（PR #633 merge、改訂前 HEAD） | `POLICIES.md` @ `3b8b7322`（PR #634 merge、main HEAD） | `G:/repositry/audio-products/ruviform` ローカル checkout、`git show 3b8b7322` |
| synvelis | `POLICIES.md` @ `f12aa66`（PR #448 merge、改訂前） | `POLICIES.md` @ `8aa5cb0`（PR #449 merge）; その後 main は `b6a079a`（PR #450、無関係の player 変更）へ進行 | `G:/repositry/audio-products/synvelis`、`git fetch origin main` + `git show 8aa5cb0` |
| audio-lens | `POLICIES.md` @ `dafd895`（PR #211 merge、改訂前） | `POLICIES.md` @ `eb62440`（PR #212 merge、main HEAD） | `G:/repositry/audio-products/audio-lens`、`git fetch origin main` + `git show eb62440` |

三 repo の改訂 commit はいずれも `docs(policy): align virtual-first verification and Ruviform test harness` で、`POLICIES.md` のみを `+21/-10` で変更している。共通節（§1〜§5）の差分は三 repo で同一内容。差異はヘッダの repo 名・§6 固有節・audio-lens の行末スタイルのみ。

行単位の詳細な diff 記録は `evidence/rf627/policies-baseline-comparison.md` に保存した。

## 変更箇所の整理（旧 2026-11-13 版 → 新 2026-11-15 版）

### 変更された箇所（全 repo 共通）

| 箇所 | 旧 | 新 |
| --- | --- | --- |
| メタデータ `Effective` | `2026-11-13` | `2026-11-13（OS別検証・CI）、2026-11-15（仮想優先受入）` |
| §2 見出し | `OS別検証レベル（2026-11-13決定）` | `OS別検証レベル（2026-11-13決定・2026-11-15仮想優先改訂）` |
| §2 EXT-L3 定義行 | 「実際に使用する物理Audio/MIDI機器、実VST3/CLAP/AU等のPlugin/Editor、Native Dock、外部実環境と結合する機能の実測」 | 「本番OS Audio/MIDI API・実Plugin ABI/Editor・Native Dock・外部Provider Protocol等を通す結合検証。初回から仮想OS Endpoint・正規ABIのTest Pluginを標準利用し、物理機材・市販Pluginは通常必須としない」 |
| §2 OS 別表 Windows EXT-L3 | 「実際に提供・利用する機器・Plugin等に限り必須」 | 「実際に提供・利用するOS API・Plugin形式・Provider等に限り必須（仮想優先）」 |
| §2 Windows 補足 bullet 1 | 「Mock Pluginは実Plugin動作の代用にならない」等（物理前提の列挙） | Fake/Headless/Offscreen・Generic/Floating の代用禁止を維持しつつ「正規ABIを実装した実バイナリのTest Pluginはその対象形式のHost/Process/Editor結合検証に利用できる」を追加 |
| §2 Windows 補足 bullet 2 | 「実GUI/物理機器/実PluginをそのOSで利用する必要が生じた時点で…Requiredへ昇格」 | 「実GUI/本番OS Device API/実Plugin ABI等…Requiredへ昇格…**物理機材・市販Pluginの通常必須化は行わない**」 |
| §2.1（新設） | なし | 「仮想優先の受入と共通テスト基盤（2026-11-15決定）」7項目: 仮想優先の原則、OS-L1とOS API L3の分離、実Plugin ABIとMockの分離、実GUIとNative Dockの分離、物理適合性の限界、共通化とDogfooding、外部Provider |
| §3 NotRun bullet | 「実機がない、CI jobが未実行…は`NotRun`」 | 「Requiredの実OS Window/仮想Endpoint/実ABI Test Plugin環境がない…は`NotRun`…**物理機材がなくても、仮想方式で受入条件を満たしたCaseを機材不在のみで未達にしない**」 |
| §3 証拠 bullet | Repo/SHA・OS/Arch・Feature/テスト名・期待/観測・環境・CI Run URL/実機ログ | 上記に加え OS版・Case ID・要求状態と実行結果・**環境種別**・Driver/Endpoint/Plugin/Fixture の Version/Hash・**独立Oracle** を必須化 |
| §5 Windows L2/L3 bullet | 「実機受入は、Daily OS-L1 CIへの委譲では代替できない」 | 「実OS Window・本番OS API・実Plugin ABI受入は、Daily OS-L1 CIのFake/Headless成功で代替できない。Windows専用の仮想Endpoint/実Window/実ABI Test Pluginの手動・opt-in・定期実行レーンを使用できるが、RequiredのCaseがNotRun/Failなら未受入」 |

### repo 固有の変更（§6）

- **ruviform §6**: 「実Window・Native Plugin Dock・**本番WASAPI/WinMMを通す仮想Audio/MIDI Endpoint・実ABI Test Plugin**」への文言更新 + 「共通Test Runtime/仮想Endpoint/正規ABI Test Plugin/GUI Test Runner…をRuviform自身の汎用契約で構築し、独立Oracleと両Consumer回帰で自己検証バイアスを抑える」bullet を追加。
- **synvelis §6**: 「実MIDI」→「**OS MIDI**」への修正、「初回からRuviformの実OS仮想Audio/MIDI Endpointと正規ABIのTest Plugin/Native Editorを使用できる」+「Test RuntimeのPassを製品Passへ自動転用しない」bullet を追加。
- **audio-lens §6**: 「物理Audio/MIDI/実Plugin等を製品機能として提供しない限りそのEXT-L3を一律要求しない」→「提供する場合も物理機材・市販Pluginを通常必須とせず、対応する仮想L3で検証する」+「f64/長尺/PTS/欠測/CLI/可視化の期待値・性能はAudioLens独自Oracleで判定する。SynvelisのPassやRuviform Test Fixture自身の結果を流用しない」bullet を追加。

### 非変更箇所（維持された規範）

- §1 方針の適用と責務（repo 責務分離、共通基盤変更の独立評価）
- §2 OS-L1 / GUI-L2 の定義行、macOS/Linux の Deferred 維持、「3OSを設計対象から除外する判断ではない」
- §3 要求状態4値・実行結果4値の直交構造、「`Deferred`かつ`NotRun`は現在の必須受入を妨げない」「`Required`の`NotRun`/`Fail`は受入未達」「環境不在を技術的非対応と誤認しない」「Jobが定義・greenであることだけでは個別CaseのPassを示さない」
- §4 着手・CI・Issue・PR運用の全般
- §5 のローカル検証≠統合受入、fail-closed 集約、委譲済み扱い禁止

## repo 間残差分

| 差分 | 内容 | 評価 |
| --- | --- | --- |
| ヘッダ repo 名・Scope 行 | 各 repo 名義 | 意図的。問題なし |
| §6 固有節 | repo 別の Oracle 領域と固有ルール | 意図的。契約 §5 のマトリクスと整合 |
| audio-lens の行末スタイル | メタデータ行が `Status: Accepted\` のようにリテラル `\` で終わる（ruviform/synvelis は行末2スペース） | 改訂前からの既存差異。Markdown 上は `\` がそのまま表示される可能性がある軽微な体裁問題。**改訂による退行ではない**ため本スライスでは修正しない（POLICIES.md 非変更の制約）。残件として記録 |
| `environment_kind` の値表記 | ポリシー本文は「deterministic backend / OS仮想Endpoint / 実ABI Test Plugin / 実Window / 物理機材等」と和英混在の列挙。契約 schema は `deterministic-backend` / `virtual-os-endpoint` / `test-plugin` / `real-window` / `physical-device` の kebab-case を定義 | 意味論は一致。記録形式の統一は契約側で吸収。ポリシー本文の改訂は不要（「等」で開いた列挙） |

## 契約との適合確認

`docs/designs/issue-627-virtual-first-verification-contract-design.md` の要求に対する適用済みポリシーの充足状況:

| 契約要件 | ポリシー上の根拠 | 判定 |
| --- | --- | --- |
| 物理機材・市販 Plugin を初回必須にしない | §2.1「初回から仮想優先」「物理適合性の限界」 | 適合 |
| OS-L1/GUI-L2/EXT-L3 の3層維持 | §2 表（変更なし構造） | 適合 |
| Windows GUI-L2/EXT-L3 の Required 維持（利用機能に限る） | §2 OS 別表・§5 | 適合 |
| macOS/Linux GUI-L2/EXT-L3 の Deferred 維持 | §2 OS 別表・補足 bullet | 適合 |
| Required の NotRun/Fail は未受入（fail-closed） | §3・§5 | 適合 |
| 環境不在を Unsupported にしない | §3（変更後も維持） | 適合 |
| 仮想 Endpoint が本番 OS API を通過すること | §2.1「OS-L1とOS API L3を分離」 | 適合 |
| 実バイナリ Test Plugin と Mock/Fake の分離 | §2.1「実Plugin ABIとMockを分離」・§2 補足 | 適合 |
| Native Dock 実画素と Fake Surface の分離 | §2.1「実GUIとNative Dockを分離」 | 適合 |
| 独立 Oracle・Pass 非転移 | §2.1「共通化とDogfooding」・§6 各 repo | 適合 |
| Case schema の必須項目（case_id, environment_kind, resolved SHA, oracle, evidence 等） | §3 証拠 bullet が項目を列挙するが**形式的 schema としては未定義** | **残件**: 本契約設計 §2 が形式化を担う |
| 境界 Case の反証可能な定義 | ポリシーは原則を述べるが Case 単位の判定手順は未定義 | **残件**: 本契約設計 §3 が担う |
| ゲート移管台帳 | ポリシーは個別 Issue を参照しない（「恒久ポリシーには個別Issue番号を固定しない」） | **残件**: 本書 §移管台帳 が担う |

結論: ポリシー本文は仮想優先の**原則**を過不足なく反映しており、意味論上の残差分はない。残件は「原則 → 形式的 Case schema・境界判定手順・移管台帳への具体化」であり、いずれも本 doc-chain（契約設計・本書・test-spec・plan・task・IR）と `evidence/rf627/` が担う。追加の `POLICIES.md` 改訂は現時点で不要。

## ゲート移管台帳

旧物理 Required で NotRun/未達だった Case の移管先を以下に固定する。「移管」は受入責務の移動を意味し、旧 Case の `requirement` 状態を書き換えるものではない。状態変更は各 repo のレビューで行う。

凡例: 「仮想 Case」= `environment_kind` が `virtual-os-endpoint`/`test-plugin`/`real-window`/`deterministic-backend` の新規 Case。「物理維持」= `environment_kind=physical-device` として opt-in で残す。

| 旧 Issue | 旧物理 Required の内容 | 移管先（仮想 Case） | 旧 Case の扱い |
| --- | --- | --- | --- |
| RF #251 | 実機 Audio/WASAPI record・実デバイスでの検証 | 決定的 PCM/Clock/Fault（OS-L1）+ Windows 仮想 WASAPI Endpoint 経由の本番 Backend（EXT-L3, `virtual-os-endpoint`） | 物理機材依存部分は `physical-device` opt-in へ。仮想 Case の Pass で旧 Required を Pass にしない |
| RF #253 | 実 MIDI 機器経由の検証 | MIDI trace/clock（OS-L1）+ 仮想 WinMM MIDI Endpoint（EXT-L3, `virtual-os-endpoint`） | 同上 |
| RF #388 / RF #532 | Native Editor/DComp 実画素・実 Window での Dock 検証 | 実子 HWND/native editor + 実 Window/pixel/input Runner（GUI-L2 `real-window` + EXT-L3 `test-plugin`）。#532 の既知 DComp 不具合は「既知の不具合を残す」規定どおり未解決として追跡を継続 | Fake/Offscreen/Generic/Floating を Pass に読み替えない。物理依存部分のみ `physical-device` |
| RF #389 | 市販 Plugin での互換検証 | 正規 ABI の実バイナリ Test Plugin（VST3/CLAP）+ 独立 ABI Validator（EXT-L3, `test-plugin`） | 市販 Plugin 固有互換は `physical-device`（opt-in）。Test Plugin Pass ≠ 市販 Plugin Pass |
| RF #294 / RF #281 | 候補 tuple Gate・consumer 証拠契約 | fixture 版・三 repo 候補 SHA・OS/Case/Oracle 別証拠を Case schema（契約 §2）で記録。#294 の tuple Gate 構造は維持 | 証拠項目を schema に合わせて拡張。旧証拠の意味は変えない |
| SV #115 / SV #117 | 実 Audio/WASAPI・録音・モニタリングの製品受入 | 仮想 Endpoint 経由の PCM/RT 検証 + Synvelis 独立 Oracle（PCM 内容・RT no-alloc/no-block） | 物理 Direct Monitoring・実機録音品質は `physical-device` opt-in |
| SV #126 | 実 MIDI 機器経由の製品受入 | 仮想 WinMM MIDI Endpoint + 固定 MIDI event→録音 PCM の独立解析 | 同上 |
| SV #127 / SV #128 | 市販 Plugin（VST3/CLAP）での host 受入 | 実 ABI Test Plugin/Native Editor + Synvelis 独立 Oracle（plugin state/process・native dock） | 市販 Plugin 固有挙動は `physical-device` opt-in |
| SV #176 | Native editor/DComp 実画素 | 実子 HWND/native editor + 実 Window runner（GUI-L2） | #532 と同じく既知不具合は継続追跡 |
| SV #112 / SV #114 | GUI 手動確認項目 | 実 Window runner + Synvelis project/receipt scenario で自動化可能な範囲を Case 化 | 完全自動化できない UX 確認は `real-window` Case + 手動 receipt の併用 |
| SV #23 / SV #28 | （cross-consumer 連携・候補 gate 関連） | 三 repo 候補 SHA・resolved Ruviform SHA・fixture 版を揃えた tuple 証拠 | #294 と同じく証拠 schema 拡張のみ |
| AL #127 | AudioLens の継続追従・実環境依存項目 | 固定 audio fixture + GUI runner + 独立 f64/長尺/PTS/missing-data/CLI/visualization golden Oracle | 物理 capture 機材依存は `physical-device` opt-in。AudioLens は物理 Audio を製品機能として提供しない限り EXT-L3 一律要求なし（§6） |

### 旧物理 Required の扱い（明示ルール）

1. **自動書き換え禁止**: 対応する仮想 Case が `Pass` しても、旧物理 Required Case の `requirement`/`result` を自動的に `Pass`・`Deferred`・`NotApplicable` へ書き換えない。状態遷移は当該 repo の Issue/PR レビューで明示的に行う。
2. **二重計上の禁止**: 同一の物理依存受入条件に対し、仮想 Case の `Pass` と旧 Case の `Pass` を両方 Gate の充足証拠として計上しない。仮想 Case は「仮想環境での受入」、旧 Case は「物理環境での受入」として別個に管理する。
3. **opt-in の物理 Case**: `environment_kind=physical-device` の Case は、報告済み問題の原因調査または正式サポート宣言時にのみ `Required` とし、それ以外は `Deferred`/`NotApplicable` とする。初回受入・通常回帰の必須条件にしない。
4. **未保証範囲の記録**: 仮想環境で再現できない物理特性（Driver 差異・ADC/DAC・機種固有挙動・市販 Plugin 固有互換）は、製品のサポート範囲文書または Issue に「未保証」として記録する。

## Scope

### In Scope

- 三 repo `POLICIES.md` の旧版→新版の行単位差分整理と baseline SHA の記録
- 適用済み改訂と #627 契約の適合確認・残件の特定
- repo 間残差分（ヘッダ・§6・行末スタイル・用語表記）の記録
- 既存 Issue のゲート移管台帳と旧物理 Required の扱いルール

### Out of Scope

- `POLICIES.md` 本文の変更（三 repo とも改訂適用済み。残差分があっても本スライスでは修正しない）
- audio-lens の行末 `\` スタイルの修正（既存差異・体裁のみ。別 Issue 候補として記録）
- #628〜#631 の実装、Case schema の実装コード化
- 物理機材・市販 Plugin の購入・接続・個別適合 Case の実行

## Data and Control Flow

1. **Baseline 固定**: 各 repo の改訂 commit SHA を baseline として記録（本書「比較 baseline」節）。
2. **行単位照合**: `git show <commit> -- POLICIES.md` の diff を三 repo で比較し、共通変更・固有変更・非変更を分類。
3. **契約適合判定**: 設計契約の各要件をポリシー本文の該当節へ写像し、適合/残件を判定。
4. **移管台帳の確定**: 旧 Issue ごとに移管先 `environment_kind` と旧 Case の扱いを記録。
5. **レビュー**: 三 repo の関係者が本記録をレビューし、残件（schema 形式化・台帳の運用）が doc-chain として確定することを確認。

## Non-Functional Constraints

- **証拠の再現性**: 比較に使った commit SHA・パス・コマンドを `evidence/rf627/` に記録し、第三者が同じ diff を再現できること。
- **非変更の証明**: `POLICIES.md` を含む範囲外ファイルに差分がないことを `git status`/`git diff` で確認できること。
- **履歴の保全**: 旧 Case の状態を消さず、移管は「新しい仮想 Case の追加 + 旧 Case の扱いの明示」として記録する。

## Risks and Trade-offs

| リスク | 影響 | 緩和 |
| --- | --- | --- |
| 改訂が先に merge されたため「承認前ルール」の適用対象が曖昧になる | 仮想 Case 成功を旧 Required の Pass と誤読する余地 | 移管台帳の明示ルール 1・2 で「自動書き換え禁止」「二重計上禁止」を固定 |
| audio-lens の `\` 行末が将来の Markdown レンダリングで問題化 | ポリシー冒頭の体裁崩れ | 既存差異として記録し、POLICIES.md 非変更制約のため本スライスでは触らない。必要なら別 Issue |
| ポリシーが「等」で開いた列挙と schema の閉じた列挙が乖離 | 記録値の揺れ | 契約 schema の値を canonical とし、ポリシー側は列挙例として扱う（意味論は一致） |
| Synvelis main が照合後に進行（`b6a079a`） | baseline の鮮度 | `8aa5cb0` が policy 変更 commit であり `b6a079a` は無関係の player 変更と確認済み。`POLICIES.md` 自体に後続差分がないことを確認して記録 |

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| audio-lens `\` 行末スタイルの統一 | POLICIES.md 非変更制約 + 既存差異で退行ではない | 別 Issue 候補（軽微な体裁） | Markdown で `\` が可視化される可能性 |
| 旧物理 Required Case の実際の状態遷移 | 各 repo のレビュー・承認が必要で本スライスの権限外 | 各 repo の Issue/PR | 台帳が「扱いの提案」に留まり、実状態が更新されない |
| `physical-device` Case の個別定義 | 製品の正式サポート宣言時に必要 | 各製品 Issue の Activation 条件 | 特定機材問題が仮想 fixture に還元されない |

## Verification Notes

- [ ] `evidence/rf627/policies-baseline-comparison.md` に三 repo の行単位 diff と baseline SHA が記録されている
- [ ] 本書の変更/非変更箇所・残差分・適合判定・移管台帳が `docs/test-specs/issue-627-virtual-first-verification-contract.md` の Rule/Example で検証可能
- [ ] `git diff` で `POLICIES.md`・範囲外ファイルに変更がないことを確認
- [ ] `review-protocol` で blocker なし
