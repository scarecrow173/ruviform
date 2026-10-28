---
id: "TSPEC-034NXpU1IxxAo36yJ3zGbW"
type: "test-spec"
status: "approved"
title: "L1 video Failed parity tests（Issue #118 — 子スコープ terminal・帰属可能 per-instance・inert・timeout の video 写像）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-export/src/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/vshm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-export/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs"
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
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 video Failed parity tests（Issue #118 — 子スコープ terminal・帰属可能 per-instance・inert・timeout の video 写像）

## Purpose

video 経路の `Failed` 粒度・通知・inert・timeout 契約の oracle — #117 が確定した子スコープ terminal + 帰属可能 per-instance 収束 + inert 規則 + `InstanceFailed` fan-out の **video 写像**を pin する（「verbatim 適用」の検証版）。wire codec 形式は `issue-118-wire-v3-codec-tests`、公開面の正常系は `issue-118-video-e2e-tests` が所有する。本 spec は「video 子・video instance が死ぬ・違反する・wedge する全分岐が #117 と同じ粒度で収束する」ことを対象とする。video 経路が将来の wire version で再設計されても、#117 契約が生きる限り本 spec の guarantee は残る。

## Feature

video L1 経路は、子死亡・pipe 破壊・protocol violation・timeout・inert frame・mid-verb 死亡の各観測を、`Inner.instances`/`InstPhase`/waiter/notify/`converge_child`/`converge_instance` の kind 非依存機構と、runner 側の normalize/panic 正規化の video 対応で、#117 と同一の粒度に収束させる。video 固有の帰属可能違反は `Rendered.output` の post-decode semantic 検証失敗（非 canonical・config dims/format 不一致・region-fit 超過）。

## Rules

- **子スコープ terminal**（帰属不能）: 子死亡・kill・stdout EOF・stdin write 失敗・decode 不能・未知 kind・scope/`instance` 範囲外・live instance への waiter-less `Reply`・`Reply.instance` 不一致 → 全 live instance `Failed` + 全 pending waiter `Err(Ipc)` + 各 minted instance に `InstanceFailed{handle, exit}`（同一 `ChildExit`）— #117 と同一
- **帰属可能 per-instance**: well-formed で live instance 一意帰属の違反 — `Rendered.output` の post-decode semantic 検証失敗（非 canonical desc・config dims/format 不一致・`offset+len > out_bytes`）→ 対象 instance のみ `Failed`・caller の `Render` は waiter drain 経由で `Err(Ipc)`・Dedicated で sibling は存在しないが機構は同一。video reply の `instance`/seq 相関違反（seq 一致+instance 不一致）と live instance 宛の waiter-less `Reply` は demux 判定表で **child-scope** に分類される（audio `l1/mod.rs` の demux row と同一規則 — `Reply.instance` が waiter の instance と一致しない場合・waiter を持たない live instance 宛 reply は `Demux::Child` → 子スコープ terminal）のため本行には含めない — 上の child-scope 行がカバーする
- **inert**: registered-but-not-live（`Failed` 未 `Destroy`・`Destroying`・`Creating`）宛 frame・post-removal 宛 `Reply`/`Rendered`・post-`Shutdown` frame・**video runner 宛 inbound `BlockIn`**（direction-legal だが video には帰属不能な foreign doorbell）→ 静かに drop — 違反でも収束でもない・返信なし
- **wedge**: `render_timeout`/`command_timeout` 超過 → 帰属不能 → 子スコープ `Failed`（timeout は instance を名指しできない #117 規則）— `VideoRender` reply wait は `render_timeout`（既定 60 s）・それ以外の handshake/lifecycle/param 動詞は `command_timeout`（既定 10 s）
- **mid-verb 死亡**: mid-`VideoRender` 死亡 → render waiter `Err(Ipc)` wake・out 領域は読まれない・影響 instance `Failed` + `InstanceFailed`。mid-`CreateVideoInstance` 死亡 → 通知なし・`Err(Ipc)`・handle 未 mint。mid-`DestroyInstance` 死亡 → `Failed` + 通知 + `Destroy` は `Ok`（slot 解放）
- **video `Err` reply で mirror 不変**（L0-faithful・audio doorbell との意図的 divergence）: `Err(Lifecycle)` pre-commit・`Err(PluginFailed)`・`Err(Ipc)`/`Err(Unsupported)`/`Err(InvalidValue)` defined 拒否 — いずれも instance mirror を一切変更しない。video が per-instance `Failed` 収束するのは instance 帰属の protocol violation のみ
- **runner 側 normalize**: video hook panic → `Err(PluginFailed)` で子は生存（`catch_unwind` — audio と同じ）。`Shutdown`/stdin EOF → 全 video entry normalize → exit 0。inbound `BlockDone`/`Reply`/`Hello` は direction-illegal → `normalize_all` + `ExitCode::FAILURE`（audio `serve` と同規則 — `BlockIn` のみ静かに drop で非対称）
- **`Failed`/`Destroying` 宛の video 動詞**: host mirror で `Err(Failed)`（`Destroy` 以外 — audio と同一語彙）。runner 側も同型
- **`vshm` attach/host create の fail-closed**: header 検証（`in_off==28`・`out_off==28+in_bytes`・`reserved==0`・`len >= out_off+out_bytes`・`wire_version==WIRE_VERSION`）・`in_bytes`/`out_bytes` の `frame_bytes(config)` 照合・`frame_bytes ≤ WIRE_VIDEO_MAX_FRAME_BYTES` 子側 bound 再検査 — 不一致は `Err(Ipc)` reply で entry 無しのまま閉じる（子は死なない）

