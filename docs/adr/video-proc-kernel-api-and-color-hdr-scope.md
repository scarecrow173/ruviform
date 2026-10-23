---
id: "ADR-034JcidaiOOip5EhNPOnDI"
type: "adr"
status: "accepted"
title: "video-proc カーネル API 形状と色変換・HDR 対応範囲"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
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
    - "../../AGENTS.md"
  refines:
    - "../specs/implement-video-foundation-api.md"
  refined-by: []
  derives-from:
    - "../discovery/explore-video-foundation-api-scope.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "realtime-contract-and-verification-policy.md"
    - "public-enum-non-exhaustive-policy.md"
  derived-by:
    - "docs/adr/broad-color-management.md"
    - "docs/plans/implement-issue-26-video-foundation.md"
    - "docs/specs/composite-transition-kernels.md"
    - "docs/designs/composite-transition-kernels.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/scarecrow173/ruviform/issues/41"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# video-proc カーネル API 形状と色変換・HDR 対応範囲

## Context and Problem Statement

GitHub Issue #26（動画基盤: `ruviform-video-core` / `ruviform-video-proc` の API 確定・色変換・HDR 対応範囲）の実装にあたり、相互に関係する 7 つの設計論点を確定する。設計ベースラインは「色変換・HDR の対応範囲」を**要 PoC**として本 Issue に割り当てており（採否分類表 L479）、ここで確定しない限り video-proc の API 形状も実装範囲も宙に浮く。また `ruviform-rt` の `Processor` trait・`RtContext` mint API が未実装（`crates/ruviform-rt/src/lib.rs:44`）という現状制約の下で、カーネル API を将来の `Processor` 接続を壊さない形で確定しなければならない。さらに Issue #41 トリアージで `SharedPlane`/`VideoFrame` の所有権・drop・zero-copy 契約の文書化が本 Issue に移管されている。

## Decision Drivers

* ベースラインの処理契約形状: `prepare(setup)`（非 RT・確保可）→ 別の処理オブジェクト → `process(&mut self, io, ctx)`。`Processor`/`RtContext` は `ruviform-rt` が所有する（ベースライン L283）
* `RtContext` はドライバ以外 mint 不可の設計であり、現状 mint API 自体が存在しない — それを要求する API は実装・テスト不能（L284、`rt/src/lib.rs:44`）
* RT 契約: `process` 系は RT 適合設計（alloc なし）だが、保証は API 単位で個別明示されるまで非保証（L197・L335–337・L430–432、RT 契約 ADR）
* DSP/SIMD 決定: スカラー参照実装を必須とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約を定義する（L370・L375）
* 広範色管理（moxcms/OpenColorIO 級）はベースラインで**保留**（第 2 ターゲット設計時、L275–276・L497）
* クレート依存方向: 層内順序 `audio-core`/`video-core` → `dsp`/`video-proc`/`media-io`（同順位の相互依存禁止、L169）。GPU 境界 API は利用側ポートとして `video-proc` が定義する（L179）
* 公開 enum `#[non_exhaustive]` 統一方針（[公開 enum ADR](public-enum-non-exhaustive-policy.md)）により video-proc は外部利用者として未知 variant を graceful に処理する
* 外部クレート依存ゼロ・`unsafe` 原則禁止の暫定規約（AGENTS.md）

## Considered Options

### D1. カーネル API 形状

* **Option D1-a — 状態なし自由関数** `fn convert(src: &VideoFrame, dst: &mut VideoFrame)`: 最も単純だが `prepare`/`process` 分離の契約形状に合わず、係数・ワーク領域の事前確保先がない
* **Option D1-b — 非 RT `prepare`/コンストラクタ → 準備済みオブジェクトの `process(&mut self, src, dst)`（ctx なし）**: ベースラインの二相形状に整合し、`RtContext` なしで今日実装・テスト可能。将来 `Processor` trait が定義された時点で trait impl を後方互換的に追加できる
* **Option D1-c — `ruviform-rt` に `Processor` trait を今定義**: 正規契約型への早期到達だが、RT 基盤 Issue の権限を侵し I/O ジェネリック設計を巻き取る
* **Option D1-d — `process` が `RtContext` 引数を要求**: 最終形に近いが mint API 不在のため誰も呼べない

### D2. 初期カーネル実装範囲

