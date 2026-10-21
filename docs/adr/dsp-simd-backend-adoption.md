---
id: "ADR-034JEe7SAbksaycyrum16R"
type: "adr"
status: "accepted"
title: "DSP/SIMD 二層構造とバックエンドの採否"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/19"
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
    - "../designs/ruviform-architecture-design-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/dsp-simd-poc-issue-19.md"
    - "../impl/exp/dsp-simd-poc-20261021.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../../AGENTS.md"
  refines:
    - "../designs/ruviform-architecture-design-baseline.md"
  refined-by: []
  derives-from:
    - "../plans/dsp-simd-poc.md"
    - "../designs/dsp-simd-poc.md"
    - "../specs/dsp-simd-poc-adr.md"
    - "dsp-simd-poc.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# DSP/SIMD 二層構造とバックエンドの採否

## Context and Problem Statement

Issue #19 の最終成果物として、DSP/SIMD PoC の計測証拠に基づき採否を確定する。設計ベースライン「DSP／SIMD 設計（A.4）」節で **要 PoC** とされた論点 — SIMD 二層構造（スカラー参照実装 + ランタイム分岐 + フォールバック）の採否、`fearless_simd` / `pulp` / `multiversion` のバックエンド選定、数値等価基準の具体値、AVX-512 の扱い、denormal 方針、MSRV 判断 — の 6 項目を本 ADR で結論する。

評価アプローチは先行 ADR [dsp-simd-poc.md](dsp-simd-poc.md)（`ADR-034JcihlNe8WCZVpp9RBCq`）で確定済みであり、そこで明示的に送られた「採否判断」が本 ADR の責務である。証拠は T4/T6 の実績として確定済み: 計測レポート [dsp-simd-poc-20261021.md](../impl/exp/dsp-simd-poc-20261021.md)（生データ `data/dsp-simd-poc-20261021-01.jsonl` / `-02.jsonl`、イベントログ [dsp-simd-poc-benchmark-comparison.jsonl](../impl/exp/dsp-simd-poc-benchmark-comparison.jsonl)）と等価性 matrix（`crates/ruviform-math/tests/simd_equiv.rs`、契約定義 `tests/simd_poc/equiv.rs`）。計測環境は Windows 11 x64・i9-9900K（AVX2 まで、AVX-512 非搭載）・rustc 1.99.0・criterion 0.8.2・2 run × 756 ケース。

## Decision Drivers

* 設計書の判定目安（`dsp-simd-poc.md`「判定目安の数値化」）: 代表カーネル（add/scale/clamp/fade/dot/fir）の過半数で主要帯 256/1024/4096 のスカラー比 ≥1.5x、小サイズ 64/256 で dispatched 経路が <0.9x に回帰しないこと、dispatch オーバーヘッド @1024 ≈≤5%。不達なら「不採用」または「条件付き採用」が正当な結論であり、先に決めた結論へ数値を寄せない
* 確定制約（設計ベースライン・AGENTS.md）: stable Rust のみ・workspace MSRV `rust-version = "1.85"`・`unsafe` 原則禁止・候補クレート型の公開 API 非露出・スカラー参照必須・x86_64 は SSE2 baseline → AVX2/FMA 起動時検出、aarch64 NEON は baseline
* 採否基準の補助軸: コンパイルコスト・unsafe カプセル化範囲・MSRV 適合・保守状況・等価性検証の網羅性・denormal 影響
* 計測の信頼限界: 単一環境（Windows x64 のみ）・run 間 |Δ| p50=1.8%/p90=25.7% の並行負荷ノイズ（評価は run 内比率で相殺）・AVX-512 未計測・aarch64 はコンパイル確認のみ

## Considered Options

**SIMD 二層構造:**

* Option A — 採用する（スカラー参照を常備し、SIMD は dispatch 経路として追加）
* Option B — 否決する（スカラーのみ）

**バックエンド:**

* Option A — `fearless_simd` 1.1.0 + `fearless_simd_macros` 0.1.0
* Option B — `pulp` 0.22.3
* Option C — `multiversion` 0.9.0 + autovec スカラー

**`dot` 等価基準の確定形:**

* Option A — ulp 上限 256 で確定（実測 max 46 に ~5.5x マージン）
* Option B — 相対誤差・代替契約形へ切替（f64 真値基準やスケール相対）

**AVX-512:**

