---
id: "DESIGN-034XAEBuSiPGfciAlww2l3"
type: "design"
status: "approved"
title: "Issue #627 仮想優先の3repo検証契約"
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
        path: "docs/designs/issue-627-virtual-first-verification-contract-design.md"
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
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  blocks:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-627-policies-revision-proposal.md"
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

# Issue #627 仮想優先の3repo検証契約 — 設計

## Context

Issue #627 は、Ruviform / Synvelis / AudioLens の三 repo が、初回から物理 Audio/MIDI 機材・市販 Plugin を前提とせずに受入を進められるようにする横断検証契約を定義する。本設計文書はその契約の実体——Gate taxonomy、Case schema、境界 Case、独立 Oracle 規則、受入マトリクス、Dogfooding 責務分担——を固定する。

前提として、三 repo の `POLICIES.md` は 2026-11-15 に仮想優先改訂が適用済みである（Ruviform PR #634 `3b8b7322`、Synvelis PR #449 `8aa5cb0`、AudioLens PR #212 `eb62440`）。改訂本文は `environment_kind`・独立 Oracle・証拠項目を要求するが、これらを満たす形式的な Case schema は未定義であり、本設計がその形式化を担う。改訂と契約の行単位照合・残差分・ゲート移管台帳は `docs/designs/issue-627-policies-revision-proposal.md` に記録する。

本契約は #628（Test Runtime）・#629（仮想 OS Endpoint）・#630（Test Plugin）・#631（GPUI Scenario Runner）の実装が従うべき入力であり、`docs/specs/issue-219-consumer-compat-contract.md` の Consumer 互換契約と `docs/specs/issue-294-cross-consumer-candidate-gate.md` の候補 tuple Gate・証拠契約の上に乗る。

## Scope

### In Scope

- Gate taxonomy: OS-L1 / GUI-L2 / EXT-L3 の維持と、EXT-L3 内の仮想適合性を `environment_kind` で区別する規則
- 検証 Case schema: 要求状態と実行結果の直交、必須記録項目、証拠の最低要件
- 3つの境界 Case（Device 仮想化 vs 本番 OS-API、Plugin Fake vs 実 ABI バイナリ、Native Dock 実画素 vs Fake Surface）の反証可能な定義
- 独立 Oracle 規則と repo 別の所有権（AudioLens / Synvelis / Ruviform）
- 三 repo 受入マトリクス: 各 repo の機能領域ごとに、どの環境種別のどの Case が独立 Oracle を持つか
- Dogfooding 責務分担と OS 依存アダプタ境界
- 明示ルール: 仮想 Case の成功を物理環境依存の受入条件の Pass として扱わないこと、Required の NotRun/Fail は未受入のまま残すこと

### Out of Scope

- `POLICIES.md` 本文の変更（三 repo とも改訂適用済み。本設計は契約の形式化のみ）
- #628〜#631 の実装（本設計はそれらが従う契約を定義する）
- 物理機材・市販 Plugin の個別適合 Case の定義（製品が正式サポートを宣言した時点で各 Issue が定義する）
- macOS/Linux の GUI-L2/EXT-L3 Required 昇格（利用要件発生まで Deferred 維持）

## Components and Boundaries

### 1. Gate taxonomy と `environment_kind`

検証レベルは既存の3層を維持する:

| Level | 内容 | 初回受入での位置づけ |
| --- | --- | --- |
| OS-L1 | ネイティブ build/link、実行可能な unit/integration/headless/RT/CLI テスト、製品固有の精度・整合性 Oracle | 三 OS（Windows/macOS/Linux）で Required |
| GUI-L2 | 実 OS Window、実ユーザー入力、フォーカス、表示/画素、DPI/IME（該当する場合） | Windows で実際に提供・利用する機能は Required。macOS/Linux は利用要件発生まで Deferred |
| EXT-L3 | 本番 OS Audio/MIDI API、実 plugin ABI/editor/native dock/provider protocol | Windows の利用機能は Required。標準環境は仮想 Endpoint + 実バイナリ Test Plugin。物理機材・市販 Plugin は通常 Required ではない |

各検証 Case は `environment_kind` で環境の実体を区別する（OS-L1 では `deterministic-backend` が中心、GUI-L2 では `real-window`、EXT-L3 では `virtual-os-endpoint`/`test-plugin`/`physical-device` が中心）。契約が定める値:

| `environment_kind` | 実体 | 何を証明するか | 何を証明しないか |
| --- | --- | --- | --- |
| `deterministic-backend` | 決定的テスト Backend、生成データ、VirtualClock、fault injection | OS-L1 の契約・タイミング・故障時挙動 | OS API 経路、実デバイス挙動 |
| `virtual-os-endpoint` | 本番 WASAPI/WinMM 等の API を実際に通過する仮想 Endpoint | OS API 境界の正しさ、format/channel/clock/buffer 契約 | 物理ドライバ、ADC/DAC、機種固有挙動、直結モニタリング |
| `test-plugin` | サポートする VST3/CLAP ABI を実装した自前の実バイナリ Test Plugin とその Native Editor | host/processor/editor ABI、state 往復、native dock 手順 | 市販 Plugin 固有の互換性、エコシステム内の実在 Plugin 挙動 |
| `real-window` | 実 OS Window + 実入力 + 実画素証拠を取る GUI Runner | GUI-L2 全般、native dock の実画素・フォーカス・リサイズ・入力 | 仮想 Endpoint 側の音声内容 |
| `physical-device` | 実 Audio/MIDI 機材・市販 Plugin | 当該機材/Plugin を正式サポート対象として宣言した Case のみ | 通常の初回受入では使用しない |

`physical-device` は「報告済み問題または正式サポート宣言がある場合の opt-in 調査」に限定し、その結果は再現可能な回帰として仮想 fixture へ還元することを推奨する（Issue #627 のスコープ外事項として維持）。

### 2. Case schema（要求状態 × 実行結果の直交）

各検証 Case は以下の2軸を直交させて記録する。混同を防ぐため、要求状態と実行結果を同一フィールドに畳み込んではならない。

**要求状態** `requirement`: `Required` / `Deferred` / `NotApplicable`

**実行結果** `result`: `Pass` / `Fail` / `NotRun` / `Unsupported`

直交の規則:

- `Required` かつ `NotRun` または `Fail` は**未受入**であり、環境不在を理由に `Unsupported` へ書き換えてはならない。`Unsupported` は「その環境で技術的に実行不能であることが文書化されている」場合に限る。
- `Deferred` の Case は実行しても Gate 判定に算入しない。結果を記録すること自体は妨げない。
- `NotApplicable` はその repo/OS/機能の組合せで要求自体が存在しないことを示す。環境の失敗を隠すために使ってはならない。
- 仮想 Case の `Pass` は、同じ `requirement` を持つ**旧物理 Required Case の `Pass` として計上してはならない**（後述の移行規則）。

**必須記録項目**（全 Case 共通）:

| 項目 | 内容 |
| --- | --- |
| `case_id` | repo 内で一意な Case 識別子 |
| `feature` | 検証対象の製品機能 |
| `level` | `OS-L1` / `GUI-L2` / `EXT-L3` |
| `environment_kind` | 上表のいずれか |
| `os` / `arch` | 実行 OS とアーキテクチャ |
| `device` / `endpoint` | 仮想 Endpoint 名・物理デバイス識別（該当する場合） |
| `plugin` / `abi` | Test Plugin / 市販 Plugin の識別子と ABI 版（該当する場合） |
| `sdk` | OS SDK / toolchain 版 |
| `repo` / `sha` | 検証を実行した repo と commit SHA |
| `resolved_ruviform_sha` | Consumer が解決した Ruviform の SHA（Cargo.lock 等の実解決値。宣言値ではない）。Ruviform 自身の Case では `sha` と同一値を記録する |
| `oracle` | 独立 Oracle の識別子（下記 §4） |
| `expected` / `observed` | 期待値と観測値（独立 Oracle が生成する期待値と、被検証系の観測値を分離して記録） |
| `evidence` | 証拠 URL または CI RunID。CI job の存在・緑化だけでは Case の `Pass` にならない |
| `fixture_version` | fixture/driver/plugin の版または hash（再現性のため） |

**証拠の最低要件**: `evidence` は「何を実行し、どの SHA で、どの Oracle が何を比較したか」を第三者が再確認できる粒度でなければならない。`expected` と `observed` を同一実装が生成する記録は独立 Oracle 規則（§4）に違反する。

