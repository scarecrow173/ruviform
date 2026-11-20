---
id: "DSC-630clapOutputDrain01"
type: "discovery"
status: "final"
title: "Issue #842 CLAP output-drain slice — 調査記録"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/842"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-clap-output-drain.md"
    - "docs/designs/issue-630-clap-output-drain.md"
    - "evidence/rf630/clap-output-drain-verification.md"
  related:
    - "../specs/issue-630-clap-output-drain.md"
    - "../designs/issue-630-clap-output-drain.md"
    - "../test-specs/issue-630-clap-output-drain.md"
---

# Issue #842 CLAP output-drain slice — 調査記録

## 既存機構はゼロ差分で足りる（wire/plugin-core/export/composite）

調査の結果、レポート経路の全層が Issue #253 で既に実装済み:

- `WireCommand::DrainOutputReports`（tag 23）→
  `WireReply::Reports { events }`（`WIRE_MAX_EVENTS` ごとの
  チャンク — 子側は remainder を backlog に保持）。
- export `Entry::drain_output_reports` が plugin の
  `drain_output_reports` で `report_backlog` へ畳み、最古順に
  応答 — 親側 `l1::L1Instance::drain_output_reports` は短い
  reply まで loop して全量を回収する。
- `ErasedPlugin::drain_output_reports` は `Plugin` 経由で
  `PluginProcessor::drain_output_reports` に既定 forward —
  `ClapPlugin` 側の override だけで全経路が生きる。
- composite/`PluginHost::drain_output_reports` は L0 in-process
  スロットへは `InProcessHost`、L1 スロットへは wire 動詞を
  経路別に委譲するだけ。

→ 変更は `ruviform-plugin-clap` の host/adapt 層のみで完結。

## clack `EventBuffer` は構造的に無界

`clack-common` 0.2.0 の `EventBuffer` は `Vec` 裏付けで、
`OutputEventBuffer::try_push` は常に `push`（再確保あり）を呼び
拒否しない。CLAP 契約上 `try_push` 失敗は「ホストが bounded で
あること」の合図だが、stock 実装はその back-pressure を持たず、
悪意ある/多産なプラグインがホスト側 RT 確保を強制できてしまう。
→ `OutputEventBuffer` を impl する `BoundedOutputBuffer` wrapper
（件数 + イベントバイトの二重上界で `TryPushError`）で構造的
バウンドを実現し、`EventBuffer::with_capacity` の予約
（index slot + `size_of::<TransportEvent>` 量子）に収める設計と
した。

## canonical `Event` のスタンプ表現

`Event::new` は const・非検査で、`offset` の block 境界チェックは
`EventList::new`（入力側）が担う。`frame_offset` は `u32` —
`u32::MAX` は canonical 表現可能。Issue #253 のレポート契約は
「ホストは蓄積した複数 block の offset を再解釈しない」ため、
block 外スタンプを届けても契約違反にならない。VST3 の入力側
collect は block 外 offset を drop するが、出力レポート面で
`bad_time` の `u32::MAX` を drop/clamp すると `void()` シンクと
観測上区別がつかない — 表現可能な変形スタンプは忠実に運ぶ
faithful-stamp 法則を選択。

## `translate_output_event` の配置（coordinator 承認）

翻訳を host.rs 内に書くと ABI→中立 写像が adapter 層に漏れるため、
`adapt.rs` への `pub(crate) fn translate_output_event` 追加を
coordinator ask で lease 拡張承認（既存 helper 再利用・構造 note
写像追加・host 側は収集/drain のみ）。

## clack API の実装詳細（実コードで確定した点）

- `Pckn` の `port_index`/`channel`/`key` はメソッドではなく
  `Match<T>` のフィールド公開。
- `TransportEvent` が標準イベント中最大サイズ — バイト上界の量子
  として採用。
- `UnknownEvent::as_bytes()` がイベントのバイト長を露出。
- `TryPushError` は unit struct（`::new()` で生成）。

## L1 で見えた「armed ≠ matched」の非同期性

`host.process` は submit が子キューへ届いた時点で返り、arm
イベントは子の `process()` で非同期に発火する。`passthrough == 0`
（matched）は「子が実レンダーした」証拠だが、その block が arm
発火「後」だったとは限らない — 初回実装は matched 2 block で
drain 2 を期待して 1 しか来ず失敗。正しい oracle は per-block の
*観測可能証拠*（`bad_time` → `tail_frames == u32::MAX` claim、
`invalid` → 全 sample NaN）で armed block を数える既存 lane の
法則に倣ったもの。また `bad_time` → `invalid` 遷移窓では旧 arm
のまま処理された block のレポートが straggler として残り得るため、
`invalid` 側の空 drain 断言は flush drain で遷移窓を分離してから
行う必要がある（test-spec に反映済み）。
