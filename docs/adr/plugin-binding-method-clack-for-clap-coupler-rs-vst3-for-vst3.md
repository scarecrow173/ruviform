---
id: "ADR-034JDzseXORnckswW1oYKh"
type: "adr"
status: "accepted"
title: "Plugin binding method: clack for CLAP, coupler-rs vst3 for VST3"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "plugin-architecture-clap-vst3.md"
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "public-enum-non-exhaustive-policy.md"
    - "../discovery/evaluate-plugin-binding-options.md"
    - "../specs/implement-plugin-foundation.md"
    - "../plans/implement-plugin-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/evaluate-plugin-binding-options.md"
    - "../tasks/poc-clap-binding-adapter.md"
    - "../tasks/poc-vst3-binding-adapter.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/prokopyl/clack"
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Plugin binding method: clack for CLAP, coupler-rs vst3 for VST3

## Context and Problem Statement

Issue #29 は規格中立 `ruviform-plugin-core` の上に CLAP/VST3 の薄いアダプタを構築する。plugin ADR は binding 構築方式（既存 binding クレート採用 vs 自前実装）を「T3/T4 の PoC 実測に基づく方式 ADR」へ明示的に保留した。両 PoC（`crates/ruviform-plugin-clap`・`crates/ruviform-plugin-vst3`、最小ゲインプラグイン・実 `clap_entry`/`GetPluginFactory` エクスポート）が完了し実測が揃ったため、規格ごとの構築方式を確定する。方式差異は規格単位で許容する — 両規格に同一方式を強制しない。

## Decision Drivers

* spec 受け入れ基準: ゼロコピー借用・process 経路 alloc なし・canonical planar f32・`TimeBase`+整数カウント・RT/非RT 分離
* unsafe は原則禁止・必要なら SAFETY コメント付き局所許容のみ（workspace 規約）
* binding クレート型の公開 API 非露出（spec Requirements）
* ビルド要件は crates.io 取得のみが望ましい（外部 SDK/サブモジュールを増やさない）
* `rust-version = "1.85"` workspace 制約・ライセンス適合（licensing ADR）
* 依存木サイズ・保守状況・拡張カバレッジ（params/events/audio-ports/thread-check）

## Considered Options

* **CLAP 側**: (a) `clack` 0.2.0 系採用 / (b) `clap-sys` 直上の自前薄ラッパー
* **VST3 側**: (a) `vst3`（coupler-rs）0.3.0 採用 / (b) 自前 COM 層＋生成バインディングなし

## Decision Outcome

Chosen option: **CLAP = `clack` 0.2.0 系採用（clack-plugin/-common/-extensions `=0.2.0` + dev-dep `clack-host`）、VST3 = `vst3` 0.3.0 採用**。実測比較（発見記録の正本: [discovery PoC Findings](../discovery/evaluate-plugin-binding-options.md)）:

| 評価項目 | CLAP `clack` 0.2.0 | VST3 `vst3` 0.3.0 | 退路との比較 |
| --- | --- | --- | --- |
| ゼロコピー | 達成（`ChannelPair` → `PlanarBufferView`、clack-host でポインタ一致実証） | 達成（`channelBuffers32` → `PlanarBufferView`、テスト実測） | 同等に可能 |
| アダプタ unsafe | **記述 0**（export マクロ内部のみ） | `unsafe fn` シグネチャ義務 54 件＋局所ブロック 51 件（約半数はテスト COM ダブル）・実質 raw 操作は `block.rs` 3 箇所 | `clap-sys` 薄ラッパーでは clack 水準に追うための抽象層を自前化する必要。自前 COM 層でも `unsafe fn` 義務・`RefCell` 内部可変性・検証層は同様に必要 — **クレート採否より unsafe 局所化方針が実質の決定点** |
| process 経路 alloc | なし（activate 時 `BlockScratch` 事前確保） | なし（activate/`setupProcessing` 時 scratch 事前確保） | 同等に設計可能 |
| ビルド要件 | 追加ツールなし | **VST3 SDK ヘッダ・サブモジュール不要を実測**（生成バインディング同梱・Open Question 解消） | `clap-sys` も同程度。自前 COM 層は生成バインディング喪失で工数増 |
| MSRV/ライセンス | 1.85 内でビルド成功・MIT OR Apache-2.0 | `rust_version` 未宣言・pinned 1.99.0 でビルド成功（**1.85 単独ビルドは未実測 — T6 確認残件**）・MIT OR Apache-2.0 | 同様 |
| 拡張カバレッジ | audio-ports・params・events・thread-check・gui を feature 分割提供 | COM 全般を自前ラップする設計（`kNotImplemented` スタブで段階拡張可能） | clap-sys は C API のみ — 拡張も自前。自前 COM 層も同じ |
| 依存木・保守 | 15 パッケージ・活発なリリース系 | 小さめ・唯一の保守された生成バインディング | clap-sys はさらに小さいが抽象なし |

