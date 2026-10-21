---
id: "TSPEC-034JNskaqcN34Ki4layRNK"
type: "test-spec"
status: "approved"
title: "FrameRate 型・Timecode シグネチャ・時間変換精度の契約保証"
created: "2026-10-21"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/58"
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
    - "../designs/time-model-extension.md"
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
  derived-by: []
  verifies:
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/42"
  defers: []
  deferred-by: []
---

# FrameRate 型・Timecode シグネチャ・時間変換精度の契約保証

## Purpose

Issue #42 で設計され Issue #58 で実装される時間型拡張——`FrameRate`/`TimeBase` の意味的分離（fps 向き有理数を保持する newtype）、`Timecode::from_media_time(_drop)` の `frame_rate: FrameRate` シグネチャ、丸め誤差累積ポリシーの検証可能な誤差上界——の「なぜテストが存在するか」を保持する。Issue #58 の受け入れ条件（設計文書 Verification Notes・`nominal_fps` の u64 中間演算・精度行列・連鎖プロパティ・`nominal_fps()==0` 拒否回帰・drop 集合維持）を、実装の改変や後続クレート追加の際に「どの振る舞いを守るべきか」「そのテストはまだ必要か」を判断できる形で固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂が行われたとき（例: `FrameRate` の型形状見直し、ドロップフレーム対象集合の拡張、丸めポリシーの変更）。

## Feature

`FrameRate` が `TimeBase` の newtype として「正の fps 有理数」不変条件を構築時に強制し、保持値を `TimeBase` 値として露出しないこと。`Timecode` の `from_media_time` / `from_media_time_drop` が `FrameRate` を受け取り、nominal grid 決定を `FrameRate::nominal_fps()` に移譲した上で、従来どおりのラベル規則・エラー契約を維持すること。時間変換が「呼び出しごとに最大 1 回の丸め」「安定アンカーからの変換」「base-hopping 禁止」のポリシーを前提に、検証可能な誤差上界（単回 ≤ 宛先 0.5 tick、往復 = 中間 base 0.5 tick の元 base 換算 + 元 base 0.5 tick、N 連鎖 ≤ Σ 各ホップ宛先 0.5 tick の共通単位換算）を満たすこと。

## Rules

- **`FrameRate` の不変条件は構築時に強制される**。`FrameRate::new(0, r)`・`FrameRate::new(n, 0)`・`FrameRate::from_fps(0)` は `Err(Error::InvalidValue("frame_rate"))` を返し、正の fps 有理数のみが存在する。保持値は `TimeBase::new` 経由で gcd 正規化される（`new(60_000, 2_002)` ≡ `new(30_000, 1_001)`）。
- **保持する fps 有理数は `TimeBase` 値として取り出せない**。`to_time_base()`・`From<FrameRate> for TimeBase`・`From<TimeBase> for FrameRate` は存在しない。`TimeBase` を返す公開 API は `frame_duration()`（逆数 = 真の tick 長）のみであり、fps 有理数の成分は `numerator()`/`denominator()` で取得する。
- **`nominal_fps()` は `u64` 以上の中間演算で計算される**。round-half-up（`(2*num + den) / (2*den)`）規則で、`u32` 中間演算では `num > ~2^31` で `2*num` がオーバーフローするため。`4294967295/1` → `4294967295`、`1/4294967295` → `0` の極端有理数で破綻しない。
- **`Timecode` の `frame_rate` 引数は `FrameRate` のみ**。旧 `frame_rate: TimeBase` シグネチャは併設されず残存しない（破壊的置換 — `publish = false` の間に限り許容される ADR 決定）。
- **`from_media_time` は `nominal_fps() == 0` の rate を `Err(Error::InvalidValue("frame_rate"))` で拒否する**（nominal grid が 0 に丸められる sub-fps rate はラベル格子を構成できない。現行 `sub_fps_rate_is_rejected` 契約の維持）。
- **ドロップフレーム対応集合は `30000/1001`・`60000/1001` のみ**。`from_media_time_drop` は正規化後にこの 2 レート以外を `Err(Error::UnsupportedDropFrameRate)` で拒否する現行契約を維持する（非 NTSC 一般化・フィールドレートは保留分類）。
- **`elapsed_frames`（`floor(t * rate)`）の厳密整数演算と負時刻拒否は不変**。負 `MediaTime`（pre-roll）は `Err(Error::InvalidValue("negative time"))`。`i128`/`u128` 中間演算・`f64` 非使用を維持する。
- **単回変換の丸め誤差は宛先 base の 0.5 tick 以下**。`MediaTime::convert(to, Rounding::NearestEven)` は round-half-to-even で |誤差| ≤ 0.5 tick（exact-half tie はちょうど 0.5 tick で偶数側に落ちる）。
- **往復変換の誤差上界は「中間 base の 0.5 tick を元 base 単位に換算した値 + 元 base の 0.5 tick」**。`t → B → 元base` の 2 ホップで、各ホップの丸め項を計上する。粗い中間 base 経由では元 base 換算で大きくなり得る（例: `1/48000` → `1/24` → `1/48000` で 1000 tick 級）が上界内に収まる。
- **N 回連鎖の累積誤差は「Σ（各ホップ宛先 base の 0.5 tick）を共通単位に換算した値」を超えない**。同時に、アンカーからの直接変換（ポリシー遵守形）は常に宛先 base の 0.5 tick 以下に収まる対照実験として検証する。
- **誤差測定は整数有理数で行い `f64` を中核経路に持ち込まない**。テスト判定も `i128`/`u128` 有理数比較で行う（`to_seconds_f64` は表示側の既存例外であり精度判定には使わない）。

## Examples