* **Option D2-a — 色変換のみ実装**: `Yuv420p`/`Yuv422p`/`Yuv444p` ↔ `Rgba8`（matrix・range 考慮、chroma resampling 込み）+ `Rgba8 ↔ RgbaF16` をスカラーのみで実装
* **Option D2-b — (a) + リサイズ/合成カーネル**: video-proc 責務の全領域をカバーするがスコープ過大
* **Option D2-c — API 定義のみ・実装は次 Issue**: 「対応範囲」の確定が空文化する

### D3. HDR「対応範囲」

* **Option D3-a — メタデータ搬送 + 非破壊のみ**: 変換は code-value 演算（matrix/range のみ適用）。transfer 関数適用・トーンマップを行わない
* **Option D3-b — (a) + transfer 考慮変換**: PQ/HLG→リニア化・トーンマップ。浮動小数点パイプラインと出力 range 規定を要しスコープが跳ねる
* **Option D3-c — 広範色管理**: moxcms/OCIO 級。ベースラインで保留済み

### D4. GPU 計算ポート trait

* **Option D4-a — ポート trait を今定義**: 境界の早期固定ができるが、実装者ゼロの trait は YAGNI で形状がバックエンド選定（wgpu 採否 = 要 PoC）に依存
* **Option D4-b — 所有宣言のみ記録し trait 定義は ruviform-gpu Issue へ**: 責務所在（L179）を明文化しつつ過剰設計を回避
* **Option D4-c — 完全後送り**: ポート所有が暗黙化する

### D5. `SharedPlane`/`VideoFrame` 所有・zero-copy・プール契約

* **Option D5-i — 現行維持 + 契約文書化**: `SharedPlane` は offset-less のまま（decoder が plane 単位 `Arc` を供給する前提。FFmpeg の per-plane `AVBufferRef` と概ね整合）。所有・drop・書込ゲート・プール返却方向を doc で規定。introspection API は契約文書化が必要とする場合のみ最小追加
* **Option D5-ii — `SharedPlane` に `offset: usize` 追加**: 一つの `Arc` から複数 plane を切り出せるが、media-io の未検証実需を先回り固定化する
* **Option D5-iii — `PlaneBacking` enum（Owned/External{arc, offset, len}）**: 表現力最大だが公開 API が膨らみ zero-copy 具体方式（media-io 要 PoC）を先取りする

### D6. クレート配線

* **Option D6-a — `crates/ruviform-video-proc` を `members` 追加（video-core 直後）、deps = `ruviform-video-core`（+ `ruviform-media-core` は `Error` 使用時のみ）、`ruviform-rt`・同順位依存なし**: 層内順序と依存方向に整合
* **Option D6-b — `ruviform-rt` にも依存**: 将来の接続が楽だが、`RtContext` を取らない設計では未使用依存（YAGNI 違反）

### D7. エラーモデル

* **Option D7-a — `ruviform_media_core::Error` 再利用**: video-core/audio-core と同一パターン。`Copy`・RT-suitable・`#[non_exhaustive]` で variant 追加も non-breaking
* **Option D7-b — video-proc 独自 `#[non_exhaustive] enum Error`**: ドメイン固有 variant の自由度があるが、2 系統の橋渡しコスト
* **Option D7-c — thiserror 等の外部クレート**: 外部依存ゼロ方針に反する

## Decision Outcome

