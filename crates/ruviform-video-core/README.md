# ruviform-video-core

Ruviform メディア基盤の映像データ型クレート。変換・コンポジット・I/O は持たない —
色変換・HDR 処理は `ruviform-video-proc`、フレームプール・デコーダ機構・
ストリームポートは後続クレートの責務。

## 責務

Issue #18 で確定・実装済みのデータモデル API:

- ピクセルフォーマット: `PixelFormat`
- 色空間・HDR 記述子: `ColorInfo`, `HdrMetadata`, `MasteringDisplayInfo`,
  `ContentLightLevel`, `Chromaticity` とラベリング列挙型
- プレーンビュー: `Plane`, `PlaneMut`, `SharedPlane`
- フレームコンテナ: `VideoFrame`, `VideoFormat`

### 所有権・ライフサイクル

- `VideoFrame` のプレーン backing は参照カウント（`Arc`）: clone は共有であり
  コピーではない。変更には排他所有が必要 — 共有中は `VideoFrame::plane_mut` が
  `None` を返す（`Arc::get_mut` 書き込みゲート）。`plane_mut() == None` が
  観測可能な共有シグナルであり、他の共有調査 API は存在しない。
- 最後の `Arc` の drop で backing が解放されるため、`VideoFrame` を RT パス上で
  最終 drop してはならない — retire 済みフレームは非 RT 側へ引き渡す
  （ベースラインの drop-retreat パターン）。
- 外部確保バッファ（デコーダ等）は `SharedPlane` +
  `VideoFrame::from_shared_planes` のみから入る唯一のゼロコピー供給経路。
  `SharedPlane` にプレーンオフセットフィールドはない（既知の制限 —
  media-io issue で再評価）。

## 層・許可依存

- 層: Domain foundations（設計ベースライン クレート表 A.2）
- 許可依存: Primitives（`ruviform-math`, `ruviform-rt`）+ `ruviform-media-core`。
  実際に使用中: `ruviform-media-core`（`MediaTime`, `MediaDuration`,
  `Error`/`Result`）。

## RT 契約

フレーム・プレーンのアクセサは RT-suitable: `Plane`/`PlaneMut` の借用・
行スライスはアロケーションなしで panic しない。アロケートするコンストラクタ
（`VideoFrame::new_owned`, `VideoFrame::from_shared_planes`）、`VideoFrame` の
`Clone` と `Drop`（参照カウントのライフタイム管理 — 最後の `Arc` の解放で
backing を deallocate）は非 RT 操作: 所有フレームは最終 clone の drop 前に
RT パスから離脱させる。API ごとの保証は各項目の `**RT contract:**` 行に記載される。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「動画データモデル」）
- RT 契約 ADR: docs/adr/realtime-contract-and-verification-policy.md
- video-proc カーネル API・色/HDR スコープ ADR（D5-i — 所有権契約）: docs/adr/video-proc-kernel-api-and-color-hdr-scope.md
