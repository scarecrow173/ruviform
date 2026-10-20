---
id: "TSPEC-034JgliZJcJ6cl2OeZLkuZ"
type: "test-spec"
status: "proposed"
title: "オーディオデバイス境界ポートと cpal adapter の契約保証"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
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
    - "../adr/select-audio-device-backend.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/audio-device-boundary-and-poc.md"
    - "../specs/audio-device-backend-evaluation-poc.md"
  derived-by: []
  verifies:
    - "../specs/audio-device-backend-evaluation-poc.md"
    - "../designs/audio-device-boundary-and-poc.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# オーディオデバイス境界ポートと cpal adapter の契約保証

## Purpose

Issue #23 のデバイス境界ポート（`ruviform_audio_core::device`）と cpal adapter（`ruviform-audio-device`）の振る舞い契約——trait の object safety と `Send` 境界、コールバック型の HRTB、`ErrorKind → DeviceError` のコンテキスト依存写像、非 cpal 実装でもポートが組み立てられる差替可能性、interleave pack の有界性（chunk 分割・ゼロフィル）、cpal 依存の非露出——の「なぜテストが存在するか」を保持する。[spec](../specs/audio-device-backend-evaluation-poc.md) の受け入れ基準と[設計](../designs/audio-device-boundary-and-poc.md)（approved）の検証設計を、後続の backend 追加・trait 変更・adapter 内部再構成の際に「どの振る舞いを守るべきか」を判断できる形で固定する。廃止してよいのは、ポート契約自体を変更する ADR/spec 改訂（例: trait 形状の見直し、エラータクソノミ変更、正準フォーマット方針の変更）が行われたとき。

## Feature

デバイス境界ポートと cpal adapter の契約: `DeviceEnumerator`/`Device`/`OutputStream` の object-safe ポートとコールバック型、`DeviceError` へのエラー正規化、`AudioBufferViewMut::try_new` と interleave 書込経路、cpal 依存の adapter 内閉じ込めが、設計が定める不変条件を満たすこと。

## Rules

- **ポート trait は object-safe で `Box<dyn>` 経由のやり取りが成立する**。`DeviceEnumerator`/`Device`/`OutputStream` はジェネリック・`Self` 返却・関連型を持たず、`Box<dyn Device>`/`Box<dyn OutputStream>` は trait object 経由で `Send` を満たす。`DeviceEnumerator` にはスレッド境界を課さない。
- **コールバック型は HRTB と非確保を満たす**。`OutputDataCallback = Box<dyn for<'a> FnMut(AudioBufferViewMut<'a, f32>, OutputCallbackInfo) + Send + 'static>` へ普通の `FnMut + Send + 'static` クロージャが強制変換（coerce）可能であること（`for<'a>` 推論の成立）。`OutputErrorCallback` が受け取る `DeviceError` は `Copy` であり確保を伴わない。
- **`AudioBufferViewMut::try_new` は借用のみ・非確保で構成され、不正入力を `Err` で表現する**。`channels == 0` または `data.len() != channels * frames` で `Err(Error::InvalidValue)`。正常構成では `frames`/`channels` が引数どおり観測され、`copy_to_interleaved` と結合して channel-major 配置で書き出せる。
- **エラー正規化は `map_error(e, ctx)` 1 関数に集約され、コンテキストで結果が変わる**。cpal `ErrorKind` → `DeviceError` の写像は設計の写像表どおり: 制御経路では `DeviceNotAvailable → DeviceNotFound`、err_cb 経路では `DeviceNotAvailable → StreamInvalidated`、`DeviceChanged` は `Backend("device_changed")` の非致命アドバイザリ、`Xrun` は err_cb のみ `Xrun`。`HostUnavailable`/`PermissionDenied`/`InvalidInput`/`ResourceExhausted`/`BackendError`/`Other` と将来の未知 variant（wildcard）は `Backend("...")` 系に倒れる。呼出側は個別の写像ルールを持たない。
- **非 cpal 実装でもポートを実装できる（差替可能性）**。モックバックエンドが `DeviceEnumerator`/`Device`/`OutputStream` を実装し、`Box<dyn>` 経由で列挙 → 既定デバイス → open → スクリプト発火のコールバック → start/stop → `now()` の流れをハードウェアなしで踏める。trait シグネチャにバックエンド固有情報が混入しない。
- **interleave 書込経路は有界・非確保で「未定義データを送出しない」**。`n = data.len() / channels` フレームのみ view に写像し、残尾 `data.len() % channels` は dst 側をゼロフィル。`n <= frames_capacity` では露出 scratch をゼロフィルしてから user コールバックを呼び `copy_to_interleaved` で pack。`n > frames_capacity` では容量単位に chunk 分割し、各 chunk でスクラブ再借用 + `try_new` 再構成（move 済み view を使い回さない）。`copy_to_interleaved` 失敗時も dst 残部をゼロフィルする。
- **コールバック経路の非 alloc を `rt_` レーンで検証する**。view 構成 + `copy_to_interleaved` + テスト用 user cb を包んだ `assert_no_alloc` 試行テストは `rt_` プレフィックス命名とし、`mise run test-rt` で選択される。統合不能時のフォールバック（コードレビュー＋静的 grep 証跡）は実験ログ側の責務であり本 spec の保証対象外とする。
- **cpal 依存は adapter クレートに閉じる**。`cargo metadata` で cpal が `ruviform-audio-device` の `[dependencies]` にのみ存在し、`ruviform-audio-core` は audio-device なしで `cargo check` に成功する。両クレートの公開 API 面に cpal 由来の型・エラー・feature 名が存在しない。

