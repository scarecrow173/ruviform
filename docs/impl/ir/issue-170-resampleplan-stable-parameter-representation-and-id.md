---
id: "IMPL-034PrM2BjKBhjGVsAarWhT"
type: "impl"
status: "completed"
title: "Issue #170 ResamplePlan stable parameter representation and ID"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/170"
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-170-resampleplan-stable-parameter-representation-and-id.md"
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/src/resample.rs"
      - type: "file"
        path: "crates/ruviform-numeric/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-numeric/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #170 ResamplePlan stable parameter representation and ID

## Summary

audio-lens #102（R4 契約提案）が要求する「`ResamplePlan` の安定した
パラメータ表現/ID」を `ruviform_numeric::resample` に追加する。
AL の `normalized_pcm` 永続契約・cache 鍵は plan の内部パラメータ
（getter 経由の `up`/`down`/`radius`/`cutoff`）へ直接依存しており、
内部表現の変更で永続成果物が崩れ得る。変換を一意に決める正準
パラメータ値 `ResampleParams` と、そこから決定的に導出される
バージョン化 stable ID を ruviform 側の契約として提供する。

## Context

- Source: ruviform issue #170 / audio-lens issue #102（R4 契約提案）。
- AL 側参照（読み取りのみ、変更禁止）:
  - `al-locate/src/normalize.rs` — `transform(plan)` が
    `source_rate`/`target_rate`/`up`/`down`/`radius`/`cutoff` getter と
    AL 側定数（`algorithm_id`=`"polyphase-blackman-sinc"`・
    `algorithm_version`=`"1"`・window/endpoint/rounding/phase 規約）から
    `serde_json::Value` を構築。`NormalizedParams.transform` として
    artifact `params` に埋め込み、`r.params == value` の JSON 一致で
    dedup、`validate()` で `transform(&plan)` を再導出して完全一致を
    要求する（fail-closed）。
  - `al-locate/src/decode.rs` — `read_normalized_record` が
    `NormalizedParams::validate` を経て永続成果物を復元する。
- AL が要求する識別粒度: artifact `params`（`serde_json::Value`）に
  埋め込んで等値比較できる文字列/値。アルゴリズム版を含む変換の
  意味的同一性をカバーし、内部表現（係数表レイアウト等）には
  依存しないこと。
- `ruviform-numeric` は std のみ（Primitives 最下位）— serde 等の
  直列化依存は追加できないため、正準表現は依存ゼロの値型 +
  正準文字列形式で提供する。

## Implementation

### Changes

- `crates/ruviform-numeric/src/resample.rs`
  - `ResampleParams`（新規公開型）— 係数表を含まない正準パラメータ
    値（`Copy`/`PartialEq`/`Eq`/`Hash`）。フィールドは
    `source_rate`/`target_rate`/`up`/`down`/`radius`/`cutoff_bits`
    （`f64::to_bits` で保持し、浮動小数表記に依らず Eq/Hash で
    bit-exact 比較できる）。
    - `ResampleParams::new(source_rate, target_rate)` — gcd 約分・
      radius・cutoff を導出。係数表を構築しないため、係数表確保が
      現実的でない極端な有理比でも ID を得られる（RT-suitable）。
    - `CONTRACT_VERSION = 1` — パラメータ集合・直列化形式・変換
      意味論の後方非互換な変更で増分する契約バージョン。
    - `ALGORITHM_ID = "polyphase-blackman-sinc"` — AL が永続してきた
      provenance トークンと同一の値を ruviform 側定数として提供。
    - `stable_id()` — 正準直列化文字列
      `rvf-resample/<version>/<src>/<dst>/<up>/<down>/<radius>/<cutoff-bits-hex16>`
      を返す（`Display` と同一、non-RT）。
    - `from_stable_id()` / `FromStr` — 逆変換。タグ・version・
      正準スペル（先頭ゼロなし 10 進・16 桁小文字 hex）・全
      フィールドの導出整合性（`new(src, dst)` との完全一致）を
      検証し、非正準・手組み・未知 version を
      `Error::InvalidValue("params")` で拒否する（RT-suitable、
      alloc なし）。
    - `output_len()` — `ceil(frames·up/down)` を plan と同一規約で
      params 単体でも提供。
  - `ResamplePlan` は `params: ResampleParams` + `phases: Vec<f64>`
    の保持へ移行し、getter/`output_len`/`phase_coefficients` は
    `ResampleParams` 側へ集約（パラメータ導出の単一の正本）。
    - `params()` — 正準パラメータを `Copy` で返す（RT-suitable）。
    - `stable_id()` — `params().stable_id()` の shortcut（non-RT）。
    - `from_params(params)` — 正準パラメータから係数表込み plan を
      構築。`new` は `ResampleParams::new` → `from_params` の 2 段に
      なり、`stable_id` から復元した params も同一経路で plan 化
      できる。
  - モジュール doc・`ResamplePlan`/`ResampleParams` の型 doc に
    安定性契約（決定性・等価性・version 運用・opaque 扱い）を記述。
    `stable_id` に golden 文字列を固定する doctest を追加。
  - 契約テスト 5 件追加（`resample::tests`）。
