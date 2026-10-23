# ruviform-video-proc

Ruviform メディア基盤の基底映像処理カーネルクレート: `VideoFrame` と
video-core のピクセル/色型に作用する色変換・スケーリング・コンポジット系
カーネル、および GPU コンピュートカーネル境界ポートの所有者。
コーデック・demux/mux・I/O は持たない（フレーム・色型は `ruviform-video-core`、
メディア I/O 境界は `ruviform-media-io`）。

## 責務

Issue #26 で実装済みの色変換カーネル:

- `YuvToRgb` — planar `Yuv420p`/`Yuv422p`/`Yuv444p` → packed `Rgba8`
  （matrix/range 対応・chroma upsampling。`ColorInfo` 更新規則と bit-exact
  等価契約はモジュールドキュメント参照）
- `RgbToYuv` — packed `Rgba8` → planar `Yuv420p`/`Yuv422p`/`Yuv444p`
  （matrix/range はコンストラクタ引数で固定、Q16 ドメインでのセル平均
  chroma downsampling、bit-exact 等価契約）
- `Rgba8ToRgbaF16` / `RgbaF16ToRgba8` — `Rgba8` ↔ `RgbaF16` チャネル正規化
  （`v/255` ↔ `round(clamp(v)·255)`、`ColorInfo` は不変で引き継ぐ。
  順方向は ulp 有界・逆方向は bit-exact の等価契約）

カーネルはベースラインの二相処理形状: 非 RT の `prepare`/コンストラクタが
係数とワークエリアを固定して準備済み処理オブジェクトを生成し、その
`process(&mut self, src, dst)` はアロケーションなし。`RtContext` 引数・
`Processor` trait 実装は取らない — `ruviform-rt` の `Processor` への実装は
後方互換の追加拡張とする（ADR D1）。全カーネルはスカラー参照実装 —
将来の最適化バックエンドが bit-exact か ulp 有界かは各カーネルが文書化する。
スケーリング・コンポジットカーネルは後続 Issue へ保留。

GPU コンピュートカーネル境界ポート（ベースライン「契約型・境界ポートの所有」表の
ユーザー側ポート）の所有を宣言するのみ — ポート trait 自体は `ruviform-gpu`
実装 Issue に委譲する（ADR D4）。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 A.2）
- 許可依存: Primitives（`ruviform-math`, `ruviform-rt`）+ Domain foundations
  （`ruviform-video-core`）。使用中: `ruviform-video-core`（フレーム・色型）と
  `ruviform-media-core`（`Error`/`Result` — ADR D7）。`ruviform-rt` 依存はなし
  （カーネルは `RtContext` 引数を取らない — ADR D6）。
- 同階層クレート（`ruviform-dsp`, `ruviform-media-io` 等）への依存は禁止。

## RT 契約

`process` API は設計上 RT-suitable（アロケーションなし・非ブロッキング・有界）
だが、保証は API ごとの `**RT contract:**` ラベルでのみ宣言される —
ラベルなき API は RT 保証されない。初期化/`prepare` API は非 RT。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、`ruviform-video-proc` 行、「契約型・境界ポートの所有」GPU 境界行、Domain foundations 層順）
- カーネル API・色/HDR スコープ ADR（D1/D4/D6/D7）: docs/adr/video-proc-kernel-api-and-color-hdr-scope.md
- Issue #26（クレート新設・色変換カーネル実装）