* Option A — 将来扱い（実測不在を明記し、実機環境が得られた時点で再検証）
* Option B — 未計測のまま有効化前提で採用側に含める

**denormal:**

* Option A — 方針のみ記録し実装はエンジン層 Issue へ送る（policy-only）
* Option B — PoC/カーネル側で対策実装まで行う

**MSRV:**

* Option A — workspace 1.85 を維持し、候補は dev-dep 封じ込めのまま本番昇格を MSRV 引上げ Issue に送る
* Option B — MSRV を即時引き上げて本番 `[dependencies]` 昇格まで行う
* Option C — 1.85 適合の旧版 pin で候補を採用する

## Decision Outcome

### 決定 1 — SIMD 二層構造: **採用**（カーネル別適用を条件とする）

スカラー参照実装 + ランタイム dispatch + フォールバックの二層構造を採用する。ただし証拠は仮説を精緻化した: スカラーは「参照・フォールバック」に留まらず、**小サイズ・メモリ帯域律速のカーネルでは本番経路として残る**。

* 性能根拠（スカラー比・2 run 平均 @256/1024/4096、計測レポート判定表）: `fearless_avx2` で dot 7.9–8.3x・fir 3.6–5.8x・biquad ~2.8x・fade 4.1–6.7x、`pulp_v3` で dot 8.1–8.9x・fir 2.6–3.7x・fade 3.9–6.6x — 演算律速カーネルで採用側目安 ≥1.5x を明確に充足
* 反面、補助条件「dispatched 経路 <0.9x 回帰なし」は naive な per-call dispatch では不達: `fearless_dispatched` add/64=0.58x・add/256=0.83x・scale/64=0.82x・clamp_unit/64=0.73x・clamp_unit/256=0.70x、`pulp_dispatched` clamp_unit/64=0.82x、`multiversion` fir/64=0.76x。elementwise 小バッファでは SIMD + dispatch がスカラーに劣後する
* よって採用形は「スカラー参照（=小サイズ本番経路を兼ねる）+ レーン別 SIMD カーネル + カーネル・サイズ応答のルーティングを dispatch 層（`ruviform-math`）に集約」。反証材料 `sine` は全 impl ≈1.00x で設計どおり SIMD 非適用領域の存在も実証された

### 決定 2 — バックエンド: **`fearless_simd` を条件付き採用**、`pulp` を検証済み代替として記録、`multiversion` は不採用

`fearless_simd`（+ `fearless_simd_macros`）を採用バックエンドとして選定する。条件: (a) 適用は演算律速カーネルに限定し、メモリ帯域律速・小サイズはスカラー経路へルーティングする。(b) dispatch は per-call ではなく prepare/ブロック境界で `Level`/`Simd` トークンを保持・渡す設計とし、`fearless_dispatched` で観測された軽量カーネル劣後（@1024 で add 0.62x・scale 0.66x vs 強制 avx2）を構造的に回避する。(c) 本番 `[dependencies]` 昇格は決定 6 の MSRV 引上げ Issue 完了後。

* 採用根拠: 演算律速カーネルで総合最良（fir 3.6–5.8x vs pulp 2.6–3.7x、biquad ~2.8x vs ~1.5x — DSP で中核となる累積・再帰系カーネルで差がついた）、`Level` トークン経由の強制 level 指定で等価性 matrix を決定的に検証可能、依存ゼロ・Apache-2.0 OR MIT・3 年保守公約（Linebender、1.1.0 = 2026-10-06）、`#[non_exhaustive]` の `Level` enum が公開 enum 統一方針 ADR と同型。`unsafe` カプセル化は利用側コード 0（`assume_supported`/`new_unchecked`/`std::arch` 不使用、lint ゲート通過）
* `pulp` は性能上も有力な対抗（dot でやや優位・dispatched ≈ 強制列で per-call オーバーヘッド未観測）だったが、fir/biquad での倍率差・MSRV 未宣言・MIT 単独・依存 ~8 で次点。実績（faer 系列・31M DL）は強みであり、fearless_simd の dispatch 実装が本番統合で構造的問題を示した場合の再評価対象として記録する。PoC ハーネスに pulp 実装が残るため再評価コストは低い
* `multiversion` は不採用: dot ~1.0x・fir 0.69–0.73x と性能根拠不達に加え、個別 clone 強制 API が存在せず等価性 matrix が「検出環境の最良 clone のみ」に構造的に限定される（検証網羅性の差として記録）。dispatch 層のみの構成単純さは魅力だが、SIMD 抽象化を持たず中身を autovec に委ねる限界が実測で裏付けられた

