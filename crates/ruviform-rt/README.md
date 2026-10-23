# ruviform-rt

Ruviform メディア基盤の RT（リアルタイム）契約定義と RT-safe プリミティブを担うクレート。
スケジューリング・I/O は持たない。

## 責務

Issue #21 で確定・実装済みの RT 基盤 API 一式:

- `Processor` / `RtProcessor` — `prepare`（非 RT）/ `process`（RT）分離の処理契約
- `RtContext` — RT 区間を表す能力トークン
- `MasterClock` — 共有クロック
- `Error` / `Result` — クレート共通エラー型
- RT-safe プリミティブ（固定容量・事前確保の受け渡し構造）:
  - `channel` — SPSC リング
  - `retire_channel` — 遅延 drop 用 retire チャネル
  - `slot` — 不変スナップショット交換

## 層・許可依存

- 層: Primitives（最下位、設計ベースライン クレート表 A.2）
- 許可依存: なし。外部同期クレート（`rtrb`、`arc-swap`）は非公開の実装詳細であり、
  その型を公開 API に露出しない。

## RT 契約

本クレートは契約定義サイト。全公開 API は RT-suitable（no-alloc・非ブロッキング・有界）
であるか、`**RT contract:**` 項目ラベルで明示的に non-RT only と標識されるかの
どちらか — RT ADR の「保証範囲は API ごとに定義する」規則に従う。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「RT／非 RT の分離方式」「RT 契約」）
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- RT 基盤実装 ADR: docs/adr/rt-foundation-primitives-implementation.md
- Issue #21 RT 基盤 API 設計: docs/designs/rt-foundation-api-design.md