* **D1 — Option D1-b を採用。** 各カーネルは非 RT の `prepare`/コンストラクタ（係数・ワーク領域の確保）で準備済み処理オブジェクトを生成し、その `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame)` は alloc-free とする。`RtContext` 引数は取らず（現状 mint 不可・要求すると呼べない）、`Processor` trait も実装しない。`ruviform-rt` で `Processor` が定義された時点で trait impl を追加する後方互換の拡張余地を crate doc に記録する。`**RT contract:**` ラベルは API 単位で付す
* **D2 — Option D2-a の縮小版を採用。** スカラーのみの色変換: `Yuv*p ↔ Rgba8`（`MatrixCoefficients` Bt601/Bt709/Bt2020 + `ColorRange` Limited/Full を考慮、chroma resampling 含む）と `Rgba8 ↔ RgbaF16`。スケーリング・合成は将来カーネルとして予約し本 Issue では実装しない。カーネルごとに等価性契約（スカラー = 参照実装）を doc 明記する
* **D3 — Option D3-a を採用。** HDR 対応範囲 = **メタデータ搬送 + 非破壊**。`matrix` が `Unspecified` なら Bt601、`range` が `Unspecified` なら Limited とみなす（doc 明記の既定規則）。出力 `ColorInfo`: YUV→RGB では `matrix` を `Identity` に更新、range 変換時は `range` を更新、`primaries`/`transfer`/`hdr` は入力をそのまま伝播する。transfer 関数適用・トーンマップは行わない。広範色管理は引き続き保留（第 2 ターゲット設計時）
* **D4 — Option D4-b を採用。** video-proc の crate doc に「GPU 計算カーネル境界のポート所有は本クレート」（ベースライン L179）と記録し、trait 本体は ruviform-gpu 実装 Issue に移管する
* **D5 — Option D5-i を採用。** 現行モデル（`Arc` 強共有 + `Arc::get_mut` 書込ゲート: 共有中は読取専用・排他所有時のみ `plane_mut` が `Some`）を doc で規定する。drop 挙動: 最後の `Arc` が外れると backing を解放（dealloc）するため、RT パスで `VideoFrame` の所有を drop しない — 退役フレームは非 RT 層へ送出して drop する（Drop 退避、ベースライン L287）。フレームプールは非 RT 層が所有し、返却・所有権移転は明示的 API として後続設計で定義する。`SharedPlane` の offset-less は既知の制約として記録し media-io Issue で再評価。zero-copy 供給は `from_shared_planes` が唯一経路（外部バッファ取り込みの検証規則を明記）。共有/排他所有の introspection API は契約文書化が必要とする場合のみ最小追加とする
* **D6 — Option D6-a を採用。** `crates/ruviform-video-proc` を `members` に追加し、deps は `ruviform-video-core`（+ `ruviform-media-core` は `Error`/`Result` 使用時のみ）。`ruviform-rt` 依存・同順位（dsp/media-io）依存は持たない
* **D7 — Option D7-a を採用。** `ruviform_media_core::Error`/`Result` を再利用する。video 固有の失敗（`Unspecified` で解釈不能・未対応 format ペア等）を表す必要が出た場合は media-core `Error` への variant 追加で対処する（`#[non_exhaustive]` のため non-breaking）

### Consequences

* 良い点: カーネル API が `RtContext` 不在の今日でも実装・テスト可能な形で確定し、`Processor` 導入時は impl 追加で済む。スカラー参照実装 + 等価性契約が SIMD PoC の検証基盤になる。HDR 範囲の明示答え（メタデータ搬送 + 非破壊）により video-proc の責務境界が明確になる。`SharedPlane`/`VideoFrame` の所有・drop・zero-copy 契約が文書化され、media-io・engines の実装者が参照できる。ポート所有宣言で ruviform-gpu Issue のスコープ境界が確定する
* 悪い点: `RtContext` を要求しない `process` は「RT であること」の型レベル強制がなく、誤用防止は `**RT contract:**` doc 契約とレビューに依存する。`SharedPlane` offset-less 制約により、単一 `Arc` バッファから複数 plane を参照する decoder 形式は media-io 側で per-plane `Arc` への再包装が必要になる。siting 前提（水平 co-sited・垂直 center）は暗黙仮定であり、相違コンテンツで半画素ずれが起き得る

### Confirmation

下記 Verification のチェックリストで確認する。`process` の alloc-free は `rt_` プレフィックスの契約テスト（`mise run test-rt` で選択）とコードレビューで確認し、等価性契約は golden vector テストで検証する。ドキュメント規則（所有モデル・drop・RT ラベル・GPU ポート宣言・HDR 規則）は API doc / crate doc の記載確認で検証する。

## Implementation Plan

* Affected paths:
  * `crates/ruviform-video-core/src/lib.rs` / `frame.rs` / `plane.rs` — 所有権モデル・drop 挙動・zero-copy 経路・`**RT contract:**` ラベルの doc 精緻化（必要時のみ introspection API の最小追加）
  * `crates/ruviform-video-proc/`（新設）— `Cargo.toml`・`src/lib.rs`（crate doc: 責務・依存層・RT 制約・GPU ポート所有宣言・`Processor` 拡張余地）・カーネルモジュール
  * `Cargo.toml`（ルート）— `members` に `crates/ruviform-video-proc` を video-core 直後に追加
  * `AGENTS.md`・`README.md` — ワークスペース構成・クレート一覧の同期