### 決定 3 — 等価基準の確定値: `dot` ulp ≤ **256**、`fir`/`biquad` ulp ≤ **8**、elementwise/再帰一致系は bit 完全一致を維持

* `dot`: 設計初期値 8 → **確定値 256**。T4 matrix で実測 max 46 ulp を観測し、f64 真値との照合で「実装バグではなく契約不適合」と診断済み（impl・scalar ともに真値から ~1e-7 絶対誤差内: レーン分割 reduction + 打ち消しセルが、中間スケールの数 ulp の差を結果スケールの数十 ulp に増幅）。256 は実測 max の ~5.5x のマージンで、本機未計測のレーン幅（NEON 4-lane・AVX-512 16-lane）のヘッドルームを含む。相対誤差形への切替は採用しない — 絶対誤差が小さいままであることが実証されており、契約形を fir/biquad と揃える方が単純。ただし ulp 上限は結果スケール相対であり、強打ち消し入力では任意の固定 ulp 上限を超え得る点を契約の限界として明記する。本番 API 契約では「有限入力・PoC 入力クラス」を前提条件とし、それを超える入力域を保証する必要が生じた場合は設計書の代替契約形（f64 真値基準 or スケール相対誤差）へ切り替える — 事後緩和はせず、超過時はまず不具合/契約不適合として調査するプロトコルを再確認する
* `fir` ulp ≤ 8（確定）: 実測 max 0。候補 impl はスカラー参照のタップ順序を保つ非融合 multiply-add を採用 — FMA 版は修正前に ~3994 ulp の打ち消しドリフトを実測しており、融合不使用は実測に基づく設計判断として記録する
* `biquad` ulp ≤ 8（確定）: 実測 max 0（FMA 不使用の同様の理由による）。設計書の代替契約形（先頭 N フレーム ulp + 相対誤差/エネルギー上限）は発動しなかった
* bit 完全一致（`to_bits` + NaN `is_nan` 一致）: `add`/`scale`/`clamp_unit`/`fade`/`sine` — 全構成可能列でクリア（実測済み）

### 決定 4 — AVX-512: **将来扱い（未計測を明記）**

計測環境に AVX-512 が存在せず、`fearless_avx512`・`pulp_v4` は probe skip 記録のみで**実測なし**。本 ADR の採否は SSE4.2/AVX2（aarch64 NEON はコンパイル確認のみ）の証拠に基づく。方針: AVX-512 dispatch 列・等価性セルは probe skip 機構として既に存在するため、実機（または CI の AVX-512 環境）が得られた時点で matrix・bench を再実行し、特に `dot` の 16 レーン reduction が確定上限 256 に収まることを確認してから有効化する。未検証のまま AVX-512 を本番 dispatch に含めない。aarch64 NEON 実測も同様に後続 Issue/CI `workflow_dispatch` へ送る

### 決定 5 — denormal: **方針のみ記録、実装はエンジン層 Issue へ送る**（policy-only）

* 実測（denormal/normal 中央値比 @1024）: `fir` は全 impl で崖（5.1–31.8x、広い SIMD ほど絶対比が大きい: fearless_avx2 31.8x・pulp_v3 21.1x）、`biquad` 1.6–4.3x、`sine` 全 impl ~10x（scalar 含む）、`dot` 崖なし。崖は性能のみで等価性は維持される（denormal 入力は matrix で契約適合を検証済み）
* FTZ/DAZ の設定は Rust 上で形式的 UB（`std::arch` docs 明記）のため、カーネル内での FTZ/DAZ 操作は行わない。方針: denormal 対策はエンジン層の責務（ホスト側 FTZ/DAZ 設定、または将来の API 契約で定義されるカーネル内ソフトウェア flush）とし、カーネルは denormal 入力を契約上正しく処理するが性能崖対策は持たない。実装の移管先は spec「Deferred Design Concerns」どおりエンジン層 Issue

### 決定 6 — MSRV: **workspace 1.85 を維持、候補は dev-dep 封じ込めのまま、本番昇格は MSRV 引上げ Issue 前提**