### 3. 境界 Case（反証可能な定義）

「仮想優先」は「fake で済ませる」ことではない。以下の3境界を、Fail になり得る Case として定義する。いずれも「境界を越えたか」を証拠で示す必要がある。

**境界 A: Device 仮想化 vs 本番 OS-API 通過**

- 仮想 Endpoint（`environment_kind=virtual-os-endpoint`）は、ユーザ空間の fake ではなく、製品が実際に使う本番 API（Windows なら WASAPI/WinMM の render/capture/MIDI 経路）を**実際に通過**しなければならない。
- 反証 Case: Endpoint が見える・列挙される・format を返すだけで、製品の audio graph がその Endpoint を source/sink として open・stream していない記録は `virtual-os-endpoint` の証拠として無効。`deterministic-backend` へ格下げするか `NotRun` とする。
- 仮想 Endpoint が保証しないもの（物理ドライバ・ADC/DAC 解析遅延・直結モニタリング・機種固有挙動）を Pass の根拠に含めてはならない。これらを要求する Case は `environment_kind=physical-device` の別 Case として残す。

**境界 B: Plugin Fake vs 実バイナリ ABI**

- `environment_kind=test-plugin` の Case は、サポートする VST3/CLAP ABI を**実バイナリとして実装**した Test Plugin を host が実際に load し、processor/editor/state の各経路を通過した記録を要求する。
- mock/fake の Plugin（host 内部で振る舞いを再現するだけのオブジェクト）は ABI 検証の代用にならない。旧ポリシーの「Mock Plugin は実 Plugin 動作の代用にならない」はこの境界について維持される。
- 反証 Case: ABI validator が export/interface を実際に解決していない、editor window が生成されていない、state blob の往復が行われていない記録は `test-plugin` の `Pass` として無効。

**境界 C: Native Dock 実画素 vs Fake Surface**

- GUI-L2 / plugin editor の native dock 検証は、実子 HWND/native editor を実 Window 内に dock し、実画素・実入力・フォーカス・リサイズを観測する Runner（`environment_kind=real-window` + `test-plugin` の組合せ）を要求する。
- headless・offscreen・generic/floating surface の成功は実 GUI の代用にならない（旧ポリシー規定を維持）。
- 反証 Case: スクリーンショットが renderer のオフスクリーンバッファのみ、入力が synthetic event で OS の focus/IME 経路を通っていない、dock が floating/独立 Window である記録は `real-window` の証拠として無効。

### 4. 独立 Oracle 規則

production と同じ実装が自分自身の正解値を生成してはならない。Oracle は被検証系と独立した実装・データ・解析器でなければならない。

**共通 Oracle 手段**: 既知数学波形（正弦波・インパルス・既知 PCM 列）、固定 MIDI event 列、外部 ABI Validator、独立 PCM 解析器（被検証系と別実装の解析コードまたは既知の参照データとの比較）。

**repo 別の所有権**:

| repo | 独立 Oracle が所有する領域 | 具体例 |
| --- | --- | --- |
| AudioLens | f64 精度、長尺処理、PTS、missing/invalid data、CLI、visualization/golden 解析 | 既知波形の f64 PCM 列との bit 近傍比較、長尺 fixture の PTS 単調性・欠損検出、CLI 出力の golden 比較 |
| Synvelis | PCM 内容、RT 制約（no-allocation/no-blocking）、Project/Track/Clip/Undo/Save、録音・モニタリング、MIDI、plugin state/process、GUI/native dock | 固定 MIDI event に対する録音 PCM の独立解析、RT 違反検出、Project 往復の独立 schema 検証、Test Plugin state blob の独立 decode |
| Ruviform | 共通 fixture・Test Runtime・仮想 Endpoint・Test Plugin・GUI Runner の契約と再利用可能な検証機構 | fixture の版管理、Endpoint の format/clock 契約、ABI Validator、pixel/receipt 証拠の形式 |

**Pass の非転移**: Ruviform の fixture/Runtime の成功、Synvelis の Oracle 結果、AudioLens の Oracle 結果は相互に転移しない。ある repo の Pass が別 repo の Case の Pass 根拠になることはない。三 repo の候補 tuple Gate（#294 系）は repo 別 SHA と resolved Ruviform SHA を分離して記録する。