* Patterns to follow: video-core/audio-core の `**RT contract:**` ラベル形式（「RT-suitable — …」「non-RT only — …」）、`from_shared_planes` の検証スタイル（`Error::InvalidValue(&'static str)`）、`Plane`/`PlaneMut` の `row()` 単位処理（padding を跨がない）、enum `#[non_exhaustive]` + `_` arm の graceful 処理、`#![allow(clippy::exhaustive_structs)]` の literal 構築ポリシー
* Patterns to avoid: `process` 内の alloc/dealloc・ブロッキング・無制限待機、`RtContext` 引数、`Processor` trait の先行定義、外部クレート型の公開 API 露出、`unreachable!()` による未知 variant 処理、`unsafe`、非 RT テスト名への `rt_` 混入（substring フィルタ誤検出）
* Dependencies or configuration: 外部依存追加なし。`ruviform-video-proc` の path 依存は `ruviform-video-core`（+ `ruviform-media-core` は `Error` 使用時のみ）

## Verification

* [ ] `crates/ruviform-video-proc` が workspace `members` に存在し、deps が D6 の規則どおり（`cargo metadata` / `Cargo.toml` 確認）
* [ ] 各カーネルが非 RT `prepare`/コンストラクタ → `process(&mut self, src, dst)` 形状を持ち、`RtContext`・`Processor` trait を参照しない
* [ ] `Yuv*p ↔ Rgba8` が matrix（Bt601/Bt709/Bt2020）・range（Limited/Full）・chroma resampling をカバーし、`Rgba8 ↔ RgbaF16` が存在する。golden vector テストがグリーン
* [ ] `Unspecified` 既定規則・出力 `ColorInfo` 更新規則・HDR 非破壊（transfer/hdr 伝播）が doc にあり、実装・テストが一致する
* [ ] video-core の所有モデル・drop 挙動・zero-copy 供給経路・`SharedPlane` offset-less 制約・プール返却方向が doc に記載されている
* [ ] 借用・共有・確保系 API に `**RT contract:**` ラベルがあり alloc/dealloc 挙動をカバーする。`process` の alloc-free を検査する `rt_` テストが存在する
* [ ] crate doc に GPU ポート所有宣言と trait 移管先（ruviform-gpu Issue）が明記されている
* [ ] 全公開 enum が `#[non_exhaustive]`、`unsafe`・外部依存なし。`mise run check` / `lint` / `test` / `test-rt` / `fmt-check` がグリーン

## Pros and Cons of the Options

### D1 — Option D1-b（採用）

* Good, because `prepare`/`process` の二相分離（ベースライン L283）に整合しつつ `RtContext` を要求しないため、今日のワークスペースで実装・テスト・doctest が成立する。`Processor` impl は struct 形状が合っていれば後方互換で追加できる。
* Bad, because RT コンテキストの型レベル強制がなく、「RT で呼んでよい」は doc 契約とレビューに依存する。将来 `process(&mut self, io, ctx)` へ trait シグネチャを合わせる際に引数追加が必要になる可能性がある。

### D1 — Option D1-a（不採用）

* Good, because 最も単純で型も少ない。
* Bad, because 係数テーブル・ワーク領域の事前確保先がなく、確保を `process` 内に押し込むか毎回計算するかになり、RT 契約・性能の両面で劣る。

### D1 — Option D1-c / D1-d（不採用）

* Good, because 正規の契約型に近い。
* Bad, because `Processor` の I/O ジェネリック設計・`RtContext` mint 規則は RT 基盤 Issue の権限であり、本 Issue での先行定義は越権かつ仕様凍結リスクを負う。`RtContext` 引数は mint API 不在で文字通り呼べない。

### D2 — Option D2-a 縮小版（採用）

* Good, because 要 PoC 行の「対応範囲」を最小実装で確定でき、スカラー参照実装としての役割と等価性契約の検証対象を両立する。
* Bad, because スケーリング・合成が未実装のまま残り、video-proc の責務全体をカバーしない。

### D2 — Option D2-b / D2-c（不採用）

* Good, because D2-b は責務全体を早期カバーでき、D2-c は本 Issue を軽くできる。
* Bad, because D2-b はフィルタ・合成モデルの設計論点を増やして本 Issue を収束不能にし、D2-c は「対応範囲」の確定が名目的になり実装上の規約（siting・端数・等価性）が検証されない。

### D3 — Option D3-a（採用）

* Good, because `ColorInfo` の設計意図（記述・搬送のみ、変換しない）と整合し、第 1 ターゲットの再生・変換用途に十分。実装・検証範囲が小さく、広範色管理の保留決定（ベースライン L276・L497）と矛盾しない。
* Bad, because PQ/HLG の正しい表示変換は別レイヤー（第 2 ターゲットの色管理）を待つ必要がある。

