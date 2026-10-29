# ruviform-observability

Ruviform の共有観測プリミティブ — `foundation-observability-minimal-api`
スケッチ（Issue #101）の最初の実消費者面として、Issue #121 の L1
diagnostics が使う **counter pair** を提供する最小クレート。

## 責務

- `counter() -> (Counter, CounterReader)` — 共有 `Arc<AtomicU64>` セルを
  持つ計数ペアを生成する。
- `Counter`（書き手側）: `increment`（`Relaxed` `fetch_add`）・`read`・
  `reader()`。`Clone` は同一セルを共有する writer ハンドルを増やす。
- `CounterReader`（観測側）: `read`（`Relaxed` load のみ）。

## 層・許可依存

- 層: Primitives（最下位・相互依存なし）
- 許可依存: なし（std のみ）。依存なしの最小構成を維持する。
- 消費方向: `ruviform-plugin-host → ruviform-observability`（Issue #121）。
  `ruviform-rt` からの逆依存は追加しない（foundation DAG/ADR 問題は
  別 Issue — `docs/designs/foundation-observability-minimal-api.md`）。

## RT 契約

- `increment`/`read`/`reader`/`CounterReader::read`: wait-free・no-alloc・
  有界 — `Relaxed` atomic / `Arc` refcount のみ。
- `counter()`（生成）・`Clone`・`Drop`: 非 RT のみ（`Arc` セルの alloc/free）。
- 複数 counter の逐次 `Relaxed` 読みは同一時点の整合 snapshot を
  **保証しない** — 公開契約として `lib.rs` に明記。
- `u64` は modulo 2^64 wrap（現実的構成では到達不能 — saturating 措置なし）。

## 実装 Issue

- Issue #101: 候補・trigger（`foundation-observability-minimal-api`）
- Issue #104: `observability_poc` の `Counter`/`CounterReader` 参照形
- Issue #121: 初消費者 L1 diagnostics（audio counters・`Counter`/`CounterReader` を
  `InstKind::Audio` arm が所有）
