---
id: "SPEC-034QhNRt6XTihijW6B8pOy"
type: "spec"
status: "approved"
title: "Issue #185 Generic Probe: Node/Port observation capability, Off/Monitor/Capture states, bounded RT-safe PCM capture, time/identity metadata, non-RT delivery contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/185"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/media-observation-non-destructive-pcm.md"
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/190"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #185 Generic Probe: Node/Port observation capability, Off/Monitor/Capture states, bounded RT-safe PCM capture, time/identity metadata, non-RT delivery contract

## What Should Be Built

Issue #185 が要求する「一般的な Node/Port 境界での観測能力宣言・選択的観測実行・RT→非 RT 転送契約」を、M2 typed runtime（`ruviform-audio::typed`、#195 確定済み）上のモック Graph PoC として確立する。具体的には:

- **Logical Probe Capability**: `ValidatedGraph`/`GraphSpec` の Port と `PreparedPlan` の slot binding を突き合わせ、各観測候補点について `Configured` / `Inferred` / `Unavailable` / `Unsupported` / `Unknown` を分離して報告する列挙契約。存在しない物理バッファを観測可能として返さない（分類の優先順: 物理 binding 解決 → 宣言確認。「観測実行中」を表す Synvelis 側語彙の `Observed` は capability ではなく実行状態 `ProbeMode`（Monitor/Capture）として報告し、`Activation::Disabled` port は `Unavailable` に分類する）。
- **実行 Probe 状態**: `Off` / `Monitor` / `Capture` の mode を `ProbeId` 単位で制御する非 RT API と、plan 世代と独立に動作する RT 側適用機構。Capture は期間・メモリ予算を明示制御し、budget 到達・明示停止で Monitor へ復帰できる。
- **計測・取得**: Audio port に対する per-channel Peak/RMS の軽量メトリクス（`ruviform-audio-analysis` 契約の再利用）と、選択点の PCM block 取得。TimedEvent/Control domain は観測型の拡張可能性を型で担保するが、本 Issue では `Unsupported` として分類するのみで取得契約は持たない。
- **時間・識別メタデータ**: `ProbeId`/`NodeId`/`PortId`/plan `generation`/block 先頭の絶対 `sample_position`/`frames`/`channels`/sample rate・経路遅延・loss・未観測/不連続の区別を共通メタデータ封筒（`RecordMeta`）として全レコード型（Metric/Captured/Status）に載せる。レコードの `generation` は plan の generation（slot binding の staleness key）であり、IR 層の `GraphRevision` とは別系列であることを明記する。`sample_position` は `RtRuntime` の絶対 frame cursor（可変 `frames` 対応・`seq` callback ordinal とは別物）。多点比較は同一 `generation`+`sample_position` で整列可能にし、PDC 遅延の有無をメタデータで説明可能にする。
- **RT 安全 Capture**: prepare 時確保のチャンクプール（非 RT→RT `free` チャンネル / RT→非 RT `chunks_out`・`telemetry` 2 チャンネル）による bounded・非ブロッキング搬出。`chunks_out` は `Box<CaptureChunk>` のみを運び capacity を pool サイズとする — 在飛 chunk 総数が pool を超えない構造により RT 側 push は原理的に満杯にならない（防御的 `try_push` 失敗時は working slot 再利用 + fault 計数、`Retirer` への退避は teardown 専用で leak-on-full を PCM 搬送には使わない）。`telemetry` は `Metric`/`Status` の POD record を運び、満杯時は drop-newest + loss 計数（monitor 流量が capture を枯渇させないよう分離）。drain 済み chunk は collector 側 `recycle` で `free` へ返却し、broker が定期的に pool を補充する。RT 側で heap alloc・lock 待ち・OS I/O・UI 通知を行わない。
- **動的 ON/OFF**: probe の install/rebind/set-mode/remove を非 RT で prepare/publish/retire。OFF 時はタップ処理自体を実行せず、ON/OFF が再生音声・routing に影響しないことを検証する。
- **データ配送**: 非 RT `ProbeCollector` への pull/drain（subscription は collector 保持側が自由に実装）。`ruviform-ui` 等の上位層への依存は持たず、RT と WebView IPC を直接結合しない。

## Why Now