### D3 — Option D3-b / D3-c（不採用）

* Good, because HDR 実用性・正しさが上がる。
* Bad, because リニア化は浮動小数点パイプラインと出力規定を要求し、広範色管理はベースラインが明示的に保留した第 2 ターゲット設計の領分であり本 Issue の権限外。

### D4 — Option D4-b（採用）

* Good, because ベースライン L179 の責務所在を crate doc に明文化しつつ、バックエンド選定（wgpu 採否の要 PoC）に依存する trait 形状を無理に固定しない。
* Bad, because trait 不在の間は GPU 経路が未接続であり、ruviform-gpu Issue が始まるまで境界は宣言のみに留まる。

### D4 — Option D4-a / D4-c（不採用）

* Good, because D4-a は境界を早期固定でき、D4-c は作業ゼロ。
* Bad, because D4-a は実装者ゼロの trait で YAGNI かつ形状凍結リスク、D4-c はポート所有の暗黙化で gpu Issue 側の責務争いを残す。

### D5 — Option D5-i（採用）

* Good, because AC の文書化要求を型変更なしで満たし、zero-copy 具体方式（media-io 要 PoC）の実需確定まで型を固定しない。FFmpeg の per-plane `AVBufferRef` モデルと概ね整合し、主要 decoder 形式をカバーする。
* Bad, because 単一 `Arc` バッファ内の plane オフセット参照を表現できず、必要になれば media-io 側で再包装または `SharedPlane` 拡張が必要になる。

### D5 — Option D5-ii / D5-iii（不採用）

* Good, because 表現力が上がり単一バッファ multi-plane を直接扱える。
* Bad, because 未検証の実需を先回りで公開 API に固定し、検証規則・所有セマンティクスを複雑化する。zero-copy 方式の確定は media-io Issue の割当と重複する。

### D6 — Option D6-a（採用）

* Good, because 層内順序・依存方向ルールに整合し、未使用依存を持たない。
* Bad, because `Processor` impl 時に `ruviform-rt` 依存を追加する変更が別途必要になる（ただしその時点で正当化される）。

### D6 — Option D6-b（不採用）

* Good, because 将来の trait impl 準備が早い。
* Bad, because `RtContext` を取らない設計では実質未使用の依存であり YAGNI 違反。

### D7 — Option D7-a（採用）

* Good, because video-core/audio-core と同一パターンで一貫し、`Copy`・RT-suitable なエラーが `process` 周辺でも使える。variant 追加は `#[non_exhaustive]` のため non-breaking。
* Bad, because video 固有エラーの表現粒度は `InvalidValue(&'static str)` の文字列と将来の variant 追加に依存する。

### D7 — Option D7-b / D7-c（不採用）

* Good, because D7-b はドメイン固有 variant を自由に設計でき、D7-c は記述が楽。
* Bad, because D7-b はエラー系統の分岐と橋渡しコストを生み、D7-c は外部依存ゼロ方針に反する。

## More Information

* 一次ソース: GitHub Issue [#26 [impl] 動画基盤（video-core / video-proc API・色変換・HDR 対応範囲）](https://github.com/scarecrow173/ruviform/issues/26)（`relations.source`）。受け入れ条件は #26 コメント（#41 トリアージ item 6 由来）: 所有権モデル文書化・drop/プール契約・zero-copy 整合性・RT ラベル。
* 上流ドキュメント（`relations.derives-from`）: [調査 discovery](../discovery/explore-video-foundation-api-scope.md)（D1–D7 の選択肢分析の正本）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（採否表 L479「色変換・HDR の対応範囲 = 要PoC → 動画基盤 Issue」は本 ADR で決定に昇格）、[RT 契約 ADR](realtime-contract-and-verification-policy.md)、[公開 enum ADR](public-enum-non-exhaustive-policy.md)。
* 精緻化対象（`relations.refines`）: [spec implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — 本 ADR が spec の設計判断を確定する（spec 側は `refined-by` で逆リンク）。
* 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定開発規約）。
* 参照（`relations.references`）: Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（親 Issue・ベースラインの一次ソース）、Issue [#41](https://github.com/scarecrow173/ruviform/issues/41)（AC 移管元）。
* 調査ブリーフ: `tmp/issue-26/research-brief.md`（リポジトリ管理外の作業成果物。API インベントリ・エッジケース一覧の一次集約）。