候補全員が workspace MSRV 1.85 を超過: `fearless_simd` 1.1.0 = 1.89、`multiversion` 0.9.0 = 1.86、`criterion` 0.8.2 = 1.86、`pulp` 0.22.3 = 未宣言（`x86-v4` 有効時は `bytemuck` の `avx512_simd` 経由で推移的下限 ≈1.89）、`approx` 0.5.1 = 未宣言。実測: `rustup run 1.85 cargo check -p ruviform-math --lib --locked` はグリーン（dev-dep 封じ込め下で `--lib` 範囲は 1.85 適合）。既知の制限: `cargo +1.85` での fresh resolve は dev-dep 範囲に適合版がなく失敗し得る — lockfile 運用では非問題として記録する。

* 決定: 現行 MSRV 1.85 を維持し、候補クレートは `[dev-dependencies]` のまま据え置く。`fearless_simd` の本番 `[dependencies]` 昇格には workspace `rust-version` の ≥1.89 への引上げが前提となる — spec「Deferred Design Concerns」どおり**別途 MSRV 引上げ Issue** を起票し、そこで判断する（本 ADR は MSRV 自体を変更しない）
* 旧版 pin（1.85 適合版への downgrade）は採らない: `fearless_simd` に 1.85 適合系は存在せず、選定候補を旧版へ固定することは採用判断と矛盾する

### Consequences

* 良い点: 二層構造・バックエンド・等価基準が実測根拠付きで確定し、後続の DSP 実装 Issue（dispatch 層設計・カーネル API 確定）が着手可能になる。スカラー参照・等価性 matrix・bench ハーネスが dev-dep 内の資産として残り、バックエンド差替え・新規カーネル追加の検証基準として再利用できる。`unsafe` 0・公開 API 非露出・stable-only の確定制約を全て維持したまま採用判断を得た。不採用候補（multiversion・wide）と未決事項（AVX-512・denormal・MSRV 引上げ・他アーキ実測）の根拠と送り先が明文化された
* 悪い点: `fearless_simd` の per-call dispatch オーバーヘッドは軽量カーネルで実測される一方、dispatch コストかノイズかの分離は未解決（低負荷環境での再計測が残課題）。本番昇格は MSRV ≥1.89 引上げというコストの大きい前提に依存する。dev-dep 増でコールド `cargo check --workspace --all-targets` が 0.60s→30.14s（cargo 報告、`--timings` 単位別: `pulp` 系 ≈9.9s・`fearless_simd` 系 ≈8.4s・`criterion` 自身 1.68s・`multiversion` 系 ≈1.2s・`approx` 0.14s — ただし推移的依存は計測済みキャッシュ使用、~30s 増分には criterion/pulp の推移的依存分も含む）。AVX-512・aarch64 実測・denormal 対策は未解決のまま後続へ持ち越す

### Confirmation

下記 Verification のチェックリストで確認する。本 ADR は決定記録でありコード変更を伴わないため、確認は (a) 採否基準への証拠照合の実在（計測レポート判定表・等価性 matrix・lint/MSRV 検証の実績）、(b) 決定事項が後続 Issue の前提として参照可能であること（MSRV 引上げ・denormal・AVX-512 の移管先明記）の 2 軸で行う。

## Implementation Plan

* Affected paths: 本 ADR は記録のみでコード変更を伴わない。決定が影響する後続対象: `crates/ruviform-math`（dispatch 層集約・本番昇格は MSRV Issue 後）、エンジン層（denormal 方針の実装先）、workspace ルート `Cargo.toml`（MSRV 引上げ Issue で `rust-version` 変更の候補）、CI `workflow_dispatch`（AVX-512/aarch64 実測の実行先）
* Patterns to follow: スカラー参照を正とし dispatch level 強制指定の matrix で等価性を検証する（`tests/simd_equiv.rs`・`tests/simd_poc/equiv.rs` が既存パターン）。dispatch は `Level`/`Simd` トークンを prepare・ブロック境界で保持して繰り上げ、per-call `dispatch!` を軽量カーネルに使わない。小バッファ・メモリ律速カーネルはスカラー経路へルーティングする。候補クレートの型は `pub` API に露出しない
* Patterns to avoid: カーネル内での FTZ/DAZ 操作（Rust 上で形式的 UB）。`assume_supported`/`new_unchecked`/`std::arch` 直接呼出し（unsafe 原則禁止）。観測値に合わせた等価基準の事後緩和（超過時はまず不具合/契約不適合として調査）。未計測 ISA（AVX-512）の有効化前提の設計。`multiversion` 式の「検出環境依存のみ」の等価性検証
* Dependencies or configuration: 本 ADR 自身は依存を変更しない（PoC dev-dep は既に配置済み）。本番昇格時: `fearless_simd` を `[dependencies]` へ移す作業は MSRV ≥1.89 引上げ Issue の完了が前提

