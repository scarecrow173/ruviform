---
id: "IMPL-034QsKJh8joRi3VcMuhrPP"
type: "impl"
status: "completed"
title: "Issue #183 BandFilter shared spectrum and per-call scratch API"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/183"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-183-bandfilter-shared-spectrum-and-per-call-scratch-api.md"
    modified:
      - type: "file"
        path: "crates/ruviform-numeric/src/bands.rs"
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

# Issue #183 BandFilter shared spectrum and per-call scratch API

## Summary

`ruviform_numeric::bands::BandFilter` の保持スペクトルを不変共有体
`BandSpectrum` として分離し、呼出ごとの排他スクラッチ `BandScratch`
を外部注入する API を追加した。入力 1 本あたり forward FFT 1 回の
まま、`&`/`Arc` 共有のスペクトルに対して (channel, band) タスク
単位の scratch で帯域抽出を並列化できる（AudioLens #71 の需要）。
`BandFilter` は `BandSpectrum` + `BandScratch` の逐次実行向け便宜型
として再構成し、公開契約・出力規則は不変。

## Context

- Source: ruviform issue #183 / audio-lens issue #71。
- 現行 `band`/`band_into` は `&mut self`（内部 scratch 保持）のため、
  52 task/pair の並列化でタスクごとに入力全長の forward FFT を再計算
  する必要があった。読み取り対象は不変の `spectrum_re/im` と `plan`
  のみで、`RealFftPlan` は既に「複数スレッドで `&` 共有できる」契約
  を持つ。
- 既存 `BandFilter`/`bandpass`/`bandpass_many` の契約・出力規則は
  不変（additive のみ・破壊的変更なし）。

## Implementation

### Changes

- `crates/ruviform-numeric/src/bands.rs`
  - モジュール doc: 「共有スペクトルと並列化（Issue #183）」節を追加し、
    `BandSpectrum` の `Send + Sync` 共有・`BandScratch` の `&mut` 排他・
    同一 `fft_size` での scratch 使い回し・RT 契約を明文化。bin 写像
    規約とゼロ埋め規約の記述を `BandSpectrum` 込みに更新。
  - `BandSpectrum`（新規）— `plan`・`bin_hz`・`len`・`spectrum_re/im`
    の不変共有体。`new(samples, sample_rate)` で forward FFT を 1 回
    実行し、`len`/`is_empty`/`fft_size`/`bin_hz`/`memory_bytes` と
    `band_into(&self, &mut BandScratch, lo, hi, out)` を提供。
    全フィールドが値型のため `Send + Sync` を自動実装し、doctest で
    `std::thread::scope` からの `&` 共有を実演する。
  - `BandScratch`（新規）— `ws`・`masked_re/im`・`real`・構築元変換長
    `n` を持つ排他スクラッチ。`BandScratch::new(&spectrum)` で確保し、
    `memory_bytes` を提供。
  - `BandFilter` — `spectrum: BandSpectrum` + `scratch: BandScratch`
    の 2 フィールドに再構成し、全メソッドを委譲。`band_into` は
    `spectrum.band_into(&mut self.scratch, ..)` を呼ぶため出力規則は
    構造的に同一（bit-exact）。`memory_bytes` の合計値も不変。
  - テスト 3 件追加: `band_spectrum_matches_band_filter_bit_exact`
    （len×帯域行列で `band`/`band_into` と bit-exact）、
    `band_spectrum_shared_across_threads`（scoped thread での共有 +
    `Send + Sync` 静的保証）、`band_scratch_contract`（変換長不一致
    の拒否・同一 n での使い回し・`memory_bytes` 合計一致）。
- `crates/ruviform-numeric/src/lib.rs`
  - `rt_spectral_paths_do_not_allocate` に `BandSpectrum` +
    `BandScratch` 経由の `band_into` を `assert_no_alloc` 窓内へ追加。
- `crates/ruviform-numeric/README.md`
  - `bands` 責務に `BandSpectrum`/`BandScratch` 分離 API と #183 を追記。

### Decisions During Implementation

- `BandScratch` に構築元変換長 `n` を保持し、`band_into` が
  `scratch.n != plan.n()` を `Error::InvalidValue("scratch")` で拒否
  する契約とした。これがないと、別変換長の scratch では
  `masked_re[lo..hi]` のスライス index が panic し得る（公開 API の
  panic-free 契約違反）。`n` が等しければ全バッファ長が一致するため
  1 フィールドの検証で足り、同一 `fft_size` の `BandSpectrum` 同士で
  scratch を使い回せる実用上の利点もある。
- 検証順序は `out.len()` → `scratch.n` → `len == 0` 早期 return とし、
  `BandFilter::band_into` 従来の先頭検証（`out` 長）を維持した。
- review-protocol で `BandSpectrum::band`（所有版）を除去した。
  `BandFilter::band` は内部 scratch 一致が不変条件として保証されるため
  `let _ =` で握り潰せるが、`BandSpectrum` では呼出側が scratch を
  供給するため不一致時にゼロ波形を黙って返す API になる。`Err` を
  返せない所有版は「誤用を隠す」形になるため、Issue が要求する
  `band_into` のみに絞った（YAGNI）。
- `BandFilter` を `BandSpectrum + BandScratch` の便宜型として再構成
  した（issue 本文の選択肢）。mask→IFFT→切戻しの実装を 2 重化せず、
  既存 API の bit-exact 等価を構造で保証する。

## Related Experiments

### Adopted

（なし — 既存 `BandFilter` との構造分離のみであり Experiment Log は作らない）

### Rejected

（なし）

## Validation

- `mise run fmt` / `mise run fmt-check` — pass。
- `mise run check` — pass（workspace 全ターゲット）。
- `mise run lint` — pass（`cargo clippy --workspace --all-targets -- -D warnings`）。
- `cargo test -p ruviform-numeric` — 108 unit + 7 doctest 全 pass
  （新規 `bands::BandSpectrum` doctest・3 契約テストを含む）。
- `mise run test` — workspace 全 113 テストバイナリ pass・0 fail。
- `git merge origin/main` — Already up to date（fetch 済み・差分なし、
  生成物の再生成不要）。
- review-protocol（承認代替）: CONFIRMED P1 1 件
  （`BandSpectrum::band` が scratch 不一致を黙ってゼロ返しする）
  → `band` 除去で解消。残存 P0/P1 なし、verdict: ACCEPT。

## Risks

- `BandSpectrum::band_into` の RT-suitable 表記は `assert_no_alloc`
  窓での機械検証 + 従来 `band_into` と同一実装に基づく。ブロッキング
  なし・有界時間であることは実装レビュー範囲（既存契約と同じ）。
- `BandScratch` を `Arc` 等で共有する使い方は `&mut` 借用規則で
  コンパイル時に排除されるが、doc でも「共有は想定しない」と明記済み。

## Follow-ups

- AudioLens 側（audio-lens#71）で `band_gain` 並列化へ
  `BandSpectrum` + `BandScratch` を接続するのは別リポジトリ作業。