## Examples

- video 子 kill → `Err(Ipc)` waiter wake + `lifecycle_state == Failed` + `InstanceFailed` 通知 1 回（once — `Notify` 一撃）・`Destroy` は `Ok`
- mid-render 死亡（`--crash-after-msgs` を `VideoRender` 送信直前に合わせる）→ `Render` caller `Err(Ipc)`・out 領域非読・`Failed` + 通知
- `--noncanonical-output`（worker が `Rendered{output}` に非 canonical stride/offset または config 不一致 dims/format を載せる）→ well-formed・一意帰属 → 対象 instance のみ `Failed`・caller `Err(Ipc)`・`InstanceFailed` 通知
- `--fail-render`（worker が `Err(PluginFailed)` を返す）→ caller `Err(PluginFailed)`・**mirror 不変（Processing のまま）**・通知なし・子生存 — audio doorbell `Err` 収束との divergence を pin
- `--fail-initialize`（`VideoInitialize` `Err(PluginFailed)`）→ mirror は `Created`（InitFailed loop）・`Destroy` 合法
- `--fail-deactivate`（`VideoDeactivate` `Err(PluginFailed)`）→ **mirror は Initialized commit 済 + error reply**（commit-then-advisory — audio `StopProcessing` 同型）
- `--hang-after-msgs=N` で render wedge → `render_timeout` 超過 → `Err(Ipc)` + 子スコープ `Failed`。handshake/lifecycle wedge → `command_timeout` で同型
- video runner への synthetic `BlockIn{seq,instance}` → 静かに drop（子は生き・返信なし — MiniHost が後続 `Scan`/動詞で生存確認）。synthetic `BlockDone`/`Reply`/`Hello` → protocol violation で子終了
- worker kill 後の post-removal `Rendered`/`Reply`・`Shutdown` 後 frame → host 側 inert drop（non-event — 合成 frame で demux 判定を unit pin）
- mid-`CreateVideoInstance` 死亡 → `Err(Ipc)`・通知なし・handle 未 mint（`Creating` phase の converge は silent — `minted()==false`）
- `vshm` attach 拒否行列: magic 不一致・`wire_version` 不一致・`in_off != 28`・`out_off != 28+in_bytes`・`reserved != 0`・`len < out_off+out_bytes`・`in_bytes != frame_bytes(config)`・`frame_bytes > WIRE_VIDEO_MAX_FRAME_BYTES` → 全て `Err(Ipc)` reply + 子生存

## Guarantees

- video 経路の `Failed` 収束は #117 確定形と 1:1 — 子スコープ/帰属可能/inert/non-event の判定が video で変わらない
- `Rendered.output` semantic 違反は host 側で帰属可能 per-instance `Failed` に写像される — child-side `VideoRender.input` の同違反は `Err` reply（`Ipc` または `Prepare(UnsupportedConfig)`）で子は生きるという非対称が pin される
- `InstanceFailed` は minted instance 毎に exactly-once — `try_recv_notification` の video arm が video `notify_rx` を drain する（skip しない — arm 欠落は通知喪失になるため必須）
- `Err` reply が instance mirror を汚染しない — video では plugin error と protocol violation が reply variant で区別できるため「失敗した lifecycle 動詞は遷移しない」L0 意味論に忠実
- timeout は `render_timeout`/`command_timeout` の役割分離どおり — `VideoRender` のみ render timeout・他は command timeout
- runner は direction-legal foreign `BlockIn` を静かに drop し direction-illegal frame を protocol violation で畳む — video 子が doorbell を consume しない契約の対称面

## Non-goals

- codec 層の bounds・tag・presence 検証 — `issue-118-wire-v3-codec-tests` が対象（本 spec は decode 後の収束意味論を対象とする）
- 正常系 lifecycle/render/param — `issue-118-video-e2e-tests` が対象
- audio 経路の `Failed` 粒度そのもの — `issue-117-bundle-failed-granularity-tests` が正本（本 spec は video 写像の差分・転用箇所のみ）
- demux 判定表の全行再検証 — kind 非依存機構は #117 suite が pin 済み。本 spec は video 固有行（`Rendered` semantic 違反・inbound `BlockIn` drop・`Err` reply mirror 不変）と「video でも同じ収束が観測される」代表行を pin
- supervisor の video 対応 — v1 範囲外（`RestartRecipe` は audio 形 — spec/design deferred）

## Risk

- `try_recv_notification` の video arm 欠落 → video `InstanceFailed` が永遠に失われ `#117 AC4(a)` parity が video で成立しない
- `Rendered.output` の semantic 検証を codec 層に置く → config 由来の region 容量を codec が知らず検証不能 or host 側検証漏れで違反 desc を受理 → 出力 frame が壊れた desc で caller に返る
- 帰属分岐の誤り → `Rendered` 違反を子スコープ収束に畳み sibling/child 全体を殺す（Dedicated では観測しにくいが機構の契約違反）
- `Err` reply → `mark_failed` の audio 写像を機械踏襲 → plugin の通常 error が instance `Failed` 化し L0 意味論と非整合（再試行不能化）
- `BlockIn` drop 未実装 → video runner が audio doorbell を protocol violation で畳み子死（audio 契約 frame の誤送を子の違反に仕立てる）
- timeout の混線（render も `command_timeout` 固定のまま）→ 重い filter が 10 s で `Failed` 化する実害