- `crates/ruviform-numeric/src/lib.rs`
  - `rt_processing_paths_do_not_allocate` に `params()`・
    `ResampleParams::new`・`from_stable_id`・`output_len` の
    alloc-free 検証を追加。
- `crates/ruviform-numeric/README.md`
  - `resample` の責務記述に `ResampleParams`/stable ID を追記。
- `docs/impl/ir/README.md` — 本記録を索引へ登録。

### 正準直列化形式（version 1）

```text
rvf-resample/1/<source_rate>/<target_rate>/<up>/<down>/<radius>/<cutoff-bits>
例: rvf-resample/1/48000/96000/2/1/32/3fee147ae147ae14
```

- `source_rate`/`target_rate`/`up`/`down`/`radius` は先頭ゼロなしの
  10 進、`cutoff` は `f64::to_bits` の 16 桁小文字 hex。
- 同一 version 内で「同一パラメータ → 同一文字列」を全プラットフォーム・
  全実行で保証する（`DefaultHasher`・`HashMap` 順序等の非決定的な
  仕組みは使わない）。
- 等価パラメータは内部表現（係数表レイアウト・確保方法）が異なっても
  同一 ID。パラメータ集合・形式・変換意味論を変える修正は
  `CONTRACT_VERSION` を増分し、永続済み ID は自然に無効化される
  （前方互換の考え方: 新 version は旧 version の ID を受理しないため、
  古い cache 鍵は新実装と衝突しない）。
- `from_stable_id` は「本 version が生成し得る正準値」以外を拒否する
  fail-closed 境界。AL 側は復元時に `new` 由来の値と比較せず
  この parse に委ねられる。

### Decisions During Implementation

調査で確定した事実と、それに基づく設計判断:

- AL の識別粒度: artifact `params`（`serde_json::Value`）へ埋め込み
  JSON 一致で dedup し、`validate()` で plan getter から `transform`
  を再導出して完全一致を要求する。よって「文字列として比較可能・
  再導出可能」な ID が必要十分で、serde 型は不要（数値クレートは
  std のみの層ルールで serde を引けない）。
- 正準パラメータは変換を一意に決める意味的集合
  （`source_rate`/`target_rate`/`up`/`down`/`radius`/`cutoff`）とし、
  係数表を含めない。係数表は params からの全射導出物であり、
  「同じ意味的パラメータ → 同じ係数」が成り立つため ID には不要。
- `ResamplePlan` のフィールドを `ResampleParams` に集約したのは、
  パラメータ導出（gcd・radius・cutoff）を plan 構築と正準表現で
  二重化させないため。`output_len` も同一規約の正本として
  `ResampleParams` 側へ移し、plan は委譲する。