### Consequences

* Good, because CLAP 側は safe 抽象によりアダプタ unsafe を実質ゼロに抑えられ、VST3 側も生成バインディング＋COM 足場により自前 COM 層の工数を回避できる。両側とも追加ビルドツールチェーンが不要で CI 適合性が高い。
* Good, because in-place 処理・境界検証・ライフサイクル写像の方式が両側で確定し、T6 は方式判断ではなく実装に集中できる。
* Bad, because `clack` は 0.x のため API churn リスクがある — exact pin（`=0.2.0`）で封じ、バージョン緩和は再評価時まで行わない。
* Bad, because `vst3` の `rust_version` 未宣言・MSRV 1.85 単独ビルド未実測、macOS/Linux エントリのクロスビルド未実測、`sort_unstable` 同等のイベント整列安定化残件、生成バインディングの SDK 出自（ライセンス系統の継続調査）が T6/T7 の確認残件として残る。

### Confirmation

* T6 のアダプタ完成実装が本 ADR の方式（clack 系／vst3 系・unsafe 局所化・in-place/エイリアス・スクラッチ・プラットフォームエクスポート条件）に従うことを review-protocol で確認する。
* `docs/discovery/evaluate-plugin-binding-options.md` の PoC Findings 節が実測根拠の正本として残る。

## Implementation Plan

* Affected paths: `crates/ruviform-plugin-clap/`（方式確定済み）、`crates/ruviform-plugin-vst3/`（同）、`docs/adr/README.md`（索引登録）、`docs/specs/implement-plugin-foundation.md`（本 ADR への relation）、`docs/discovery/evaluate-plugin-binding-options.md`（`derived-by` リンク・status 同期）
* Patterns to follow:
  * **in-place 対応方式**: per-channel ポインタ対比較を検出粒度とする（バッファ全体/`in_place_pair` 等価判定のみに留めない。部分エイリアス `out[i]==in[j]`・出力チャンネルポインタ重複も検出対象）。CLAP 側は `clack` の `ChannelPair::InPlace` 分類を写像する専用分岐（読み側 view 空＋出力を入出力として処理）。VST3 側は検出したエイリアスチャンネルを activate/`setupProcessing` 時に事前確保した per-instance bounce 領域へ逃がすスクラッチ経路。**`static`/`thread_local` 共有スクラッチは禁止**（spec Requirements）
  * **VST3 エクスポートシンボルのプラットフォーム条件**: macOS `bundleEntry`/`bundleExit` 必須・Linux `ModuleEntry`/`ModuleExit` 必須・Windows `InitDll`/`ExitDll` 任意（PoC は実装済み）。T6 のアダプタ検証基準に反映する。upstream example の `BundleEntry`/`BundleExit` 命名に倣わず spec 名を採用する注意を維持
  * **unsafe 局所化**: VST3 側の `unsafe fn` シグネチャ義務と局所 `unsafe {}` は FFI 境界関数に限定し SAFETY コメント必須。raw メモリ操作（slice 再解釈・bounce 書き戻し）は `block.rs` に集中させる現行構造を維持
  * **pin 管理**: `clack-*` は `=0.2.0` exact pin を維持。`vst3` は `0.3.0`（緩和可否は MSRV 実測後に再評価）
* Patterns to avoid: `clap-sys` 直上の自前薄ラッパー・自前 COM 層（不採用側の退路として見積りのみ保持）、`static`/`thread_local` スクラッチ、f64 サンプルの f32 再解釈（`kSample64` は定義済み拒否）
* Dependencies or configuration: `clack-plugin =0.2.0`・`clack-common =0.2.0`・`clack-extensions =0.2.0`（features `clack-plugin`/`audio-ports`/`params`）・dev-dep `clack-host =0.2.0`・`vst3 =0.3.0`