- Given: `FrameRate::new(30_000, 1_001)`。When: `nominal_fps()` / `frame_duration()` を呼ぶ。Then: `nominal_fps() == 30`、`frame_duration() == TimeBase::new(1_001, 30_000)`（fps 有理数と tick 長が逆数関係にあることを型の API で固定）。
- Given: `FrameRate::new(0, 1)`・`FrameRate::new(1, 0)`・`FrameRate::from_fps(0)`。When: 構築する。Then: いずれも `Err(Error::InvalidValue("frame_rate"))`。
- Given: `FrameRate::new(4_294_967_295, 1)`（`u32::MAX/1`）・`FrameRate::new(1, 4_294_967_295)`（`1/u32::MAX`）。When: `nominal_fps()` を呼ぶ。Then: 前者は `u32::MAX`、後者は `0` を返しオーバーフロー・パニックしない（`u64` 中間演算の境界証拠）。
- Given: `MediaTime::new(1, TimeBase::new(1, 1))` と `FrameRate::from_fps(25)`。When: `Timecode::from_media_time` を呼ぶ。Then: `00:00:01:00` の非 drop ラベル。`FrameRate::new(1, 1_000)`（sub-fps）なら `Err(Error::InvalidValue("frame_rate"))`。
- Given: `FrameRate::new(25, 1)`・`FrameRate::new(24, 1)`・`FrameRate::new(50, 1)`。When: `Timecode::from_media_time_drop(MediaTime::ZERO, _)` を呼ぶ。Then: いずれも `Err(Error::UnsupportedDropFrameRate)`。`FrameRate::new(60_000, 2_002)` は正規化で `30000/1001` となり受理される。
- Given: `t = MediaTime::new(1_000, TimeBase::new(1, 48_000))`、中間 base `B = TimeBase::new(1, 24)`。When: `t.convert(B, NearestEven).convert(元base, NearestEven)`。Then: 往復誤差 ≤ `1/24` base の 0.5 tick を `1/48000` 単位に換算した 1000 tick + `1/48000` base の 0.5 tick（実測 1000 tick — 中間で 0 に丸められる）。同一 `t` のアンカー直接変換では宛先 0.5 tick 以下。
- Given: 音声 base（`1/44100`・`1/48000`・`1/90000`）× 映像 rate（`24`・`25`・`30000/1001`・`60000/1001` の `frame_duration()`）の全組み合わせと、値集合 `{0, ±1, 大きな正値, 負値}`。When: `MediaTime::convert` を各方向に呼ぶ。Then: 全組み合わせで誤差 ≤ 宛先 base の 0.5 tick、負値でも丸め規則が対称に作用する。
- Given: 代表 base 集合 `{1/48000, 1/44100, 1/90000, 1001/30000, 1001/60000, 1/25, 1/24}` の巡回連鎖（例: `1/48000 → 1001/30000 → 1/25 → 1/48000`）。When: 連鎖の各ホップを丸め済み値でつなぐ。Then: 累積誤差 ≤ Σ（各ホップ宛先 base の 0.5 tick）の共通単位換算。同じ始点・終点のアンカー直接変換は宛先 0.5 tick 以下。

## Guarantees

- fps 向き有理数と tick 長有理数の逆数混同が型で防がれ、`MediaTime::new(v, fr.into())` 型の契約違反値がコンパイルを通らない（`From` impl 非提供・`to_time_base` 非存在で保証）。
- `nominal_fps()` のオーバーフローが `u64` 中間演算で構造的に排除され、`u32::MAX` 級の極端入力でも nominal grid 決定が破綻しないことが境界テストで保たれる。
- `Timecode` のラベル規則（非 drop の nominal 分解・SMPTE drop の 29.97/59.94 限定）がシグネチャ移行後も変わらないことが回帰テストで保たれる。
- 丸め誤差累積ポリシーの検証可能な形（往復上界・N 連鎖上界・アンカー直接の 0.5 tick 保証）が deterministic 列挙テストとして残り、ポリシー違反が実害化する前に検出できる。
- `Error` は `#[non_exhaustive]` のまま既存 variant を再利用し、将来の variant 追加余地が残る（enum ADR 整合）。

## Non-goals

- `Timecode` → `MediaTime` 逆変換 API の検証（設計 Deferred・後続 Issue）。
- ドロップフレーム非 NTSC 一般化・フィールドレートの動作（保留分類 — 現行 2 レート限定の維持のみ検証）。
- `MediaTime::convert` / `MediaDuration::convert` の丸め規則・`Err` 条件そのものの再検証（`media-data-model-api.md` test-spec の既存保証に委譲。本 spec は累積誤差の上界のみを対象とする）。
- A/V 同期・編集タイムラインの実配線での精度検証（#27 以降のエンジン Issue の検証対象。本 spec はプリミティブ層の誤差上界のみ）。
- パフォーマンス・ベンチマーク（整数演算の契約テストであり計量対象外）。

## Risk

- `FrameRate` の不変条件が ctor で強制されない形に退行すると（public フィールド化・無検証構築経路の追加）、ゼロ/負の fps が `Timecode` 計算に流入し、除算パニックや無意味なラベルが発生する。
- `nominal_fps()` が `u32` 中間演算に戻ると `2*num` のオーバーフローが静かに誤 nominal grid を生み、`u32::MAX` 級入力でのラベル分解が壊れる。
- 往復・連鎖の誤差上界テストが失われると、「呼び出しごと 1 回の丸め」実装の回帰（中間値の二重丸め・早期 `i64` 切捨て等）を検出する手段がなくなり、A/V 同期・編集タイムラインの実装が始まってから精度欠陥として顕在化する。
- 旧 `TimeBase` シグネチャの併設が復活すると、fps/tick の意味混同が再び型で防げなくなり、#42 ADR の分離目的が形骸化する。