## Verification

* [x] 採否 ADR が 6 項目（二層構造採否・バックエンド選定・等価基準確定値・AVX-512・denormal・MSRV 判断）を証拠参照付きで記録している（本書 Decision Outcome）
* [x] 判定目安との照合が実測で記録されている（計測レポート「判定目安との照合」表。≥1.5x は演算律速カーネルで充足・elementwise 不達、dispatched 小サイズ回帰あり、dispatch オーバーヘッド fearless で目安超過 — すべて本文に反映）
* [x] 等価性 matrix が確定基準でグリーン（`mise run test` 実績。bit 完全一致 5 カーネル + ulp: dot ≤256 実測 max 46・fir/biquad ≤8 実測 max 0）
* [x] PoC コードの `unsafe` が 0 で `mise run lint` の `unsafe_code` ゲートを通過（`git grep "unsafe" crates/ruviform-math` は doc 言及 3 件のみ）
* [x] `rustup run 1.85 cargo check -p ruviform-math --lib --locked` がグリーン（dev-dep 封じ込め下での MSRV 検証実績）
* [ ] MSRV 引上げ Issue が起票され、`fearless_simd` の本番 `[dependencies]` 昇格判断がそこで行われる（後続作業 — 未着手）
* [ ] AVX-512 実機または CI 環境での matrix/bench 再実行と 16 レーン `dot` の ulp 256 収まり確認（後続作業 — 環境不在）
* [ ] aarch64 NEON の実測（CI `workflow_dispatch` 等 — コンパイル確認のみ実施済み、計測は未実施）
* [ ] denormal 対策の実装（エンジン層 Issue — 本 PoC 範囲外）
* [ ] `fearless_dispatched` の軽量カーネル劣後が per-call dispatch コストかノイズかの分離（低負荷環境での再計測 — 残課題として記録）

## Pros and Cons of the Options

**バックエンド:**

### Option A — `fearless_simd`（採用）

* Good, because 演算律速カーネルで最高倍率（fir 3.6–5.8x・biquad ~2.8x・dot ~8x・fade 4–6.7x）、`Level` トークンの明示指定で等価性 matrix を決定的に実行可能、利用側 `unsafe` 0、依存ゼロ・Apache-2.0 OR MIT・3 年保守公約・MSRV 1.89 明示宣言、`#[non_exhaustive]` `Level` が公開 enum 統一方針と同型、トークン保持で dispatch 繰り上げ可能。
* Bad, because per-call `dispatch!` は軽量カーネルで実測劣後（add/scale @1024 で 0.62–0.66x vs 強制列）、小サイズ dispatched 回帰あり（0.58–0.84x）。MSRV 1.89 で workspace 1.85 と衝突。1.x リリース間もなく実績はこれから。`min`/`max`/`mul_add` の NaN・±0・非 FMA 意味が implementation-defined（`*_precise` 版で回避済み）。

### Option B — `pulp`（検証済み代替として記録・不採用）

* Good, because dispatched ≈ 強制列で per-call dispatch オーバーヘッド未観測（≈0.98–1.38 @1024）、dot で最大倍率（8.1–8.9x）、`Arch`/`try_new` で強制 level 指定可能・利用側 `unsafe` 0、実績十分（faer 系列・31M DL）。
* Bad, because fir 2.6–3.7x・biquad ~1.5x と中核カーネルで fearless_simd に劣後。MSRV 未宣言（`x86-v4` では推移的 ≈1.89）。MIT 単独・依存 ~8（bytemuck・raw-cpuid・reborrow 等）。dispatch 網羅性の制限（レビュー補足）: `pulp::x86::Arch::new()` の分岐は V4 → V3 → Scalar で V2 層を持たず、SSE4.2 止まりの環境では dispatched 経路がスカラーに降りる（強制 `pulp_v2` 列のみが SSE4.2 を検証可能）。fearless_simd の `Level` は SSE2/SSE4.2/AVX2/AVX-512 を個別保持しこの層間ギャップを持たない。

