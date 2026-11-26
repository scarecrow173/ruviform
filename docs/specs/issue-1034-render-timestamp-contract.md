---
id: "SPEC-Rf1034Timestamp01"
type: "spec"
status: "approved"
title: "Issue #1034 render callback timestamp contract, baseline observation, and layer isolation"
created: "2026-11-26"
updated: "2026-11-26"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1034"
    - "https://github.com/scarecrow173/ruviform/issues/1029"
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/adr/select-audio-device-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-1034-render-timestamp-contract.md"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-test/tests/os_endpoint.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/select-audio-device-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclock-getposition"
  defers: []
  deferred-by: []
---

# Issue #1034 render callback timestamp contract, baseline observation, and layer isolation

## What Should Be Built

Issue #1034 の切り分け slice として、仮想 audio endpoint の
render callback timestamp に対する期待契約、責務境界、正式 oracle、
baseline 観測方法、layer isolation evidence の読み方を確定する。
本 spec は closeout 仕様ではなく、後続の driver / adapter / consumer
修正 slice が同じ語彙と判定境界を使うための契約である。

本 slice の実装側では、`crates/ruviform-audio-test/tests/os_endpoint.rs`
に render-only baseline case を追加し、capture loopback や xrun fault
injection に依存せず output callback timestamp 系列を bounded に記録
する。既存の regression oracle は弱めない。timestamp epoch の判定は
単一の authoritative classifier に集約し、render-only baseline と
既存 render/capture round-trip で同じ規則を使う。

## 契約

### 単一 stream epoch 内の単調性

連続した 1 つの render stream epoch 内では、consumer は
`OutputTimestamp.callback` が単調非減少であることを要求してよい。
ここで単調非減少とは、後の callback の `callback` 値が直前までに
観測した最大値を下回らないことを意味する。同一値の繰り返しは
regression ではない。

`callback` は callback が backend に通知された時点を表す backend
報告値であり、sample frame position ではない。consumer はこの値を
「同一 epoch 内の時系列順序 oracle」に使ってよいが、絶対時刻の
意味や hardware position そのものとして解釈してはならない。

### epoch 境界

open 後の最初の start から stop までを 1 つの stream epoch とする。
stop/start を跨いだ時点で OS 側は position と QPC の対応関係を
再 anchor してよい。したがって、ある epoch の最後の timestamp と
次の epoch の最初の timestamp の間に跨 epoch の単調性は要求しない。
epoch 境界自体は隠さず、boundary callback index として evidence に
記録する。

Windows shared-mode stream では、start 直後の限られた期間に OS が
position/QPC correlation を再 anchor した結果、startup transient が
観測され得る。現行 oracle は epoch 開始後 24 callback までの regression
を raw regression として記録しつつ effective regression から除外する。
これは契約違反の承認ではなく、OS startup bookkeeping と永続的な
epoch 内逆行を切り分ける bounded grace である。24 callback を超えた
regression、bounded index log に収まらず分類不能となった regression、
epoch 途中の regression は Fail である。grace window を広げない。

### timestamp の意味差

正式 oracle は `OutputTimestamp.callback` とする。各値の扱いは次の
とおりである。

- `callback`: backend が報告する callback instant。同一 stream epoch
  内の単調性 oracle に使用する。
- `playback`: backend が報告する予定再生 instant。WASAPI/cpal では
  callback instant に未消費 frame と latency 見積りを加えた値として
  構成される。診断 evidence として記録するが、本契約の単調性 oracle
  には使わない。変換不能時は `None` でよく、代替値を合成しない。
- capture timestamp: capture stream 側の値であり、render stream の
  単調性契約とは別の契約である。render/capture loopback の健全性
  evidence には使ってよいが、render-only baseline の oracle には
  使わない。
- frame position: cpal output path は frame position を公開しない。
  backend から得られない場合は `None` のままとし、callback index や
  供給 frame 数から hardware frame position を合成しない。

## 責務分担

### driver

kernel/driver 側は Microsoft `IAudioClock::GetPosition` の device
position 契約に従う責務を持つ。すなわち、device position は連続
読み取りで単調増加し、stop で凍結し、reset されない限り start で
凍結値から再開する。driver は stream position bookkeeping を正直に
報告し、逆行を隠すための時刻合成を行わない。

ただし、この device-position 契約と consumer に見える
`OutputTimestamp.callback` の契約は同値ではない。後者は WASAPI、
audiodg、cpal、adapter の変換を経た callback instant であり、
driver だけが全責任を持つ値ではない。

### WASAPI / audiodg / cpal