## Examples

- Given: `FnMut(AudioBufferViewMut<'_, f32>, OutputCallbackInfo) + Send + 'static` なクロージャ。When: `OutputDataCallback` 型の変数へ代入する。Then: コンパイルが通る（HRTB `for<'a>` の推論成立）。
- Given: `try_new(&mut scratch[..6], 2, 3)`。When: view を構成。Then: `channels()==2`・`frames()==3`。`try_new(&mut scratch[..5], 2, 3)` や `try_new(.., 0, n)` は `Err(Error::InvalidValue)`。
- Given: `ErrorKind::DeviceNotAvailable` の cpal エラー（値構成可能な場合）。When: `map_error(e, Control)` と `map_error(e, ErrCallback)` を呼ぶ。Then: 前者は `DeviceNotFound`、後者は `StreamInvalidated` を返す。値構成不可能な場合は `DeviceError` 側の `PartialEq`/`Display`/`Copy` 性質で契約を検証する。
- Given: フェイク `Device`（モック）。When: `open_output_stream` に `Box<dyn FnMut ...>` のコールバックを渡し `start()` → スクリプト発火 → `stop()`。Then: `Box<dyn Device>`/`Box<dyn OutputStream>` 経由のまま全ステップが完了し、コールバックが期待回数・期待 view 形状で呼ばれる。
- Given: `channels=2`、`frames_capacity=4` の scratch と `data.len() = 2*10` の合成 interleaved dst（`n=10 > capacity`）。When: interleave 書込経路を駆動する。Then: 4+4+2 の chunk に分割され各 chunk で view が再構成され、user cb 未記入部分を含む dst がすべて既知値（ゼロフィルまたはコールバック出力）になる。
- Given: `data.len() = 7`・`channels=2`（`n=3`、残尾 1 要素）。When: 書込経路を駆動。Then: 残尾の dst 要素はゼロフィルされ、view は 3 フレームとして構成される。
- Given: workspace の `cargo metadata` 出力。When: `cpal` を持つパッケージを検索する。Then: `ruviform-audio-device` のみが該当し、`ruviform-audio-core` の依存一覧に cpal（および adapter クレート）が現れない。

## Guarantees

- engine が `Box<dyn>` 経由でバックエンドを差し替え可能であり、第 2 backend 追加時に trait 形状が破壊的変更を要求しない。
- err_cb 経路を含む全エラーが `Copy` な `DeviceError` に正規化され、RT-adjacent 経路で確保が発生せず、バックエンド固有エラーがポート利用者へ漏れない。
- コールバック配管が「open 時 1 回の scratch 確保 + per-callback 借用 view」の構造を維持し、`BufferSize::Default` の可変フレームや `Fixed` 非尊重 backend でも確保なし・有界で動作する。
- 「非対応」が `Option`/enum variant/`DeviceError::{Unsupported, UnsupportedConfig}` で型表現され、panic や bool フラグに退行しない。
- interleave 境界が cpal `Data` との接点に限定され、planar f32 正準が内部表現から侵食されない。

## Non-goals

- 実機ハードウェア依存の振る舞い（実デバイス列挙・ストリーム実測・`BufferSize::Fixed` 尊重度・xrun 頻度・`Send`/`Sync` の backend 実測・ALSA vs PipeWire 差・WASAPI xrun 未報告）— CI 不能であり `docs/impl/exp/` 実験ログの手動検証側の責務。
- cpal/OS 内部のコールバック thread での alloc/ブロッキング実測 — `assert_no_alloc` は adapter 自前コードのみ計測でき、cpal 内部は計数型 `#[global_allocator]` example による実験ログ側の観測対象。
- engine 層 `ruviform-audio` のセッション調停・`RtContext` mint・`MasterClock` 配線（設計の Deferred Concerns。`OutputCallbackInfo` の情報コンテナ形状のみ保証）。
- 入力/duplex API、`open_timeout`、WASAPI exclusive/ASIO、F64/整数フォーマット経路、任意チャンネル数の `ChannelLayout` variant（いずれも据え置き項目）。
- 時間変換・バッファ不変条件など audio-core 既存データモデルの契約（[media-data-model-api.md](media-data-model-api.md) が保証済み。本 spec は `try_new` 追加分のみ扱う）。

## Risk

- object safety や `Send` 境界が崩れると、engine がポート経由で backend を交換する前提が成立せず、adapter 交換が API 破壊を伴う大改正になる。
- `ErrorCtx` 別写像が呼出側へ分散すると、`DeviceNotAvailable` のようなコンテキスト依存 variant が経路で不整合になり、デバイス抜去を「未発見」や「誤ったストリーム喪失」として処理する。
- ゼロフィル規約（残尾・露出領域・コピー失敗時）が欠けると、古い scratch や未初期化 dst がそのまま送出され、聴覚上のノイズとして原因追跡が困難な欠陥になる。
- chunk 分割で move 済み view を使い回す退行が起きると、借用規則違反や誤ったフレーム範囲の書込みにつながる。
- cpal 型が公開面へ漏れると、backend 交換時に公開 API を破壊せざるを得ず、media-io ADR の抽象境界契約が失われる。
- `rt_` レーンから alloc 検証が外れると、コールバック内確保の回帰をコードレビュー依存でしか検出できなくなる。