### 不採用側の退路（見積り）

* CLAP → `clap-sys` 薄ラッパー: clack が API churn/不具合で不適となった場合の退路。clack が提供する safe 抽象（ChannelPair・event space・entry 生成・host/plugin 両側）を自前で薄く再実装する範囲 — 中規模（数日〜1 週規模）。unsafe 量は増加するが `clap-sys` の ABI 宣言は安定
* VST3 → 自前 COM 層: `vst3` が保守停止/ライセンス不整合となった場合の退路。IUnknown/IDispatch 基礎・vtable 生成・`IPluginBase`/`IComponent`/`IAudioProcessor`/`IEditController`/`IPluginFactory*` の最小実装を自前で書く範囲 — 大規模（unsafe 増大・検証コスト増）。生成バインディングの代替入手（SDK ヘッダからの再生成）も選択肢

## Verification

* [x] `docs/adr/` に方式 ADR が作成され、規格ごとの構築方式決定・比較根拠・不採用側の退路が記録されている
* [x] in-place 対応方式・per-channel ポインタ対比較の検出粒度・bounce/チャンネルリストスクラッチの activate/prepare 時事前確保（per-instance・`static`/`thread_local` 禁止）・VST3 プラットフォーム条件付きエクスポートシンボル要件が ADR に記録されている
* [x] spec front matter relations に方式 ADR リンクが追加され、`docs/adr/README.md` の索引に新規 ADR 行がある
* [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --type adr --json` で 0 findings
* [ ] `node .agents/skills/doc-status/scripts/doc_status.js lint --rule broken-link --json` で 0 findings

## Pros and Cons of the Options

### CLAP: `clack` 0.2.0 採用（採用）

* Good, because アダプタ記述 unsafe が実質ゼロ — PoC の全受け入れ条件（ゼロコピー・no-alloc・境界検証・ライフサイクル・in-place）を最小規模で満たした。
* Good, because safe 抽象が検証負荷を下げ、`clack-host` が dev-dep として実 process 経路のテスト駆動を可能にする。
* Bad, because 0.x API churn — 緩和不可のため exact pin が必須で、追随コストが残る。

### CLAP: `clap-sys` 直上の自前薄ラッパー（不採用・退路）

* Good, because ABI 宣言は安定しており clack への依存を断てる。
* Bad, because clack が吸収している safe 抽象・イベント空間・entry 生成・in-place 分類を自前で再実装する必要があり、unsafe 量と検証コストが増える。

### VST3: `vst3`（coupler-rs）0.3.0 採用（採用）

* Good, because 唯一の保守された生成バインディング＋COM 足場 — `unsafe fn` 義務は COM 本質由来で自前 COM 層でも同等に必要であり、生成物が無料で手に入る。SDK ヘッダ不要を実測。
* Bad, because safe 抽象がないため `RefCell` 内部可変性・シグネチャ unsafe 義務・手書き検証層を負う。`rust_version` 未宣言（1.85 単独ビルド未実測）・macOS/Linux エントリのクロスビルド未実測が残件。

### VST3: 自前 COM 層＋生成バインディングなし（不採用・退路）

* Good, because binding 依存を断てる。
* Bad, because vtable 生成・IUnknown 基礎・全 COM インターフェース実装を自前化する大規模工数で、`unsafe fn` 義務と検証層は変わらず残るため純増のコストになる。

## More Information

* 実測根拠: [discovery `evaluate-plugin-binding-options.md`](../discovery/evaluate-plugin-binding-options.md) の PoC Findings 節（CLAP T3・VST3 T4）
* 実装記録: [T3 IR](../impl/ir/poc-clap-binding-adapter.md)・[T4 IR](../impl/ir/poc-vst3-binding-adapter.md)
* 上位文書: [plugin ADR](plugin-architecture-clap-vst3.md)（binding 方式を本評価へ保留した決定）、[spec](../specs/implement-plugin-foundation.md)、[plan](../plans/implement-plugin-foundation.md) T5
* T6 への確認残件: `vst3` MSRV 1.85 単独ビルド・macOS/Linux エントリのクロスビルド・イベント整列の安定化・生成バインディング SDK 出自のライセンス継続調査
