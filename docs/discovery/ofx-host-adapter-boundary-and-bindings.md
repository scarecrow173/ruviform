---
id: "DISC-034L2cDWe52zQHUbLyygTw"
type: "discovery"
status: "resolved"
title: "OFX ホストアダプタ境界とバインディング方針の探索（Issue #98）"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/98"
    - "https://github.com/AcademySoftwareFoundation/openfx"
    - "https://doc.rust-lang.org/stable/unstable-book/language-features/c-variadic.html"
    - "https://openfx.readthedocs.io/en/latest/Reference/ofxThreadSafety.html"
    - "https://openfx.readthedocs.io/en/latest/Reference/suites/ofxMemorySuiteReference.html"
    - "https://crates.io/crates/ofx"
    - "https://crates.io/crates/ofx_sys"
    - "https://github.com/itadinanta/ofx-rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/ofx-video-plugin-scope.md"
    - "../designs/second-target-design.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/ofx-host-adapter.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../designs/ofx-host-adapter.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OFX ホストアダプタ境界とバインディング方針の探索（Issue #98）

## Exploration Goal

Issue [#98](https://github.com/scarecrow173/ruviform/issues/98) は **OFX（OpenFX）Image Effect ホスト**機能を要求する（plugin オーサリング側ではなくホスト側）。最低限 imageEffect / property / parameter スイートを対象とし、`ruviform-video-core` の `VideoFrame`/`ColorInfo`・`ruviform-media-core` の `MediaTime`/`FrameRate` へのマッピングを考える。GPU / Generator / Transition / overlay は初期スコープ外。L0 は in-process ホスティング、L1（#76 リモート分離）は前提としない。

既存の上流成果物は [ofx-video-plugin-scope.md](../adr/ofx-video-plugin-scope.md)（accepted）と [second-target-design.md](../designs/second-target-design.md)（approved）。現時点で OFX 関連の新規ソースコードは存在しない。

**Cycle 1 レビュー結果（frozen・実施済み）**: 独立 A/B レビュー + Validator + Judge で **BLOCKED**。confirmed finding は P1/A2（Created 状態の setter/state 方針）と P1/D1（normalized コントロール変換の実害 — 下記再現ケース）、非ブロッキング P2/C1（既存 core の overflow 系不具合・別経路）。他の A/B 主張は REJECTED/UNVERIFIED — 誤りとされた SDK 主張を「修正」しない。

**Cycle 2 レビュー結果（frozen・実施済み — `scratch/judge-cycle2.md`）**: Judge **APPROVED — ACCEPT WITH NON-BLOCKING NOTES**。P1/A2・P1/D1 は design レベルで解決（実装 fix ではない）。C1/P2 既存 core overflow は causal path から外れて留保・未修正（repository bugfix ではない）。Cycle 2 A1/B1 重複指摘は **UNVERIFIED**（confirmed でも rejected でもない）— slave param change 時の `GetClipPreferences` 義務は SDK 上明確（pinned `ofxImageEffect.h` L322-L325・L833-L839）だが、design は activation-only 検査を規定しておらず実害は未証明・実装も未存在。未解決の confirmed P0/P1 なし・上限 2 フルサイクルで打切り。native/MSRV/production/platform/audioRT 検証はすべて PLANNED・未実施。`doc-audit5.json` = 282 docs・0 findings。**本 discovery は resolved — 仮説・open questions は EXPLORATION-TIME の履歴として残し、正本は spec/ADR/design に移譲済み**

## Key Issues and Assumptions

### ローカル構造の確認済みギャップ（confirmed）

- `crates/ruviform-plugin-core/src/processor.rs` の `PluginProcessor` は `BufferView`（audio planar `&[f32]`）/`BufferViewMut` + `SampleRate` ベース `ProcessConfig`。`ruviform-plugin-host` の `Plugin: PluginProcessor` 境界と `ErasedPlugin` ディスパッチも同じオーディオビュー前提。**video ペイロードを持つ plugin processor は現存しない**
- `Lifecycle` はペイロード非依存で再利用可能。audio RT 保証を OFX render で暗黙に弱めない
- `VideoFrame` は `Vec`/`Arc` 裏付け・借用 `Plane` は unsigned stride。外来 OFX イメージは layout/lifetime/row-origin/negative-stride/packing の明示チェックが必要。`VideoFrame::from_shared_planes` は NONRT・出力 mutability は `Arc` 一意性依存

### 確認済み欠陥再現（D1・frozen facts — temp-only 証拠ではない）

- `ParamInfo`/`ParamValue` の normalized 経路で、`min=-1e20, max=+1e20`（両端とも有限・妥当な宣言）に対し plain `1` を与えると、正規化 `(1-(-1e20))/(2e20)=0.5` を経由して **plain `0` に丸められて返る**（往復で値が壊れる）
- overflow 系の修正では**治らない**（範囲幅が f64 で表現可能でも normalized 中間表現の分解能が cause）
- lead の Rust 1.89 scratch 実証: source `param_range_probe.rs`・出力 `param-range-msrv-evidence.txt`（temp `i98-ofx-probe`）— 7 テスト pass・**本番テストではない**。結論: normalized 専用の video コントロール面ではなく、typed plain 値チャネル（`PlainParamValue`/`PlainParamInfo`）が必要 — 既存の normalized audio/wire コードは変更しない
- **C1（別経路・deferred core defect・未修正）**: 既存 core `param.rs:223-246` 付近の overflow 系不具合は confirmed だが、plain チャネル採用で本 causal path 外 — 別 Issue/後続作業へ留保（auto Issue 作成・コード変更なし）

### 外部証拠（lead 確認 2026-10-24・frozen）

- 公式 SDK: `AcademySoftwareFoundation/openfx` @ `c3f1c2dcf4f51762b794c4fc21181b4a501be1bf`（BSD-3-Clause）。pinned 参照: `include/` の [ofxCore](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxCore.h)・[ofxImageEffect](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxImageEffect.h)・[ofxProperty](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxProperty.h)・[ofxParam](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxParam.h)・[ofxMemory](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxMemory.h)・[ofxMultiThread](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxMultiThread.h)・[ofxMessage](https://github.com/AcademySoftwareFoundation/openfx/blob/c3f1c2dcf4f51762b794c4fc21181b4a501be1bf/include/ofxMessage.h)。scratch ミラー: `Temp/i98-ofx-probe/scratch/sdk/`（`sdk-manifest.json` に sha256）。`HostSupport/src/ofxhProperty.cpp` は同 SHA に存在しない（404 — 別名か未存在、未確定）
- C-variadic 実測（decisive scope evidence）: `pub unsafe extern "C" fn probe(_tag: i32, _args: ...) -> i32 { 0 }` を `--crate-type lib --edition 2024 --emit metadata` で検査 — **MSRV 1.89 で E0658（#44930）拒否 / dev 1.99 で受理**。1.99 での va_list 抽出・実用コールバック・他 OS は未検証。現 MSRV では Rust 内 variadic ホストコールバック実装を前提にできない。MSRV 引上げ提案なし。C shim は比較候補 — **EXPLORATION-TIME の記録であり、最終決定は accepted ADR `ofx-host-video-contract-and-c-abi.md` が権威**
- 言語リファレンス（supplementary・version-sensitive）: [unstable-book c-variadic](https://doc.rust-lang.org/stable/unstable-book/language-features/c-variadic.html)
- `ofx`/`ofx_sys`: `ofx` は client/オーサリング側 wrapper（[ofx-rs README](https://github.com/itadinanta/ofx-rs) 明示）で turnkey ホストではない。`ofx_sys` は raw bindings — 原理的に host 用途可だが実装済みホストなし・同じ C-variadic 問題が残る。crates.io メタ確定値: 両者 Apache-2.0・yanked=false・`rust_version=null`・published 2019-02-04。repo は archived=false・pushed_at 2021-04-12・license=null（repo 欠落 ≠ package 欠落）。`ofx_sys` は旧 bindgen 0.43 build-dep + libc・実効 MSRV 未検証
- 上流 C++ HostSupport: `HostSupport/CMakeLists.txt` が `cxx_std_17`・`PUBLIC expat::expat` を要求・README は XML キャッシュ+expat 記述。`HostSupport/LICENSE` は 404・README は BSD-style「see sources」— 未ライセンスとは断定しない
- [ofxThreadSafety](https://openfx.readthedocs.io/en/latest/Reference/ofxThreadSafety.html) は plugin ロック/マルチスレッド callback 許容・[ofxMemorySuite](https://openfx.readthedocs.io/en/latest/Reference/suites/ofxMemorySuiteReference.html) は汎用 alloc/free あり — **OFX render は audio のハード RT 保証を継承できない**

### 依存メタデータ（frozen — scratch のみから、再集計なし）

宣言値は package メタデータの `rust-version` フィールドで実コンパイル証明ではない。保守状況はこれらだけでは証明されない（採用前に別途確認）。closure は現在の Cargo.lock の依存辺（全 target 跨ぎ・自身含む）で、build-active feature 閉包や実効 MSRV 証明ではない。

| crate | locked | declared MSRV | package license | closure（自身含む・3 ノード） |
| --- | --- | --- | --- | --- |
| `cc` | 1.9.0 | 1.65.0 | MIT OR Apache-2.0 | cc@1.9.0・find-msvc-tools@0.1.15・shlex@2.0.1 |
| `libloading` | 0.8.9 | 1.71.0 | ISC | cfg-if@1.0.5・libloading@0.8.9・windows-link@0.2.1 |

repo 観測（GET api.github.com/repos/…・観測時刻 UTC 2026-10-23T18:45:07/18:45:14Z — raw selected fields のみ・健全性保証ではない）: `rust-lang/cc-rs` archived=false・disabled=false・pushed_at 2026-10-23T08:35:45Z・main; `nagisa/rust_libloading` archived=false・disabled=false・pushed_at 2026-08-02T09:16:45Z・master。repo SPDX は package license の上書きではない。

## Alternatives and Comparison

### 軸 1: バインディング経路

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (1) `ofx` wrapper 再利用 | ヘッダ pin 工数回避 | client/オーサリング側前提で host と構造不整合 | no |
| (1') `ofx_sys` raw bindings 再利用 | ABI 宣言の再利用で写経削減 — **ホスト側 C-variadic shim の回避効果ではない** | 実装済みホストなし・同じ C-variadic 問題・旧 bindgen 0.43（libclang）・実効 MSRV 未検証。C bridge が要る以上必須ではない | maybe |
| (2) 公式 pinned ヘッダ + in-crate 最小 FFI + 小さな C variadic shim | 依存最小・pinned SHA で監査可能・MSRV 1.89 下でも C shim 経由で定義可 | ABI audit・C ツールチェイン導入・shim 設計/テスト負荷 | candidate |
| (3) 上流 C++ HostSupport 採用 | 規格解釈を上流実装に委譲 | `cxx_std_17`+`expat::expat`（XML キャッシュ）のビルド面積が初期 profile を超える（ライセンス理由ではない） | maybe — 初回では不要 |

### 軸 2: プラグイン契約境界（video port）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 既存 audio `PluginProcessor` trait 再利用 | 追加抽象なし | ペイロード不整合（`&[f32]` planar audio ≠ video frame）。RT 保証の意味が変わる | no |
| (b) plugin-core に additive な video-core 裏付け契約を追加 | Lifecycle・ErasedPlugin の既存形と整合・audio 契約を弱めない | 新規 trait/境界の設計コスト。依存方向は plugin-core→video-core（逆方向禁止） | candidate（仮説） |
| (c) standalone ofx-only API を新設 | plugin-core を変更しない | 中立案 render 契約の所有先が不在になるリスク（境界違反の証明ではない） | no（現時点） |

**仮説（EXPLORATION-TIME の履歴記録）**: additive NONRT video 契約 + 明示的 C 境界（(2)+(b)）が最も整合 — 実承認は spec（approved）・ADR（accepted）・design（approved）側に確定済み。

## Tentative Conclusions and Hypotheses

- **confirmed**: video plugin processor 未存在・audio 契約は payload 不整合・`Lifecycle` 再利用可・OFX threading/memory 規格は audio RT 保証を提供しない
- **confirmed (D1)**: normalized-only コントロール面は実害あり（[-1e20,1e20] で plain 1→normalized .5→plain 0）。typed plain チャネルが必要
- **confirmed**: MSRV 1.89 で Rust 内 C-variadic ホストコールバック定義は不可（実測 E0658）。dev 1.99 は受理するが va_list/実用/他 OS 未検証
- **confirmed**: `ofx` は client wrapper。`ofx_sys` は raw — 原理的に host 可だがホスト実装なし・variadic 問題不変
- **hypothesis（EXPLORATION-TIME・ADR 確定済み）**: pinned ヘッダ + 最小 FFI + C shim（軸1 (2)）・additive NONRT video 契約（軸2 (b)）— accepted ADR が決定権威
- **deferred (C1)**: 既存 core `param.rs:223-246` overflow 系は causal path 外として留保（未修正・別作業）

## Open Questions

> EXPLORATION-TIME の履歴 — 反映済みの事項は spec/ADR/design が正本。残存は将来の implementation verification として追跡（architecture 未決事項ではない）。

- 最低限サポート suite 群の確定と正直な capability advertisement の条件
- filter/general contexts・parameter kinds・pixel depth/alpha/row-origin/time/color マッピング確定
- バインディング MSRV・ライセンス・依存木の確実な証拠、動的ライブラリ lifetime・suite ハンドル管理・panic 封じ込め
- audio `Plugin` supertrait を要求せず同一 L0 lifecycle に載せる統合形
- ユーザプラグインをロードしない fixture plugin 検証戦略

## Promotion Candidates

- [x] spec-doc: `../specs/ofx-host-adapter.md`（**approved — addressed**）
- [x] adr-doc: `../adr/ofx-host-video-contract-and-c-abi.md`（**accepted — addressed**）+ design `../designs/ofx-host-adapter.md`（**approved — addressed**）。plan/tests/tasks は次の planning dispatch で起票予定

## Sources and Relations

- Issue [#98](https://github.com/scarecrow173/ruviform/issues/98)・公式 SDK pinned SHA `c3f1c2dc…`（scratch `i98-ofx-probe/scratch/sdk/` + manifest）
- 公式 docs: ofxThreadSafety・ofxMemorySuite（上記 URL）
- 言語リファレンス: unstable-book c-variadic（supplementary・version-sensitive）
- 候補クレート: ofx 0.3.0 / ofx_sys 0.2.0 / ofx-rs repo
- ローカル一次証拠: `crates/ruviform-plugin-core/src/processor.rs`・`crates/ruviform-plugin-host`・`crates/ruviform-video-core`（`VideoFrame`/`Plane`/`ColorInfo`）・`crates/ruviform-media-core`（`MediaTime`/`FrameRate`）
- scratch: `i98-ofx-probe/` — probe・`param_range_probe.rs`・`param-range-msrv-evidence.txt`・`scratch/sdk-manifest.json`・`ofx-native-dependency-metadata.json`・`cc-repository-metadata.json`・`libloading-repository-metadata.json`・`judge-cycle1.md`
- 関連文書: [ofx-video-plugin-scope.md](../adr/ofx-video-plugin-scope.md)（accepted）・[second-target-design.md](../designs/second-target-design.md)（approved）