### 5. 三 repo 受入マトリクス

機能領域ごとに「どの `environment_kind` のどの Case が、どの独立 Oracle で検証されるか」を以下に固定する。表は「初回受入で Required とする組合せ」を示し、物理機材依存の行は持たない。

| 機能領域 | OS-L1 | GUI-L2 | EXT-L3 | 独立 Oracle |
| --- | --- | --- | --- | --- |
| Audio capture/playback graph | `deterministic-backend` + `virtual-os-endpoint` | — | `virtual-os-endpoint` | SV: PCM 内容・RT 制約 / AL: f64・PTS・長尺 |
| MIDI I/O | `deterministic-backend` | — | `virtual-os-endpoint`（仮想 WinMM 等） | SV: 固定 MIDI event→録音 PCM 解析 |
| Plugin host（load/process/state/editor） | `deterministic-backend` | `real-window` + `test-plugin` | `test-plugin` | SV: ABI/state/native dock / AL: f64 解析が関わる場合は AL Oracle |
| GUI 一般（window/input/focus/DPI） | `deterministic-backend` | `real-window` | — | SV: project/receipt scenario / AL: visualization golden |
| CLI / batch 処理 | `deterministic-backend` | — | — | AL: CLI golden / SV: 該当機能の独立検証 |
| 保存・復元（Project/File） | `deterministic-backend` | — | — | SV: Project schema 往復 / AL: fixture 往復 |

**非劣化の構造**: マトリクスは repo ごとに独立した行を持ち、一方の repo の改善が他方の Case を暗黙に変更しない。共通基盤（Ruviform）の変更は resolved SHA の変化として各 repo の Case に記録され、三 repo の Gate はそれぞれ自分の SHA 組合せで再評価される。

### 6. Dogfooding とアダプタ境界

- RF-VTEST-01〜04（#628 Test Runtime・#629 仮想 OS Endpoint・#630 Test Plugin・#631 GPUI Scenario Runner）は Ruviform が提供する再利用可能な契約・fixture・runner であり、Synvelis/AudioLens の Dogfooding 対象である。
- 共通化するのは「契約と機構」（fixture 形式、Endpoint 契約、ABI Validator、pixel/receipt 証拠形式）まで。OS 固有の driver/kernel/SDK/GPUI 具象ハンドルは各 repo の private adapter に閉じ込め、Ruviform に製品固有仕様を持ち込まない。
- Test-only バイナリ（Test Plugin・仮想 Endpoint driver 等）は製品配布物に含めない。
- Consumer が発見した共通基盤の不足・不具合は Ruviform Issue として起票し、一時的回避には理由・解消条件を記録する（各 repo の AGENTS.md のマルチリポジトリ規約に従う）。

### 7. 明示ルール（false green 防止）

1. **仮想≠物理**: `environment_kind=virtual-os-endpoint`/`test-plugin` の `Pass` は、物理機材 Driver・ADC/DAC・市販 Plugin 固有互換・直結モニタリングを要求する受入条件の `Pass` ではない。これらを要求する Case は `environment_kind=physical-device` として別に残り、初回受入では Required にしない。
2. **Required は fail-closed**: `Required` の Case が `NotRun`/`Fail` の間、その機能領域は未受入である。環境不在を `Unsupported` に書き換えて Gate を緑にしてはならない。
3. **証拠なき緑は無効**: CI job が定義されている・成功しているだけでは Case の `Pass` にならない。`evidence` に RunID/URL と Oracle 比較結果が必要。
4. **Pass の非転移**: §4 のとおり、ある repo/fixture の Pass を別 repo の Case に転用してはならない。
5. **移行期の旧 Required**: 改訂前に `Required` だった物理依存 Case は、対応する仮想 Case の `Pass` が得られても自動的に `Pass` へ書き換わらない。旧 Case の扱い（`Deferred` 化・`NotApplicable` 化・維持）はゲート移管台帳（`issue-627-policies-revision-proposal.md`）に従い、ポリシー上の正式な状態変更は各 repo のレビューで行う。

## Data and Control Flow

