---
id: "KNOW-0P0KowVAC1GxkKgglQP9xz"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "時間表現・音楽時間・トランスポート"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/187", "https://github.com/scarecrow173/ruviform/issues/189", "https://github.com/scarecrow173/ruviform/issues/190", "https://github.com/scarecrow173/ruviform/issues/196"]
  informs: []
  explains: []
  supersedes: []
---

# 時間表現・音楽時間・トランスポート

この文書は、時間表現・音楽時間・トランスポートに関する設計知見を、後に削除された過去の実装記録から抽出してまとめたものである。記録元の文書は存在しないため、本書はそれらに依存せず単独で読めるように記述する。事実は各記述の末尾に Issue 番号で出所を示す。実測値や観測された挙動は事実として記し、解釈が含まれる箇所はその旨を明示する。素材となった記録は主に Issue #187・#189・#190 に基づく。

## トランスポート情報と musical snapshot の構築

call_transport_info は 1 つの physical call について TransportInfo を構築する。TransportInfo は status、call offset の transport position、loop range を含む（Issue #189・#190）。

tempo map と ppqn が指定された場合、call_transport_info は TempoMap::media_to_beat と time_info による musical snapshot を生成する。この経路は Issue #189 の rational beat 経路を使い、固定 PPQN を持たない設計である（Issue #189・#190）。「ppqn が指定される」という条件と「固定 PPQN を持たない」という記述の関係は素材からは一意に読み取れず、未確定事項として末尾に記す。

## plugin bridge の責務分離と座標変換の正本

transport_bridge.rs は、media-core の canonical 契約を plugin wire 契約（TransportInfo・EventList・ProcessConfig）へ変換する薄い層である。座標変換の authority は transport_schedule.rs に置き、bridge 側で座標変換を再定義しない設計を採った。責務の重複を避け、変換ロジックの単一の正本を保つ判断である（Issue #190）。

## ProcessConfig から BlockContract への写像と capability の扱い

contract_from_process_config は、activated ProcessConfig の min_frames/max_frames を BlockContract へ写像する（Issue #190）。

in_block capability は adapter-attested 入力のみを信頼し、Unknown/Declared は supported 扱いしない。capability を申告値ではなく adapter が attest した値で判断する設計であり、未検証の申告を supported とみなさない点が「正直な扱い」の内容である（Issue #190）。

L0 は gate 内で frames < min_frames を拒否し、L1 は子 contract へ委譲する。したがって呼出側は実効 activated minimum を渡す必要がある（Issue #190）。L0・L1 が指す層またはレベルの定義は素材に含まれない。

## calls_for_span と partition 判定の一本化

calls_for_span は、span の合法 physical partition を BlockContract::partition に委譲して求める。undeliverable な span は None を返し、plugin へ届かないようにする。partition の正当性判定を scheduler 側の契約に一本化する設計である（Issue #190）。

## イベント窓の判定と EngineFrame::checked_offset

build_call_event_list は、span event 窓のうち [call_start, call_start+frames) に入る event を process-start 相対 frame_offset へ変換し、scratch 配列へ書き込んで EventList で検証する。窓は半開区間である（Issue #190）。

engine frame の半開窓判定には EngineFrame::checked_offset を使う。epoch-safe であり、PartialOrd を使わない実装とした（Issue #190）。

## min_frames 違反 call の非発行保証

process 呼出の frame 数は、plugin が BufferView::frames() から読む。mock driver（MockDriver/RecordingProcessor）は call 長に合わせた slice を渡す。これは観測された挙動である（Issue #190）。

「min_frames 違反 call を呼ばない」という保証は、callback 側の検査ではなく、呼出前に partition を要求する bridge 構造で実現する設計判断である。違反 call を検出して拒否するのではなく、違反しうる call を構造的に発行しない方針である（Issue #190）。

## zero-frame flush の wire 契約

zero-frame flush の検証は callback 経由ではなく wire 契約として直接行う。EventList::new(&[ev@0], 0) は受理され、offset>0 の event は拒否されることを確認した。これは観測された挙動である（Issue #190）。

## 未実装の Verified 経路と follow-up

in-block transport event を扱う InBlockTransportSupport::Verified 経路は、PoC では未実装である。リスクとして記録された上で adapter 側の責務として Issue #196 に委譲された（Issue #187）。

follow-up として Issue #196（adapter capability attestation + multi-bus/stateful process）と Issue #187（実 device clock backend への device_restart 結線）が挙げられている（Issue #187）。

## 未確定・保留事項

InBlockTransportSupport::Verified 経路の実装状況は、Issue #196 側の記録を参照しないと確定できない。素材の記録では PoC 未実装・Issue #196 への委譲とされている（Issue #187）。

素材の Issue #187 項には出所タグとして (Issue #190) と (Issue #187) が混在しており、どちらの Issue の記録に基づく知見か一意に決まらない。本文では項の見出しに従い (Issue #187) で示した。

「tempo map + ppqn 指定時」という条件と「固定 PPQN を持たない設計」の関係は、素材の記述だけでは一意に解釈できない。ppqn が外部からの入力パラメータを指すのか、設計が固定値を持たないという意味なのかは不明である（Issue #189・#190）。

L0・L1 が指す層またはレベルの定義、および TempoMap::media_to_beat と time_info が構成する musical snapshot の具体的な契約は、素材に含まれない（Issue #189・#190）。

素材の抽出元記録は 1 ファイルのみが対象であり、関連する計画・タスク文書は参照されていない。上記の未確定事項の一部は、それらの文書を参照すれば解消する可能性がある。