- `from_stable_id` で全フィールドを parse したうえで `new(src,dst)`
  との完全一致を要求するのは、ID 文字列が自己記述的
  （provenance から値を読める）でありつつ、手組みの非整合値による
  「存在しない変換」の偽装を防ぐため。
- `ResamplePlan::new` の巨大係数表確保は既存の挙動（OOM で
  abort し得る）。本変更はその契約を変えないが、
  `ResampleParams::new` は表を持たないため極端な有理比でも
  ID だけを安全に得られることをテストで固定した。
- `radius` の型は `ResamplePlan` と同じ `usize`（API 一致）。
  `usize` 幅が直列化値を制限するのは、そもそも係数表が構築不能な
  範囲に限られ、本用途では実害なしと判断。

## Related Experiments

### Adopted

（なし — 既知の解法であり Experiment Log は作らない）

### Rejected

（なし）

## Validation

- `mise run fmt` / `fmt-check` — pass（整形済み・差分なし）。
- `mise run check` — pass（workspace `--all-targets`）。
- `mise run lint` — pass（`clippy -D warnings`、pedantic 含む）。
- `mise run test` — pass（workspace 全スイート 0 failed。
  ruviform-numeric は resample 7 件 + 新規契約テスト 5 件、
  doctest 6 件を含む）。
- `mise run build` — pass。
- `mise run test-rt` — pass（`rt_processing_paths_do_not_allocate`
  が `params()`/`ResampleParams::new`/`from_stable_id`/`output_len`
  の alloc-free を `assert_no_alloc` 配下で検証）。
- `git fetch origin main && git merge origin/main` — 実施済み
  （Already up to date。依存追加なしのため `Cargo.lock` 差分なし）。

未実施の検証:

- `bench`/`bench-smoke` — 本変更は処理性能経路を変更しない
  （係数表の構築・演算は不変、getter 経由化のみ）ため未実施。
- AL 側での採用（`transform(plan)` を `stable_id` 等へ置き換える
  audio-lens 側の変更）— ruviform 側の API 提供が本 Issue の範囲で、
  AL の永続スキーマ変更は audio-lens の対応 Issue で実施する。
- 32bit プラットフォームでの検証 — `usize` 依存の radius 値は
  64bit 前提の巨大値になり得るが、対象環境（Windows/macOS/Linux
  x64）は全て 64bit。

## Risks

- `stable_id` の golden は現在の係数導出（`0.94·min(ratio,1)` 等）を
  pin する。導出規則を変える修正は golden/契約テストが検出するため、
  その際は `CONTRACT_VERSION` の増分と既存永続 ID の扱いを
  レビューで判断する必要がある（version 運用は型 doc に明記済み）。
- `ResamplePlan` が `params` フィールドを持つ内部構造へ変わったが、
  公開 API の振る舞い・`PartialEq` 結果（意味的に params 一致と
  等価）は不変。`resample`/`resample_into` の出力も変更なし
  （係数導出式・評価順を同一に保った）。
- `ALGORITHM_ID` は AL 側の `RESAMPLE_ALGORITHM` 値と一致させたが、
  AL の `algorithm_version`（AL 側管理の `"1"`）と本型の
  `CONTRACT_VERSION`（u32）は別系統 — AL 移行時の写像は
  audio-lens 側で決める。

## Follow-ups

- audio-lens 側: `normalize.rs` の `transform()` を
  `ResamplePlan::stable_id`/`ALGORITHM_ID` ベースへ置き換え、
  `NormalizedParams.transform` の永続・dedup を本契約へ移行する
  （audio-lens #102 R4 の対応 Issue）。移行は AL 側の
  normalized_pcm schema 変更を伴うため AL 側の pre-release
  互換ポリシーに従う。
- `ResamplePlan::new` の巨大係数表確保は OOM abort し得る
  （既存の潜在問題・本 Issue 非因果 — `ResampleParams::new` で
  識別だけは可能になったが、plan 構築自体の失敗契約は未整備。
  別 Issue として報告候補）。