WASAPI と audiodg は OS 側の stream lifecycle、position/QPC
correlation、startup/restart bookkeeping を所有する。cpal 0.18.2 の
WASAPI output timestamp では、`callback` は `IAudioClock::GetPosition`
の QPC position 由来で、`playback` は callback に未消費 frame と
stream latency を加えた値として構成される。この層は backend 報告値の
provenance を壊さず、OS 側で再 anchor された値を独自に平滑化しない。

### ruviform adapter

`crates/ruviform-audio-device` の adapter は、cpal `StreamInstant` を
`MediaTime` へ変換して backend 報告値を渡す責務を持つ。`playback`
も backend 報告値を渡し、変換 overflow の場合だけ `None` とする。
adapter は backend から得られない frame position、timestamp、latency
を合成しない。`StreamInstant` の nanos が `i64` に収まらない場合の
epoch-delta fallback は表現域の変換規則であり、時刻を平滑化する
仕組みではない。

### consumer harness

`crates/ruviform-audio-test/tests/os_endpoint.rs` は consumer 側の
oracle と evidence 収集を所有する。RT callback 内では allocation
せず、bounded slots に callback index、epoch、frame 数、供給 frame
cursor、`callback`、`playback` presence/value、callback interval、
regression backstep を記録する。layer isolation のために、同一
bounded event に host monotonic callback instant/interval、
`playback - callback` lead、`playback` interval、callback 重なり
検出用の active callback depth も記録する。分類は単一の timestamp
epoch classifier に集約し、render-only baseline と render/capture
round-trip で別の regression 判定系を作らない。host monotonic と
callback 重なりは診断値であり、Pass/Fail oracle ではない。

### runner / provisioning environment

provisioned runner は仮想 endpoint の存在、driver install 状態、
workflow serialization、run id/artifact の provenance を所有する。
endpoint 不存在は Fail ではなく `NotRun(NOT_PROVISIONED)` である。
runner scheduling や startup bookkeeping の影響は evidence として
記録するが、それだけで恒久 regression を環境起因と断定しない。

## 正式 oracle

render-only baseline の正式 oracle は次のとおりである。

1. virtual render endpoint を open/start し、deterministic stimulus を
   一定時間供給できる。
2. output data callback が観測され、frame 数が 0 ではない。
3. callback ごとの frame 数と channel count が backend buffer と
   一致する。
4. 同一 stream epoch 内で `OutputTimestamp.callback` の effective
   regression が 0 である。effective regression の定義は、epoch
   開始後 24 callback の bounded startup grace を除き、それ以降の
   regression と bounded index log overflow による分類不能 regression
   を数える既存規則である。
5. stop/start の epoch 境界は boundary callback index と epoch reset
   数として記録される。跨 epoch の単調性は要求しない。
6. `playback` presence と値、log overflow、raw/effective regression、
   first/last regression index、max backstep が evidence に残る。

oracle を緩和しない。startup grace は既存の bounded 分類を明文化した
ものであり、新たに regression を無視する規則ではない。contract 上
regression を許容する判断が必要になった場合でも、その境界、数量、
provenance を別の正式変更として限定し、従来 Fail だった観測を無条件
Pass に変えない。

## baseline instrumentation

render-only baseline case は
`rf1034.audio.virtual-endpoint.render-timestamp-baseline` とする。
既存 manual lane `cargo test -p ruviform-audio-test --test os_endpoint --
--ignored --test-threads=1` から実行され、workflow や mise lane は
変更しない。capture loopback、xrun fault injection、stereo format
contract には依存しない。

instrumentation は bounded 512 slots に記録する。overflow は隠さず
カウントし、overflow 自体は timestamp 単調性 Pass を意味しない。
regression index log は従来どおり 32 slots に bounded し、log 外の
regression は分類不能として Fail 側に倒す。

各 event は次の診断入力を保持する。`supply_frame_cursor` は consumer
が backend へ供給した deterministic fixture の frame cursor、
`host_monotonic_ns`/`host_interval_ns` は harness が同一 host clock で
測った callback 到着間隔、`playback_interval_ns` と
`playback_lead_ns` は backend 報告 timestamp 内部の関係、
`max_active_callbacks`/`overlap_events` は harness 側で regression
series を重畳させていないかの健全性値である。epoch split では
callback/playback/host の interval baseline をともに reset し、跨
epoch の interval を startup transient と混同しない。