- Synvelis Signal Flow（synvelis #18/#21/#22）が「全論理 Node の標準 Probe」「Node/Port/Graph Generation/Frame/Latency/Validity/Loss Evidence」を合意済み要件として要求しており、Ruviform 側の汎用観測面が未実装。
- 前提契約が揃った: #184（`NodeId`/`PortId`/`PortCapability::ProbePoint`/`StableHandleMap`/`GraphRevision`）と #195（`PreparedPlan`/`RtRuntime`/generation/slot ownership/retire）が merged。
- 既存の観測資産（`audio-analysis` の `LevelMeter`/`SpectrumAnalyzer`/`WaveformPeaks`、`observability` の `Counter` pair、`ruviform-rt` の `channel`/`snapshot`/`retire_channel`、#104 `observability_poc` のコスト実測）の棚卸しが完了し、PoC 判断材料が揃った。

## Users and Value

- **Synvelis（第一 consumer）**: Signal Flow のノード単位標準 Probe ON/OFF、port 単位 Monitor、選択的 PCM Capture/比較を、公開 API として組める。診断・原因推定は Synvelis 側責務のまま。
- **将来の UI/診断実装**: pull ベースの `ProbeCollector` から metrics/captured PCM/loss/状態遷移を取得し、任意の表示・解析層へ搬送できる。`ruviform-ui` 等との依存方向は engine→UI の一方を維持（probe 側は上位層を知らない）。
- **`audio-lens`（第二 consumer 分類対象）**: typed engine 面は非消費。分類は `no impact`（根拠: 本 spec の全変更は additive API 追加＋既存挙動不変の内部拡張のみで、既存シンボルの削除・改名・意味変更を一切含まない。#219 contract の分類手順に基づき、実装完了時の diff で検証する）。

## Scope

### In Scope

- `ruviform-audio::typed` への `probe` モジュール追加: capability 列挙、`ProbeBroker`（非 RT 制御面）、`ProbeDeck`/`RtProbe`（RT 適用面）、`ProbeCollector`（非 RT 配送面・drain した chunk の `recycle` 返却経路を含む）。
- `PreparedPlan`/`ScheduledNode`/`InputBinding` への additive 拡張: per-node accumulated latency メタデータ（経路遅延情報の正本化）、`InputBinding::effective_slot`（`scratch ?? 単一 edge slot` の compile 時確定 — NodeIo/materialize/probe が読む唯一の binding 規則）、`ScheduledNode::outputs` の `(PortId, slot)` 化（output port→slot 解決規則の重複排除）。既存フィールドの意味は不変（`outputs` 型のみ変更）。
- `RtRuntime`/`RtState`/`GraphRuntime` への additive 拡張: 絶対 frame cursor（`sample_position` の正本・可変 `frames` 対応）、probe command drain（generation-scoped command は activation 時・lifetime command は毎 callback）、schedule loop の tap 点、非 RT の `published_plan` アクセッサ（broker の binding 再解決用）。
- `ruviform-audio-analysis` への additive 拡張: `ChannelLevel` 算出を channel 単位で再利用可能な関数の抽出（`LevelMeter` の挙動不変）。
- `ruviform-rt` 既存 primitive（`channel`/`snapshot`/`retire_channel`）と `ruviform-observability::Counter` pair の再利用（新規同期機構は作らない）。
- モック Graph（既存 `tests/common` の `chain`/`merge` 系 fixture）上での PoC: 2 観測点以上の ON/OFF、pre/post Peak/RMS、上限制御 PCM Capture の非 RT 搬出、metadata（sample position/generation/loss）取得。
- RT 非ブロック性の検証: `rt_` 契約テスト（`assert_no_alloc` 窓内の alloc 0）＋ fault 注入（溢れ・途中停止・generation 更新・遅延/購読解除）＋ ON/OFF 負荷差の計測記録。

### Out of Scope

- Synvelis 固有の診断ルール・Probe 配置 UI・DAW Track/Bus 型・原因候補推定・異常判定・Inspector。
- 全ノード常時無制限 Capture、Plugin 内部の非公開 DSP 段階の観測保証、capture 結果の永続ストレージ形式・外部ファイル保存。
- Recording Writer（Synvelis #12 / Ruviform #187）の無欠損・fail-stop 契約 — Probe の drop/overwrite 許容方針は観測用途専用であり転用しない（共通 primitive の再利用可否は別 Issue）。
- TimedEvent/Control domain の payload 観測（M3/#188/#190 へ defer）。`SignalDomain` 単位の型拡張余地のみ担保。
- Plugin adapter 接続（M4/#196）、旧 pull 型 `Graph`/`GraphPlan`/`Session` への probe 追加（typed path のみ対象・旧 API は不変）。
- 永続的・cross-session の probe 安定識別子（`ProbeId` は broker 生存期間内の安定 ID）。

## Requirements

