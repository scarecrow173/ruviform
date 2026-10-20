---
id: "TSPEC-034J5K0lC6EBv1PrsliYd2"
type: "test-spec"
status: "approved"
title: "メディアデータモデル API の契約保証"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/18"
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
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
  derived-by: []
  verifies:
    - "../specs/implement-media-data-model-api.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# メディアデータモデル API の契約保証

> **記録注記**: 本 test spec は Issue #18 の実装（PR #39）完了後の遡及整備であり、既存テスト群が実際に保証している契約の意図を記録する。

## Purpose

Issue #18 で確定したデータモデル API の振る舞い契約——有理数時刻の正確性、planar 正準バッファ、plane/stride モデル、RT 契約レーン——の「なぜテストが存在するか」を保持する。[spec](../specs/implement-media-data-model-api.md) の受け入れ基準と[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)「データモデル」節の決定事項を、実装の改変や後続クレート追加の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂が行われたとき（例: 時間モデルの中核表現変更、planar 正準の見直し）。

## Feature

メディアデータモデル API の契約: media-core の有理数時間・タイムコード・トランスポート値型、audio-core の planar 正準バッファと境界変換、video-core の plane/stride/フレームモデルが、spec が定める不変条件を満たすこと。

## Rules

- **時間変換は正確な有理数演算で行う**。`MediaTime::convert` は表現可能な場合に厳密値を返し、表現不可能な場合のみ呼び出し側指定の `Rounding`（Floor / Ceil / TowardZero / AwayFromZero / NearestEven）を適用する。丸めは負値（pre-roll）でも符号を再適用した対称な振る舞いとし、オーバーフローは `Err(TimeOverflow)` で表出させる。
- **異なる timebase 間で `Eq` / `Ord` / `Hash` が一貫する**。`MediaTime` / `MediaDuration` の比較は正確な有理数比較（`u128` マグニチュード）に基づき、0 はどの base でも等しい。`Hash` は最低項正規化され、cross-base の `Eq` と矛盾しない。
- **ドロップフレームは表示・ラベリング規約であり整数演算のみで実装する**。`Timecode::from_media_time_drop` は NTSC 系レート（30000/1001・60000/1001、正規化後の TimeBase で判定）のみ受け付け、それ以外は `Err(UnsupportedDropFrameRate)`。負時刻はエラーとする（`Err(InvalidValue)`）。
- **planar ↔ interleaved は境界で往復変換可能**。`from_interleaved` / `copy_to_interleaved` / `copy_from_interleaved` はチャンネルメジャー配置で往復一致し、長さ不一致は `Err(InvalidValue)`。
- **plane 寸法は `PixelFormat::chroma_subsampling()` から一意に導出する**。420/422 の chroma plane は `ceil(w/sx) × ceil(h/sy)`。奇数サイズでも一貫する。
- **stride 契約は所有レイヤーで守る**。`stride >= row_bytes` でない plane ビューは構築を拒否し、backing 長は `stride*(h-1) + row_bytes` 以上を要求する。`from_shared_planes` は plane 数・stride・backing 長を検証する。
- **共有 backing では可変ビューを供給しない**。`Arc` 共有が生きている間 `VideoFrame::plane_mut` は `None` を返す（`Arc::get_mut` 失敗）。
- **RT-suitable とラベルされた操作は確保を伴わず `rt_` レーンで検証する**。`rt_` プレフィックスのテストは RT-suitable API のみを対象とし、`mise run test-rt`（`cargo test --workspace rt_`）で選択される集合を意図した RT テスト群に保つ（非 RT テスト名に `rt_` 部分文字列を含めない）。

## Examples

- Given: `MediaTime` が base `1/48000`。When: base `1/90000` へ `convert` する。Then: `48000 → 90000` の比が整数になる場合は厳密値、ならない場合は指定 `Rounding` で丸められる。`i64::MAX` 等の極端値では `Err(TimeOverflow)`。
- Given: `-3/2` の丸め。When: 各モードを適用する。Then: Floor `-2`、Ceil `-1`、TowardZero `-1`、AwayFromZero `-2`、NearestEven `-2`（負値でも対称）。
- Given: `TimeBase::new(60000, 2002)`。When: drop-frame 変換する。Then: gcd 正規化で `30000/1001` と同一視され受理される。NTSC 非系レートでは `Err(UnsupportedDropFrameRate)`。
- Given: 2ch×N frames の interleaved サンプル列。When: `from_interleaved` → `copy_to_interleaved`。Then: 入力と同一列に往復する。`dst.len() != channels*frames` なら `Err(InvalidValue)`。
- Given: `Yuv420p` 幅 5・高 3。When: `new_owned` でフレーム構築。Then: chroma plane は `ceil(5/2) × ceil(3/2)` = `3×2` に導出される。
- Given: `from_shared_planes` に `stride < row_bytes` または不足 backing の `SharedPlane`。When: 構築する。Then: `Err(InvalidValue)`。
- Given: `VideoFrame` を `Clone` した参照が生存中。When: `plane_mut(i)` を呼ぶ。Then: `None` が返り、`drop` 後に `Some` へ戻る。
- Given: `mise run test-rt`。When: `rt_` フィルタで選択されるテスト集合を確認する。Then: 各クレートの `rt_` プレフィックス・テストのみが選ばれ、非 RT 名（`short_` / `convert_` 等の `rt_` 部分文字列を含むもの）は混在しない。

## Guarantees

- timebase 変換・比較・ハッシュが f64 を介さず、長時間メディアでも累積ドリフトや不一致を生じない。
- interleaved 変換が境界でのみ行われ、内部表現が interleaved 前提に退行しない。
- plane モデルで stride パディングと backing 長が明示検証され、行の重なり・範囲外アクセスを型契約で防ぐ。
- `Arc` 共有 backing への可変アクセスがライフタイム上有効な間は供給されず、RT パスで共有データを暗黙に書き換えない。
- `rt_` レーンの選択集合が意図した RT 契約テストのみであり、RT-suitable API の no-alloc 契約が回帰検出される。

## Non-goals

- SIMD・音声処理・DSP カーネルの振る舞い（後続クレート / PoC の検証対象）。
- media-io の demux / decode の振る舞い、`Packet` やバックエンド境界（`ruviform-media-io` 実装 Issue の検証対象）。
- 色変換・色管理の実装（`ruviform-video-proc` の責務。本 spec はメタデータ型の表現力のみを扱う）。
- パフォーマンス・ベンチマーク、OS リアルタイム保証（RT ADR の範囲外注記どおり提供しない）。
- `mise run test-rt` の部分文字列フィルタ自体の精度（doctest 名の `convert_exact` が `rt_` を含む既知の誤検出は substring マッチの制約であり、本 spec の保証対象外）。

## Risk

- 時間変換が f64 や暗黙丸めに退行すると、長時間素材での編集・A/V 同期ずれが静かに蓄積し、発見時には広範な再計算が必要になる。
- cross-base の `Eq`/`Ord`/`Hash` が崩れると、マップキーや整列が base 依存で不安定になり、同期処理が再現困難な不具合を持つ。
- stride・backing 検証が削られると、plane ビューが重なり行や範囲外を露出し、映像データ破壊・unsafe 等価の破壊的振る舞いにつながる。
- 共有 backing の `plane_mut` ゲートが外れると、参照カウント済み供給元データを RT パスが書き換え、供給側と競合する。
- `rt_` レーンの選択が汚染されると、RT 契約テストの意図が薄れ、no-alloc 契約の回帰を見逃す。
