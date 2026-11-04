# ruviform-io-file

Ruviform の汎用ファイル I/O 基盤 — IO-01（Issue #283 v0.4）の
staged-publish 契約を実装する非 RT 専用クレート。
AudioLens（#143）・Synvelis（#84）・Ruviform 本体（#305/#306）が共有する
「完成ファイルの安全な公開」プリミティブの型状態・結果語彙・回復所有権を
確立する。実 OS バックエンド（atomic publish・fsync）は IO-IMP-02
（Issue #296）に委ね、本クレートは公開契約 + Fault Injection Mock 駆動の
決定論的パイプラインを提供する。

## 責務

- `StagedFile` — `Write + Seek + Send` の temp 書込み面。内部 `File` や
  clone 可能ハンドルは一切公開しない。`seal(self)` で `SealedFile` へ
  消費遷移する。
- `SealedFile` — `Write`/`Seek` を持たない sealed 状態。publish・abort・
  読み戻し borrow（`open_readback(&mut self)` → `Read + Seek` の
  `ReadbackFile<'_>`、非 File ハンドル）のみ。読取 borrow 中は publish
  不可（`&mut` borrow が所有権をブロック）。
- `PublishOptions` — mode（`CreateNew`/`Replace`）× visibility
  （`RequireAtomic`/`BestEffort`）× durability（`BestEffort`/`FileSync`/
  `FileAndDirectorySync`）の独立3軸。
- `PublishReceipt` / `PublishFailure` — `PublishEffect`
  （`NotPublished`/`Published`/`Indeterminate`）・`PublishPhase`・
  `DurabilityEvidence`（要求値ではなく観測済みの証跡）・`TempState`。
  `Indeterminate` の temp は自動削除しない。
- `RecoveryHandle` / `RecoveryInfo` / `RecoveryObservation` —
  残存 temp の inspect/cleanup を行う所有権ハンドル。Drop は削除しない。
- `abort` / `Drop` — 明示 cancel と best-effort temp 掃除（Drop は
  成功経路でも完全経路でもない）。
- private `PlatformFileOps` — OS 操作の抽象境界。validate・file sync・
  close 準備・publish・directory sync・cleanup・inspect の各段階を
  分離し、`io::ErrorKind::Unsupported`/`AlreadyExists` の規約で公開
  kind へ正規化する。実 OS 実装は Issue #296。

## 層・許可依存

- 層: 汎用ファイル I/O 基盤（メディア I/O より下位・Primitives 同様の
  相互依存なし位置）。
- 許可依存: なし（std のみ。Media/Codec/Serde 非依存）。
- `default = []`・feature なし・`publish = false` 継承・MSRV 1.89。

## RT 契約

- 全 API 非 RT — filesystem I/O にブロックし、alloc も行う。
  RT オーディオコールバックからは絶対に呼ばない。

## 実装 Issue

- Issue #283: IO-01 v0.4 設計契約（型状態・readback borrow・効果三分類）
- Issue #295: 本クレートの新設・公開契約・Mock 駆動 publish（IO-IMP-01）
- Issue #296: IO-IMP-02 — 実 Linux/macOS/Windows atomic publish・
  durability・recovery（本クレートの private adapter 契約に実装する）
- Issue #305/#306: 旧 Media File API 撤去・消費者移行・構成/OS/障害/消費者
  受入ゲート