- Rust edition 2024・workspace MSRV。`unsafe` 不使用。`publish = false` 継承。全公開 enum に `#[non_exhaustive]`。
- **RT 契約**: `RtRuntime::process` 内の probe tap 経路（metric 計算・chunk copy・record push）は alloc 0・非ブロッキング・OS I/O なし・destructor なし。`assert_no_alloc` 計測窓で 0 を検証。
- **信号不変性**: probe は slot の非破壊 read のみ — `Off`/`Monitor`/`Capture` のいずれでも DSP 出力・routing・PDC 挙動に影響しない。Probe ON/OFF と Processor の Bypass は完全に別概念。
- **能力の正直性**: 物理バッファ（arena slot）に解決できない観測点を観測可能として返さない（分類順: 物理解決 → 宣言）。未接続 input・generation 外の port・未対応 domain・`Activation::Disabled` を `Unavailable`/`Unsupported`/`Unknown` に分類する。sample rate メタデータは port 宣言の `PortFormat::Audio` rate（driver 実動作 rate ではない — 宣言情報として正直に報告）。
- **観測の正直性**: tap は `slot.stamp() == seq` の鮮度監査を `NodeIo` と同じ規則で適用し、不一致ではデータ record を発行せず `Unobserved` を記録する（stale/uninitialized slot の観測を valid として返さない）。`RtProbe` の binding は `bound_generation` を持ち、activation 後に `plan.generation` と不一致なら `Unbound` として記録・観測をスキップする（slot index 再利用による誤 slot 観測を防ぐ）。
- **bounded 制御**: probe 数・chunk プール・配送チャンネル・capture 期間・metric チャンネル全てに明示上限。budget は確保量と搬出容量の双方で定義される。
- **決定性**: 同一 spec+plan+binding の capability 報告は決定的。`ProbeId` 採番・slot 解決・loss 計数の規則は文書化される。
- **失敗の定義**: probe 制御 API の失敗（channel 満杯・予算超過・未知 point）は定義済み `Result`/`enum` で返し、RT 側は fault 記録＋deterministic degrade（観測を落としても音声経路は止めない）。
- **Consumer 影響分類**: synvelis = `impacted`（新規 API 提供）、audio-lens = `no impact`（消費面非変更 — #219 contract に基づき実検証で確認）。

## Acceptance Criteria