1. **Case 定義**: 各 repo が機能領域ごとに Case を §2 の schema で定義し、`requirement`・`level`・`environment_kind`・`oracle` を宣言する。
2. **実行**: Ruviform の Test Runtime（#628）・仮想 Endpoint（#629）・Test Plugin（#630）・GUI Runner（#631）が `environment_kind` に応じた環境を構築し、製品コードを実行する。
3. **Oracle 判定**: 独立 Oracle が `expected` を生成し、被検証系の `observed` と比較する。production 実装は `expected` を生成しない。
4. **証拠記録**: `evidence`（RunID/URL）・`repo`/`sha`・`resolved_ruviform_sha`・`fixture_version` を Case 記録へ残す。
5. **Gate 評価**: `requirement=Required` の全 Case が `Pass` であることを当該 repo/OS の受入条件とする。`NotRun`/`Fail`/`Unsupported` が残る場合は未受入または要判断として報告する。

## Non-Functional Constraints

- **再現性**: 同一 `repo`/`sha`/`resolved_ruviform_sha`/`fixture_version` で同一結果が再現できること。fixture・driver・plugin は版または hash で固定する。
- **独立性**: Oracle は被検証系と別実装であること。共有してよいのは入力 fixture と比較規則の形式のみ。
- **fail-closed**: 証拠欠落・環境不在・Oracle 未定義は `Pass` にしない。
- **proportionality**: 検証深度は変更範囲に比例する（POLICIES.md §5）。docs-only の変更に実機・実 Plugin の実行を要求しない。
- **非配布**: Test-only 成果物は製品配布物・依存解決の対象にしない。

## Risks and Trade-offs

| リスク | 影響 | 緩和 |
| --- | --- | --- |
| 仮想 Endpoint の Pass を物理適合と誤読する | サポート範囲の過剰主張、出荷後の機材問題 | §7-1 の明示ルールと `environment_kind` の記録必須化。物理依存は `physical-device` Case として残す |
| Case schema が形骸化し Issue ごとに粒度がばらつく | Gate 判定が repo/Issue 間で比較不能 | §2 の必須項目を証拠契約として固定し、#294 系の候補 tuple Gate と整合させる |
| Test Plugin が実在 Plugin の挙動を代表しないと誤解される | ABI は検証できてもエコシステム互換の保証と混同 | `test-plugin` の「証明しないこと」列を schema/台帳に明記。市販 Plugin 適合は opt-in Case |
| 旧物理 Required の一括 `Deferred` 化で履歴が失われる | どの機能が物理未検証か追跡不能 | 移管台帳で旧 Case の状態遷移を個別に記録し、`Deferred` の追跡先を残す |
| Ruviform 共通化が製品固有仕様を基盤へ漏らす | 依存方向の逆転、Consumer 間の結合 | §6 のアダプタ境界。OS/SDK 具象は private adapter に限定 |

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `physical-device` Case の個別定義 | 初回受入の対象外。製品が当該機材/Plugin の正式サポートを宣言した時点で必要 | 各製品 Issue の Activation 条件 | 特定機材問題が仮想 fixture に還元されずサポート範囲が曖昧になる |
| macOS/Linux GUI-L2/EXT-L3 の Required 昇格 | 利用要件発生まで Deferred（現行ポリシーと同じ） | 各 OS/機能の利用決定 Issue | macOS/Linux の実 GUI/実機器利用時に Case が未定義のまま進む |
| 仮想 Endpoint の物理ドライバ差異の網羅 | 無限の機種差異を初回に網羅不可 | 報告済み問題の再現 fixture 化 | 実機特有の問題が発見のたびに後追いになる |
| #628〜#631 実装中の schema 改訂 | 実装で判明する制約を契約へ反映する余地を残す | 各 Issue の doc-chain | 契約が実装と乖離したまま固定される |

## Verification Notes

- [ ] 本設計の Case schema・境界 Case・Oracle 規則が `docs/test-specs/issue-627-virtual-first-verification-contract.md` の Feature/Rule/Example に写像されている
- [ ] `docs/designs/issue-627-policies-revision-proposal.md` の照合記録で、適用済み `POLICIES.md` 改訂が本契約の要求を満たすこと、残差分があれば改訂メモとして記録されている
- [ ] `review-protocol` によるレビューで blocker が残っていない
- [ ] `mise exec -- prek run --all-files` が Pass する（docs-only の衛生チェック）