### Option C — `multiversion` + autovec（不採用）

* Good, because 構成が最も単純（関数単位 multiversioning のみ）・MSRV 1.86・no_std 可・MIT OR Apache-2.0。
* Bad, because 性能根拠不達: dot ~1.0x・fir 0.69–0.73x の回帰。個別 clone 強制 API 不在で等価性 matrix が検出環境の最良 clone に構造的限定される（検証網羅性の差）。SIMD 抽象化を持たず中身は autovec 依存 — 手書き SIMD に移すと unsafe カプセル化を自前で負う。

**その他の軸:**

* 二層構造否決（Option B）は、演算律速カーネルの 3–9x を捨てる根拠がなく、否決の対価（将来の手書き SIMD 再導入）が大きいため退けた。elementwise 不達は「二層構造否決」ではなく「カーネル別適用」を示す証拠
* `dot` の相対誤差形（Option B）は、f64 真値照合で絶対誤差 ~1e-7 が実証され契約不適合の診断が済んだため不採用 — ただし入力域拡大時の再検討経路として代替契約形を残す
* AVX-512 有効化前提（Option B）は実測不在のため採用不可 — probe skip 記録のまま将来扱い
* denormal 実装込み（Option B）は FTZ/DAZ が形式的 UB で spec 範囲外のため採用不可 — 方針のみ記録
* MSRV 即時引上げ（Option B）は PoC 依存を本番昇格する Issue として分離すべき spec 判断と不整合、旧版 pin（Option C）は選定候補に適合版がないため不可

## More Information

* 一次ソース: GitHub Issue [#19 [poc] DSP/SIMD 比較ベンチマークと採否 ADR](https://github.com/scarecrow173/ruviform/issues/19)（`relations.source`）
* 証拠: 計測レポート [dsp-simd-poc-20261021.md](../impl/exp/dsp-simd-poc-20261021.md)（判定表・denormal 崖表・dispatch オーバーヘッド・補助データ）・生データ [`data/dsp-simd-poc-20261021-01.jsonl`](../impl/exp/data/dsp-simd-poc-20261021-01.jsonl) / [`-02.jsonl`](../impl/exp/data/dsp-simd-poc-20261021-02.jsonl)・イベントログ [dsp-simd-poc-benchmark-comparison.jsonl](../impl/exp/dsp-simd-poc-benchmark-comparison.jsonl)・等価性契約コード `crates/ruviform-math/tests/simd_poc/equiv.rs`（`MAX_ULPS_DOT/FIR/BIQUAD`）・matrix テスト `crates/ruviform-math/tests/simd_equiv.rs`
* 派生元（`relations.derives-from`）: [plan](../plans/dsp-simd-poc.md)（Phase G・本 ADR の作成タスクの正本）・[design](../designs/dsp-simd-poc.md)（採否基準・判定目安・等価性契約の正本）・[spec](../specs/dsp-simd-poc-adr.md)（受け入れ基準・Deferred Design Concerns の正本）・[評価アプローチ ADR](dsp-simd-poc.md)（`ADR-034JcihlNe8WCZVpp9RBCq` — 採否判断を本 ADR へ明示的に送った先行決定）
* 上位制約（`relations.depends-on`/`refines`/`related`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md)「DSP／SIMD 設計（A.4）」節（要 PoC 項目 — 本 ADR が確定し `refines` する）、[discovery](../discovery/dsp-simd-poc-issue-19.md)（候補比較・保守状況調査値の canonical 記録 — 本 ADR の保守状況・ライセンス記述は同書 2026-10 調査値に基づく）、[RT 契約 ADR](realtime-contract-and-verification-policy.md)・[platform ADR](platform-and-toolchain-targets.md)（x86_64 SSE2 baseline → AVX2/FMA 起動時検出・aarch64 NEON baseline の前提）、[公開 enum ADR](public-enum-non-exhaustive-policy.md)、[AGENTS.md](../../AGENTS.md)
* 後続（後続 Issue として送る項目）: MSRV ≥1.89 引上げ Issue（本番昇格の前提）・エンジン層 denormal 対策 Issue・AVX-512/aarch64 実測（CI `workflow_dispatch` または専用 Issue）・`fearless_dispatched` 軽量カーネル劣後の原因分離（低負荷再計測）
* 親 Issue: GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（`relations.references` — 設計ベースラインの発生源）