- [ ] **AC-1 (要求1)**: mock Graph 上で、全 audio port（node input/output・graph boundary）について capability 報告が得られ、`Configured`（ProbePoint 宣言済み — fixture に `ProbePoint` 宣言 port を含める）/ `Inferred`（未宣言だが物理的に観測可能）/ `Unavailable`（未接続 input・`Activation::Disabled` 等・バッファ不在）/ `Unsupported`（非 Audio domain）/ `Unknown`（存在しない port）が分離される。物理解決不能な点は宣言の有無に関わらず `Observable` 系に分類されない。同一 spec+plan で報告は決定的（同一内容の再列挙が byte 等価）。
- [ ] **AC-2 (要求2・7)**: `ProbeBroker` で 2 観測点以上を `Off`/`Monitor`/`Capture` 間で遷移でき、`mode == Off` の probe は tap 点で slot `borrow`/record 発行を一切行わず（定義済み境界: deck の active-tap 判定 + per-point mode 比較のみ）、probe ON/OFF 有無で同一入力の boundary 出力が bit 一致する。
- [ ] **AC-3 (要求3)**: 同一処理区間の node input(pre) と output(post) の Peak/RMS が取得でき、差異を表示用データとして drain で返せる。メトリクス値は `ChannelLevel` 契約と一致し、metric record も共通メタデータ封筒を持つ。
- [ ] **AC-4 (要求4・5)**: 上限制御された PCM Capture が非 RT へ搬出され、chunk ごとに共通メタデータ封筒（`probe_id`/`node`/`port`/`generation`/`sample_position`/`frames`/`channels`/declared sample rate/tap 点別の `effective_latency`/loss・未観測・不連続の区別）が取得できる。chunk capacity は plan `max_frames` 以上で、`frames` が小さい block は `frames_valid` で有効 prefix を記録。budget 到達で `BudgetExhausted` 停止イベントが記録され Monitor へ復帰する。drain 済み chunk は `recycle` で free pool に戻る。
- [ ] **AC-5 (要求5・7)**: バッファ溢れ（free 枯渇・telemetry 満杯）・途中停止（明示 `Stop`）・plan generation 更新・collector 側の遅延 drain/購読解除の各注入で、RT `process` がブロックせず fault が非 RT 観測できる。generation 更新後に rebind 済み probe は**新 plan の正しい slot を観測**し（stale ではなく functional rebind）、未 rebind の窓は `Unbound` status で記録され誤 slot を観測しない。長時間 stall から回復した後も pool が健全で capture が再開する（leak なし — `chunks_out` の構造的不満杯保証）。RT 区間の alloc は `rt_` テストで 0。
- [ ] **AC-5b (要求2・5)**: 非 RT 制御 API の失敗面が定義済み `Result`/`enum` で返る: 未知 `ProbeId` への `SetMode`/`Remove`、`Unavailable`/`Unknown` 点への install、予算超過 `CaptureConfig`、command channel 満杯。パニック・無言 no-op でないことを検証する。
- [ ] **AC-6 (要求6)**: 同一 `generation`+`sample_position` で 2 観測点の chunk を整列でき、tap 点別の `effective_latency`（node input direct edge → `accum[producer]`、scratch merge → `accum[producer]+delay_frames`、node output → `accum[node]`、graph boundary output → `total_latency`）が Rebind 時に確定されメタデータで説明可能（同時刻比較の誤判定材料を型で残す）。
- [ ] **AC-7 (要求9)**: `ProbeCollector::drain` が pull ベースで全イベント（metric/capture/status）を返し、probe 側から `ruviform-ui`・WebView IPC への依存が型レベルで存在しない（`cargo tree` で依存 DAG に `ruviform-ui` が現れない）。
- [ ] **AC-8 (要求8)**: 標準 port 集合以外の任意観測点の拡張可能性が評価され、plugin 内部・任意 slot への直接 tap が「公開 API では保証しない」ことが設計書に明記される。
- [ ] **AC-9**: 実時間性能・alloc 0・ON/OFF 負荷差の計測結果が impl record に記録され、未検証範囲（実デバイス callback・多 ch>8・cross-session ID 等）が明示される。
- [ ] **AC-10**: 実行した検証コマンド（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci`）と結果・未検証項目が impl record に区別して記録される。最終検証前に `git fetch origin && git merge origin/main` を実施し生成物（`Cargo.lock` 等）を再生成する。

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| TimedEvent/Control の観測 payload | `SignalDomain` 単位の `Unsupported` 分類と、イベント系列用 `ObservedKind` の拡張余地のみ | M3（#188/#190）のイベント payload 契約 | event port を audio PCM として誤観測 |
| Recording Writer との共通 primitive | Probe の drop/overwrite 許容方針は観測専用。lossless queue 契約は分離 | Ruviform #187 / Synvelis #12 | 録音品質契約への緩い保証の混入 |
| Probe の永続識別子（cross-session） | `ProbeId` は `ProbeBroker` 生存期間内の単調 ID | consumer 側の Semantic ID 対応付け設計（Synvelis #18 側） | UI 再構成時に ID 対応が崩れる可能性 |
| 大容量 PCM の複数 drain 経路（fan-out subscription） | PoC は単一 `ProbeCollector` の pull のみ | UI/診断層の配信設計 Issue | fan-out 要求時に API 追加が必要 |
| Plugin 内部 stage の観測 | 公開契約として保証しない（`Unavailable` 分類で表現） | plugin-host 側の内部 tap 契約 Issue | 内部観測要求を満たせないことを曖昧に残す |

## Sources and Relations

- 一次要件: GitHub Issue [#185 全ノード標準Probeを支える汎用観測点・RT安全Capture基盤を設計する](https://github.com/scarecrow173/ruviform/issues/185)（`relations.source`/`implements`）、M7 実装境界は [#193](https://github.com/scarecrow173/ruviform/issues/193)。
- 上流文書（`relations.derives-from`）: [discovery](../discovery/issue-185-generic-probe-observation-inventory.md)（棚卸し・選択肢比較の正本）。
- 前提（`relations.depends-on`）: [#184 spec](issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md)（Node/Port 実行時境界・`PortCapability::ProbePoint`）、[#195 spec](issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md)（plan generation・slot ownership・retire 経路）。
- 関連（`relations.related`）: [RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)、[観測分類 ADR](../adr/media-observation-non-destructive-pcm.md)、[synvelis-driven 設計](../designs/synvelis-driven-target-architecture-and-migration.md)、[foundation observability sketch](../designs/foundation-observability-minimal-api.md)、[`AGENTS.md`](../../AGENTS.md)。
- 参考（`relations.references`）: [#104 RT 観測 PoC](https://github.com/scarecrow173/ruviform/issues/104)、[#187 duplex/recording](https://github.com/scarecrow173/ruviform/issues/187)、[#196 M4 plugin 接続](https://github.com/scarecrow173/ruviform/issues/196)。
- defer（`relations.defers`）: #187・#188・#196。