control thread では regression event ごとに bounded log 内の値から
`render_timestamp_regression_diag` を発行する。そこには epoch 内
offset、startup grace 分類、直前の epoch 内最大 callback（reference）、
backstep、regression 前後 2 callback に跨る backend callback interval、
host interval、供給 frame 数、`playback` lead/interval を含める。
この診断行は authoritative classifier ではない。既存 classifier の
観測入力を説明する evidence であり、別系統の Pass/Fail を作らない。

## layer isolation の確定範囲と未確定範囲

確定できる責務分担は次のとおりである。

1. consumer harness は backend から受け取った `callback` を補正せず、
   regression index と backstep を観測するだけである。したがって
   `callback` regression を harness が生成したとは読めない。harness
   の誤判定説は、重複 callback や供給 cursor 不整合が診断値に現れた
   場合に限り再検討する。
2. cpal 0.18.2 WASAPI output path では、`callback` は
   `IAudioClock::GetPosition` が返す QPC position から直接変換され、
   cpal の monotonic wrapper は `playback` のみを clamp し `callback`
   を clamp しない。したがって観測された `callback` regression は
   cpal の playback monotonic wrapper が隠した/作ったものではない。
   cpal conversion 説が残るのは QPC→nanos conversion または cpal の
   API 使用そのものに不具合がある場合に限定される。
3. driver position bookkeeping 説は、既存の rf977/rf952 evidence により
   弱められている。render stream の driver-reported position/counter を
   STOP 跨ぎで monotonic 化しても regression 件数が変わらなかった
   ため、少なくとも startup/restart regression については driver が
   報告しうる position bookkeeping 単独の原因ではない。ただしこの
   evidence は mid-epoch regression を完全には説明しない。
4. `playback` が前進しながら `callback` が逆行し、直後に大きな正の
   interval で追い付く sawtooth 形は、OS/WASAPI/audiodg 側の
   position↔QPC correlation bookkeeping が再 anchor された説と
   整合する。idx 3/4 の epoch-start regression はこの説で説明可能
   だが、baseline idx 250 の mid-epoch regression は startup grace
   では説明できない。

未確定のまま残す事項は次のとおりである。

- idx 250 の mid-epoch regression が runner scheduling/負荷、
  audiodg の mid-stream correlation 更新、cpal conversion、driver と
  OS の interaction のどれで最初に発生するか。host interval と backend
  two-step interval の比較、playback lead/interval、供給 cursor、
  callback overlap の provisioned evidence がそろうまでは最終帰属
  しない。
- regression-count-zero の修正検証。本 spec は oracle 緩和や修正済み
  宣言をしない。

## 非目標

本 spec と baseline slice は次を行わない。

- stereo capture channel width/format contract の再修正。
- xrun/underrun の決定的注入、fault scenario、`Invoke-XrunCases.ps1`。
- virtual endpoint harness、driver、adapter、cpal の実装修正または redesign。
- Synvelis consumer harness、Synvelis round-trip、製品コードの再評価。
- consumer oracle を緩めて Fail を消すこと。
- docs index の編集・再生成。

## 既知 evidence の解釈

rf#1029 / PR #1032 に付随する provisioned evidence では、render/capture
format は `F32/Stereo/48000Hz` であったが、output timestamp regression
が観測された。観測された regression index の一部は epoch-start grace
内だが、すべてが startup transient ではなく、現行 oracle では effective
regression が残る。したがって、既存 evidence は「全て startup transient」
という結論を支持しない。

driver の PROVENANCE では、render position clock を stop/start 跨ぎで
単調に保つ driver 側修正が試され、regression count が改善しなかった
ことが記録されている。これは driver position bookkeeping だけでは
consumer-visible regression を説明できないことを示唆するが、本 slice
だけでは原因層を確定しない。原因層の確定には、render-only baseline の
provisioned run id、artifact、case result、event log が必要である。

## 後続 slice への引き継ぎ

provisioned Windows runner が利用できない場合、本契約と local/unit
validation は完了しても provisioned verdict は `NotRun` または
`Deferred` とする。後続 slice は少なくとも次を記録する。

- `virtual-audio-driver-probe.yml` の run id、attempt、対象 SHA。
- artifact 名と `rf1034.audio.virtual-endpoint.render-timestamp-baseline`
  の case result。
- raw/effective regression、regression index、epoch boundary、
  playback presence、log overflow。
- 原因層が driver、WASAPI/audiodg/cpal、adapter、consumer harness、
  runner environment のどこにあったかの判断と根拠。

Synvelis consumer round-trip の Pass/Fail は本 Issue の upstream
evidence に転用しない。Synvelis 側の再評価が必要な場合は、対象 SHA
を固定した別 gate の evidence で確認する。
