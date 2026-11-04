# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/tasks`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| TASK-034R4Jx09ei8vexAKp8oe7 | T5 consumer-verify anonymous-HTTPS access mode（public repo 対応）+ 棚卸し refresh | in-progress | [issue-219-t5-anonymous-access.md](./issue-219-t5-anonymous-access.md) |
| TASK-yFczRmBPJEUZkKyX3bngmg | Issue #329 T1 — ci.yml verify/ui-verify へ sccache 導入 + Cold/Warm 実測 + IR | in-progress | [issue-329-t1-sccache-verify-ui-lanes.md](./issue-329-t1-sccache-verify-ui-lanes.md) |
| TASK-034RJ02dFTbxsMkC5rKofw | Issue #331 T1 — verify lane の check-ci を lint-ci に統合 + 等価性・実測記録 | in-progress | [issue-331-t1-check-step-dedup.md](./issue-331-t1-check-step-dedup.md) |
| TASK-034QeftbDcuNOqRCo3KycI | T-1 ruviform-graph crate skeleton + ids/port/graph types + builder | completed | [t-1-ruviform-graph-crate-skeleton-ids-port-graph-types-builder.md](./t-1-ruviform-graph-crate-skeleton-ids-port-graph-types-builder.md) |
| TASK-034Qefu64sxmR4DaHbCFGf | T-2 Validator + StableHandleMap + GraphProject | completed | [t-2-validator-stablehandlemap-graphproject.md](./t-2-validator-stablehandlemap-graphproject.md) |
| TASK-034QefuVxwLiU0xGxi496X | T-3 PoC ex_001-006 + index/lockfile + full verification evidence | completed | [t-3-poc-ex-001-006-index-lockfile-full-verification-evidence.md](./t-3-poc-ex-001-006-index-lockfile-full-verification-evidence.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| TASK-034Jf1Cl3xf8K9wcSuvFYK | ベンチハーネス骨格と dev-dependencies | done | [add-bench-harness-and-dev-deps.md](./add-bench-harness-and-dev-deps.md) |
| TASK-034OpsgIJWNistmrGfoPXD | T3. InstanceFailed に cause field + variant 非排他（全呼出サイト更新） | done | [add-cause-to-instance-failed.md](./add-cause-to-instance-failed.md) |
| TASK-034JOulyBprt7B3tRAzkKZ | chunk timestamp 単体テストと Issue #59 完了確認 | done | [add-chunk-timestamp-tests.md](./add-chunk-timestamp-tests.md) |
| TASK-034JPwYfiOnSaZJgGJQQyY | dispatch_probe ベンチ追加 | done | [add-dispatch-probe-bench.md](./add-dispatch-probe-bench.md) |
| TASK-034OpsfP4ZxOuDqHZhEILm | T2. FailureCause + HostError::SpawnFailed 型追加（boundary.rs・codec 互換テスト） | done | [add-failure-cause-and-spawn-failed.md](./add-failure-cause-and-spawn-failed.md) |
| TASK-034Opsj1KiLPMYmnY6yGmq | T5. l1::diag 診断レコード・DiagRegistry（ring・snapshot・retention・watermark） | done | [add-l1-diag-record-and-registry.md](./add-l1-diag-record-and-registry.md) |
| TASK-034OpsFegc9YAYfop3MYbp | T10. L1 diagnostics E2E（counter・stderr・cause・bundled・supervisor・pre-mint） | done | [add-l1-diagnostics-e2e.md](./add-l1-diagnostics-e2e.md) |
| TASK-034OpsEs0zbbjn0dDTnIIj | T9. 単体テスト拡充（counter 9 ケース・ring 不変・分類写像・watermark） | done | [add-l1-diagnostics-unit-tests.md](./add-l1-diagnostics-unit-tests.md) |
| TASK-034IdQtDnY6ZKHFhFhXKmW | 手動 CI ワークフローの追加 | done | [add-manual-ci-workflow.md](./add-manual-ci-workflow.md) |
| TASK-034OpsDG34f3LJc5NmwGNX | T8. pre-mint 診断（phase・retention 登録・SpawnFailed 境界・query 面） | done | [add-pre-mint-diagnostics.md](./add-pre-mint-diagnostics.md) |
| TASK-034JOukDzGSLe1s0mtlzeg | DataCallbackState への sample_rate 配線 | done | [add-sample-rate-to-callback-state.md](./add-sample-rate-to-callback-state.md) |
| TASK-034Jf41t2KGVmA9hbvyDVz | 等価性 matrix テスト | done | [add-simd-equivalence-matrix-tests.md](./add-simd-equivalence-matrix-tests.md) |
| TASK-034JNwUoAfb2TnY7nw07XH | 時間変換精度テストの追加（音声×映像行列・往復/連鎖誤差上界） | done | [add-time-precision-tests.md](./add-time-precision-tests.md) |
| TASK-034KSVq7PFSELAOY00SIZG | AffineTransform カーネル + 文書クローズ | done | [affine-kernel-and-closeout.md](./affine-kernel-and-closeout.md) |
| TASK-034IRZAn8ce6BYjOG6KhrZ | docs/designs/overview.md の status 注記整合 | done | [align-designs-overview-status-annotations.md](./align-designs-overview-status-annotations.md) |
| TASK-034JP1tb1wQI9ac2cEwBq4 | プラグイン境界 RT 契約テストへの assert_no_alloc 適用 | done | [apply-assert-no-alloc-to-plugin-boundary.md](./apply-assert-no-alloc-to-plugin-boundary.md) |
| TASK-034JO3X9ZpAvMoWiRIiMU4 | MSRV 1.89 適用とドキュメント同期（Cargo.toml・AGENTS.md） | done | [apply-msrv-1-89-and-sync-docs.md](./apply-msrv-1-89-and-sync-docs.md) |
| TASK-034J6PiHv9GsKby7yH2eMO | 全公開 enum への `#[non_exhaustive]` 適用と exhaustive_enums allow の撤去 | done | [apply-non-exhaustive-to-public-enums.md](./apply-non-exhaustive-to-public-enums.md) |
| TASK-034IRYXA0oq14nlgmC4Bch | workspace への publish = false 方針の適用 | done | [apply-publish-false-policy-to-workspace.md](./apply-publish-false-policy-to-workspace.md) |
| TASK-034N9j4dL22V16U4g7py5Z | media-core/dsp/audio-analysis lib test binary への AllocDisabler 宣言 | done | [arm-assert-no-alloc-lane.md](./arm-assert-no-alloc-lane.md) |
| TASK-034Kq90bnj2bifFFCKfMVA | Issue #92 文書クローズと全量検証 | done | [audio-analysis-closeout.md](./audio-analysis-closeout.md) |
| TASK-034Kq8ZHrS7hOfvYmTEUQI | audio-analysis クレート骨格 + LevelMeter/SpectrumAnalyzer/WaveformPeaks | done | [audio-analysis-crate.md](./audio-analysis-crate.md) |
| TASK-034JRQCnF0sLKz5obmCHNT | Issue #25 文書クローズと全量検証 | done | [audio-engine-closeout.md](./audio-engine-closeout.md) |
| TASK-034JRQBhUGz8OWIgGAnEgl | ruviform-audio クレート新設 + pull グラフ/ミキサー/プランスナップショット | done | [audio-engine-graph-snapshot.md](./audio-engine-graph-snapshot.md) |
| TASK-034JRQBPxKgu0yLVIB6Xm7 | セッション統合（OutputStream callback → RT グラフ駆動） | done | [audio-engine-session.md](./audio-engine-session.md) |
| TASK-034MrsUVP3NUi1Nrap9gyd | audio シーム（AudioNode FnMut impl + plan.rs 退役 — Retirer/ptr_eq/Drop for RtPlan/drain） | done | [audio-fnmut-node-and-plan-retirement.md](./audio-fnmut-node-and-plan-retirement.md) |
| TASK-034IwBWH50nD9Unn8HghJD | テンプレート残存プレースホルダーのベースラインスイープ | done | [baseline-placeholder-sweep.md](./baseline-placeholder-sweep.md) |
| TASK-034MLSwRAMOjT59ZH5kQFn | CLAP host bridge（host feature・ClapLibrary/descriptor+param 合成・intern-once・lifecycle/params/error 写像・ClapPluginFactory） | done | [clap-host-bridge.md](./clap-host-bridge.md) |
| TASK-034LFHv6XBnCj3XsdaEGVG | Issue #99 色変換カーネル closeout | done | [color-conversion-closeout.md](./color-conversion-closeout.md) |
| TASK-034LFHuE6Ii7Wqar445Znn | Issue #99 色変換カーネル final verification | done | [color-conversion-final-verification.md](./color-conversion-final-verification.md) |
| TASK-034KSVlq4M72vjrxvDRQbP | コンポジットカーネル実装（CompositeOver + Crossfade + 共有 f16 コーデック格上げ） | done | [composite-kernels.md](./composite-kernels.md) |
| TASK-034IdQmBa6UdauVn9IG59X | 最小ワークスペースと smoke test の構築 | done | [create-minimal-workspace-and-smoke-test.md](./create-minimal-workspace-and-smoke-test.md) |
| TASK-034JgsFV6CWAjSnPuekBgs | ruviform-plugin-core 共通基盤クレートの作成 | done | [create-plugin-core-foundation.md](./create-plugin-core-foundation.md) |
| TASK-034OpseKB70K4vR6W46RMi | T1. ruviform-observability クレート新設（counter pair・workspace 登録） | done | [create-ruviform-observability-crate.md](./create-ruviform-observability-crate.md) |
| TASK-034JcCRLsFJ7j8zPBi8mXY | ruviform-video-proc クレート骨格の新設 | done | [create-video-proc-crate-skeleton.md](./create-video-proc-crate-skeleton.md) |
| TASK-034JgsHEkIUa3P4jzOizoM | PoC 実測に基づく binding 方式 ADR の決定 | done | [decide-plugin-binding-method-adr.md](./decide-plugin-binding-method-adr.md) |
| TASK-034IdQqbjQcZ0Ztsy5IPpm | mise 開発・CI 検証タスクの定義 | done | [define-mise-dev-and-ci-tasks.md](./define-mise-dev-and-ci-tasks.md) |
| TASK-034JZDNRML000000000007 | Issue #54 文書クローズと全量検証 | done | [denormal-closeout.md](./denormal-closeout.md) |
| TASK-034JZDNRML000000000006 | DenormalFlush ノード実装 + テスト | done | [denormal-flush-node.md](./denormal-flush-node.md) |
| TASK-034JP1tARbXjwUcXVXxNMW | RT 検証基盤の検証文書整備と Issue #22 完了確認 | done | [document-rt-verification-scope.md](./document-rt-verification-scope.md) |
| TASK-034JcCQQD01mBg5JPjNiAS | video-core の所有権・drop・zero-copy 契約と RT ラベルの文書化 | done | [document-video-core-ownership-contract.md](./document-video-core-ownership-contract.md) |
| TASK-034KPgPqF3NHzPtZPvvCHY | edit クレート骨格 + データモデル型・mutation | done | [edit-crate-and-model.md](./edit-crate-and-model.md) |
| TASK-034KPgQj9u5utlkxH3JGqv | edit 評価射影 + テスト・文書クローズ | done | [edit-evaluation-and-closeout.md](./edit-evaluation-and-closeout.md) |
| TASK-034Mrt2jiucAb69qBSYoEg | engine-plugin 統合 final verification + closeout（Full Gate・AC-to-test 証跡・status 同期） | done | [engine-plugin-final-verification.md](./engine-plugin-final-verification.md) |
| TASK-034JglHTIvjo18tMdbO3DP | 採否 ADR 確定とワークスペース帳簿・索引の同期 | done | [finalize-backend-adr-and-sync-docs.md](./finalize-backend-adr-and-sync-docs.md) |
| TASK-034J6PpKd8jNFxEYS4CRdM | 時間演算の早期飽和・Err 修正（i128 中間演算） | done | [fix-time-arithmetic-saturation.md](./fix-time-arithmetic-saturation.md) |
| TASK-034N3OJyIb0GPezXrvD4MM | ICC LUT fixture coverage（intent pair + mft1/mft2/mAB） | done | [icc-lut-fixture-coverage.md](./icc-lut-fixture-coverage.md) |
| TASK-034LFHl0gbFixJl7RmlzQm | IccTransform + IccIntent 実装（feature icc・moxcms 経路） | done | [icc-transform-feature.md](./icc-transform-feature.md) |
| TASK-034J5KQD1rDMZHFRah7pU0 | ruviform-audio-core のデータモデル実装 | done | [implement-audio-core-data-model.md](./implement-audio-core-data-model.md) |
| TASK-034Jglv8LtJHUgWH3wlu0o | ruviform-audio-core の device ポート定義と契約テスト | done | [implement-audio-core-device-port.md](./implement-audio-core-device-port.md) |
| TASK-034JglAvWUFQoLhvYXOw2O | ruviform-audio-device クレート新設と cpal adapter 実装 | done | [implement-audio-device-cpal-adapter.md](./implement-audio-device-cpal-adapter.md) |
| TASK-034JgsGmcPdPRoeRTvicjW | 独立 per-channel planar バッファのゼロコピー借用 view/newtype の実装 | done | [implement-borrowed-planar-view.md](./implement-borrowed-planar-view.md) |
| TASK-034Jf48NGXlyYcluiTnpOR | criterion ベンチ実装 | done | [implement-criterion-benchmarks.md](./implement-criterion-benchmarks.md) |
| TASK-034JxOaMsaSwEIVfvU9MC8 | ruviform-exec スレッド優先度機構の実装 | done | [implement-exec-thread-priority.md](./implement-exec-thread-priority.md) |
| TASK-034JNwTz163N6eT3EJfdaD | FrameRate 型の実装（TimeBase newtype・検証付きコンストラクタ・nominal_fps） | done | [implement-framerate-type.md](./implement-framerate-type.md) |
| TASK-034Jgi3OFauA1SWRPdvnnb | ruviform-io-symphonia バックエンド PoC 実装 | done | [implement-io-symphonia-backend-poc.md](./implement-io-symphonia-backend-poc.md) |
| TASK-034J5KJgwtjFonTMQQIvpX | ruviform-media-core のデータモデル実装 | done | [implement-media-core-data-model.md](./implement-media-core-data-model.md) |
| TASK-034JghH9i5p72LmOoWIPGe | ruviform-media-io 抽象境界クレート実装 | done | [implement-media-io-abstraction.md](./implement-media-io-abstraction.md) |
| TASK-034JgsI5J4HU4qcoEBQfDx | 薄い CLAP/VST3 規格アダプタの完成 | done | [implement-plugin-adapters.md](./implement-plugin-adapters.md) |
| TASK-034JcD2Xk1x6RZjHn779an | RGB→YUV 変換と Rgba8↔RgbaF16 カーネルの実装 | done | [implement-rgb-to-yuv-and-f16-kernels.md](./implement-rgb-to-yuv-and-f16-kernels.md) |
| TASK-034JxNNzWfWAMoQu4EgpqG | ruviform-rt 中核型の実装（Error・RtContext・Processor・MasterClock） | done | [implement-rt-core-types.md](./implement-rt-core-types.md) |
| TASK-034JxO9ex0aVfyvjYkcaWT | ruviform-rt 退役チャネル（遅延ドロップ）の実装 | done | [implement-rt-retire-channel.md](./implement-rt-retire-channel.md) |
| TASK-034JxO9XLg2D5zvepCA7ry | ruviform-rt スナップショット交換の実装 | done | [implement-rt-snapshot-exchange.md](./implement-rt-snapshot-exchange.md) |
| TASK-034JxO8rp2rxpfFdipPzUU | ruviform-rt SPSC チャネルの実装 | done | [implement-rt-spsc-channel.md](./implement-rt-spsc-channel.md) |
| TASK-034Jf3MvCqPUi8pRT6jjES | 候補バックエンド実装と共有サポート | done | [implement-simd-poc-candidate-backends.md](./implement-simd-poc-candidate-backends.md) |
| TASK-034Jf3FazZdLzFAFTDXUa3 | スカラー参照カーネルと共通型 | done | [implement-simd-poc-scalar-kernels.md](./implement-simd-poc-scalar-kernels.md) |
| TASK-034J5KR5ueilYgXdyyF4ZU | ruviform-video-core のデータモデル実装 | done | [implement-video-core-data-model.md](./implement-video-core-data-model.md) |
| TASK-034JcD1UTgLlW1lRoWI1Ng | video-proc カーネル API 形状と YUV→RGB 変換カーネルの実装 | done | [implement-yuv-to-rgb-kernel.md](./implement-yuv-to-rgb-kernel.md) |
| TASK-W49nvd39gPw0nVVlHofI5L | ruviform-ui: window contract types (WindowId/WindowEvent/RoutedEvent/CloseDecision/WindowHandle) | done | [issue-181-01-window-contract-types.md](./issue-181-01-window-contract-types.md) |
| TASK-klq30A6pIf2U4FUAdY3iuL | ruviform-ui: UiRuntime multi-window routing (handle_window/drain_jobs_routed/cancel_window_jobs) | done | [issue-181-02-runtime-routing.md](./issue-181-02-runtime-routing.md) |
| TASK-5bJ6JAK2lt9CHr2ZLSR42R | ruviform-ui: fake-host multi-window contract tests | done | [issue-181-03-fake-host-contract-tests.md](./issue-181-03-fake-host-contract-tests.md) |
| TASK-kIMP2tx8yXfzMncvaRoJVK | ruviform-ui-webview: SessionConfig + run_multi (wry/tao multi-window host) | done | [issue-181-04-webview-run-multi.md](./issue-181-04-webview-run-multi.md) |
| TASK-xyyP2DWC1H1s2hnWVJ6bbA | demo_multi example + docs sync + full verification + impl record | done | [issue-181-05-demo-multi-and-verification.md](./issue-181-05-demo-multi-and-verification.md) |
| TASK-J9cxFOsGKSoiMZKmpXRMOp | ruviform-ui: dock model layer (ViewInstanceId/Axis/DockNode/DockTree/DockPath/DockError) | done | [issue-182-01-dock-model.md](./issue-182-01-dock-model.md) |
| TASK-341XTt270miIp3zJjTK07P | ruviform-ui: DockWorkspace + two-phase view move + close policy + ops + DockEvent | done | [issue-182-02-workspace-move.md](./issue-182-02-workspace-move.md) |
| TASK-yUcTer6Jz05I9eBkeM4mQX | ruviform-ui: layout persistence schema + geometry correction + WindowConfig.position plumbing | done | [issue-182-03-persistence-position.md](./issue-182-03-persistence-position.md) |
| TASK-AirSKtHS1TtcoQZnYr9s9U | ruviform-ui: dock integration tests + demo_dock example + docs/index consistency + verification | done | [issue-182-04-tests-demo-verification.md](./issue-182-04-tests-demo-verification.md) |
| TASK-034QkFsRkLkoZqHkFaRHhA | plugin-core: param_write contract types + coalescing + same-offset docs | done | [issue-186-01-plugin-core-param-write.md](./issue-186-01-plugin-core-param-write.md) |
| TASK-034QkFzCiPKInh3jrWFr9b | bridge: TargetId binding + dispatch ledger + cap matrix param fields | done | [issue-186-02-bridge-target-dispatch.md](./issue-186-02-bridge-target-dispatch.md) |
| TASK-034QkFAipKgHjxJRSBAYXt | tests: delivery E2E + rejection + time conversion + observability | done | [issue-186-03-delivery-observability-tests.md](./issue-186-03-delivery-observability-tests.md) |
| TASK-034QkFB3oVWTfZxG7hGYMc | rt lane: bounded dispatch no-alloc + plan vault keep/retire + verification | done | [issue-186-04-rt-verification.md](./issue-186-04-rt-verification.md) |
| TASK-034Qx4QQvbZvxt6cuQGGid | Issue #187-T1: ruviform-audio-core input port contract | done | [issue-187-t1-input-port-contract.md](./issue-187-t1-input-port-contract.md) |
| TASK-034Qx4SABRy9cBHiKXWlcL | Issue #187-T2: cpal adapter input surface | done | [issue-187-t2-cpal-input-surface.md](./issue-187-t2-cpal-input-surface.md) |
| TASK-034Qx4ThH8UjhQhXw4woIO | Issue #187-T3: mock input backend + RT handoff PoC | done | [issue-187-t3-mock-input-poc.md](./issue-187-t3-mock-input-poc.md) |
| TASK-5dlmLfhxOjW0IccOc3xUBO | media-core: midi + timed_input vocabulary, mapper and NoteLedger | done | [issue-188-01-media-core-midi-vocabulary.md](./issue-188-01-media-core-midi-vocabulary.md) |
| TASK-CKJ1IaOWUc0aUcraqOPD3q | plugin-core: EventKind::Midi + wire tag + merge_call_events | done | [issue-188-02-plugin-core-eventkind-midi-merge.md](./issue-188-02-plugin-core-eventkind-midi-merge.md) |
| TASK-gOmhMICXSl8m4R5UD5kRf5 | audio-core: midi input port trait + mock | done | [issue-188-03-audio-core-midi-input-port.md](./issue-188-03-audio-core-midi-input-port.md) |
| TASK-54RLlHRS4QloXdmLEZEM8L | bridge: InputStage + merge + NoteLedger wiring + control/status/attach | done | [issue-188-04-bridge-input-stage-merge.md](./issue-188-04-bridge-input-stage-merge.md) |
| TASK-RjMEQhLpvbKbkrKJQvqsz8 | adapters: cap.rs note/channel-event fields + CLAP translation + VST3 honesty | done | [issue-188-05-adapter-capability-honesty.md](./issue-188-05-adapter-capability-honesty.md) |
| TASK-lzn8pZrsr6ThLMc0iKw8bN | tests: midi_delivery E2E + rt_ no-alloc + full verification | done | [issue-188-06-midi-delivery-tests.md](./issue-188-06-midi-delivery-tests.md) |
| TASK-034QfdkD1MggWcolouiYsy | issue-189 T-1 BeatPosition/Ppqn/TickPosition canonical types | done | [issue-189-t-1-beat-canonical-types.md](./issue-189-t-1-beat-canonical-types.md) |
| TASK-034QfdpefNAIoQr57VQZlb | issue-189 T-2 TempoMap Beat basis + event/position/ABI migration | done | [issue-189-t-2-tempomap-beat-basis.md](./issue-189-t-2-tempomap-beat-basis.md) |
| TASK-034QfdqgoGMqtqJ2HHFDdG | issue-189 T-3 golden/RT verification + impl record + output contract | done | [issue-189-t-3-golden-verification-impl-record.md](./issue-189-t-3-golden-verification-impl-record.md) |
| TASK-034Qh5tYhNjMkLoPqRs | Issue #190 T-1: canonical transport contract types (media-core) | done | [issue-190-t-1-canonical-contract-types.md](./issue-190-t-1-canonical-contract-types.md) |
| TASK-034Qh6uIoPqWsErTyU | Issue #190 T-2: transport controller + RT span scheduler | done | [issue-190-t-2-span-scheduler-apply.md](./issue-190-t-2-span-scheduler-apply.md) |
| TASK-034Qh7vBnMxZaSdFgH | Issue #190 T-3: plugin transport bridge + Mock Driver PoC | done | [issue-190-t-3-plugin-bridge-mock-poc.md](./issue-190-t-3-plugin-bridge-mock-poc.md) |
| TASK-Awm44usEpigGUWYXNxfbWd | Issue #196 T-1: plugin-core bus role + enumeration contract | done | [issue-196-t-1-plugin-core-bus-enumeration.md](./issue-196-t-1-plugin-core-bus-enumeration.md) |
| TASK-HfEXpjuOcYSGP35UqpabKo | Issue #196 T-2: media-core receipt/fault pop + host bus enumeration | done | [issue-196-t-2-host-media-bus-enumeration.md](./issue-196-t-2-host-media-bus-enumeration.md) |
| TASK-mO3RtjrUoyUadajAWyCadp | Issue #196 T-3: ruviform-audio-plugin-bridge crate | done | [issue-196-t-3-bridge-crate.md](./issue-196-t-3-bridge-crate.md) |
| TASK-QQTcvWDmsdkWt6I3mJdYEa | Issue #196 T-4: verification + capability evidence + impl records | done | [issue-196-t-4-verification-evidence.md](./issue-196-t-4-verification-evidence.md) |
| TASK-034QdV8LJ4ZnIRqDUSRakb | T1 Consumer影響判定・証拠テンプレート（PR/Issue template群） | done | [issue-219-t1-impact-templates.md](./issue-219-t1-impact-templates.md) |
| TASK-034QdV9z3ctYdM4HMD0MsZ | T2 consumer-verify.yml Cross-Repo CI lane（preflight/verify×2/summary） | done | [issue-219-t2-consumer-verify-workflow.md](./issue-219-t2-consumer-verify-workflow.md) |
| TASK-034QdVap6IcvMjOYnaWzCR | T3 AGENTS.md の ci.yml 記述 drift 修正 | done | [issue-219-t3-agents-ci-drift.md](./issue-219-t3-agents-ci-drift.md) |
| TASK-034QdVbbaa7jKsLfhGKepC | T4 検証・IR 記録（actionlint・doc audit・実装記録） | done | [issue-219-t4-verify-ir.md](./issue-219-t4-verify-ir.md) |
| TASK-034QsXMp7VDgLZ2W5FaZxq | Issue #223 MSRV 1.89 限定 pedantic clippy 失敗の整理 | done | [issue-223-msrv-clippy-triage.md](./issue-223-msrv-clippy-triage.md) |
| TASK-M5TTFmL3gjHvlBOb7sPDjF | Issue #238 l1_proxy_e2e 壁時計 sleep 同期を barrier/有界 poll へ置換 | done | [issue-238-l1-proxy-e2e-flake.md](./issue-238-l1-proxy-e2e-flake.md) |
| TASK-034Qxfh3HNkNZecEqGSpM1 | T1 prefetch boundary-first fault + dead-generation sequence (Issue #244 P2 amendment) | done | [issue-244-t1-boundary-first.md](./issue-244-t1-boundary-first.md) |
| TASK-034QxfhGvckBvhC0QQYCgA | T2 prefetch End commit-once under backpressure (Issue #244) | done | [issue-244-t2-end-commit-once.md](./issue-244-t2-end-commit-once.md) |
| TASK-034Qxfiiwcs7VUi4SIArot | T3 seeded fault fixture + discard_pending coverage (Issue #244) | done | [issue-244-t3-seeded-coverage.md](./issue-244-t3-seeded-coverage.md) |
| TASK-034QxfiWo8R8kkqMasIoQm | T4 verification + review-protocol + impl record (Issue #244) | done | [issue-244-t4-verification-record.md](./issue-244-t4-verification-record.md) |
| TASK-034QAsO7suXcts3TkIr3Un | Issue 249 prefetch Ended outcome must emit End marker | done | [issue-249-prefetch-ended-outcome-must-emit-end-marker.md](./issue-249-prefetch-ended-outcome-must-emit-end-marker.md) |
| TASK-034RoWzXiLur2zHG5iHl7u | T1 ruviform-io-file クレート新設・公開契約・Mock Publish (Issue #295) | done | [issue-295-t1-crate-and-contract.md](./issue-295-t1-crate-and-contract.md) |
| TASK-034RoWAHTxefOCVrz2FVL4 | T2 共有台帳更新・main 取込・最終検証・IR (Issue #295) | done | [issue-295-t2-ledger-verify.md](./issue-295-t2-ledger-verify.md) |
| TASK-034RBIWkzzcEXV0X3v5ySr | T1 OsFileOps 実 OS アダプタ + factory 接続 (Issue #296) | done | [issue-296-t1-os-adapter.md](./issue-296-t1-os-adapter.md) |
| TASK-034RBIX9gn32W8GMyhNMqw | T2 実 FS 結合試験・複数プロセス競合 (Issue #296) | done | [issue-296-t2-real-fs-tests.md](./issue-296-t2-real-fs-tests.md) |
| TASK-034RBIYghnEBLw9llcomyd | T3 保証範囲文書化・impl record・最終検証 (Issue #296) | done | [issue-296-t3-docs-verify.md](./issue-296-t3-docs-verify.md) |
| TASK-QNzC22ZqitcqDhco1hHMKj | Issue #307 T1: attach 時 frame_base 検証 context（accessors + typed mismatch 拒否） | done | [issue-307-t1-attach-frame-base-verified-context.md](./issue-307-t1-attach-frame-base-verified-context.md) |
| TASK-034Rp7jV966SX0fG78cOBL | Issue #310-T1 browser-test パッケージ骨格・serve・page protocol | done | [issue-310-01-browser-harness-package.md](./issue-310-01-browser-harness-package.md) |
| TASK-034Rp7kQsMmyk0At2QYrwX | Issue #310-T2 共有 Fixture/Driver/Assertion (gesture/receipt/state/theme/canvas/a11y) | done | [issue-310-02-shared-fixtures-and-drivers.md](./issue-310-02-shared-fixtures-and-drivers.md) |
| TASK-034Rp7lRJbB15NEGHdCXpU | Issue #310-T3 実ブラウザ spec・Consumer-shaped fixture・CI/evidence 接続 | done | [issue-310-03-specs-ci-evidence.md](./issue-310-03-specs-ci-evidence.md) |
| TASK-8hTQjGqBYDGbC2jTMks1j8 | T1 versioned fixture corpus + ui/guitest package 骨格（Issue #311） | done | [issue-311-t1-fixture-corpus.md](./issue-311-t1-fixture-corpus.md) |
| TASK-jXU3ZGuh6DP2FfCSvkx5FC | T2 TS wire validators — ipcwire/bindingwire + conformance テスト（Issue #311） | done | [issue-311-t2-ts-wire-validators.md](./issue-311-t2-ts-wire-validators.md) |
| TASK-KayYlp12EsEWZZKk2pMZgF | T3 recorder/replay/oracle + consumer mocks + seeded シナリオ（Issue #311） | done | [issue-311-t3-recorder-replay-oracle.md](./issue-311-t3-recorder-replay-oracle.md) |
| TASK-uyCbW614DqUYmATBzUrQxz | T4 fault matrix + Rust conformance lanes + mise/CI/AGENTS 同期 + IR（Issue #311） | done | [issue-311-t4-fault-matrix-lanes.md](./issue-311-t4-fault-matrix-lanes.md) |
| TASK-034RBNKfGF6WS4OfHfW1of | Issue #327 CI計測Baseline 計測実施・文書化 | done | [issue-327-t1-baseline-measurement.md](./issue-327-t1-baseline-measurement.md) |
| TASK-uRQkJW5851dqMXehrKNZkI | Issue #328 T1 — ci.yml / consumer-impact-check.yml へ PR concurrency block 適用 | done | [issue-328-t1-ci-pr-concurrency.md](./issue-328-t1-ci-pr-concurrency.md) |
| TASK-034RBIMQzuYAewg2JXTXOk | T1 mise task install/freshness 分離 + ui-verify setup ステップ（Issue #330） | done | [issue-330-t1-mise-ui-install-dedup.md](./issue-330-t1-mise-ui-install-dedup.md) |
| TASK-O3Q2a0lPvct9PFKBgBig2j | T1 Playwright Chromium cache・準備 step 分離（Issue #332） | done | [issue-332-t1-playwright-browser-cache.md](./issue-332-t1-playwright-browser-cache.md) |
| TASK-69XeSXbqn9bazeVEgD0LzH | T1 ci/impact classifier + gate 実装（Issue #333） | done | [issue-333-t1-impact-classifier.md](./issue-333-t1-impact-classifier.md) |
| TASK-auXqAVyXzCi31qldUWzY6t | T2 ci.yml Shadow wiring（impact/ci-gate job・trigger types・mise task・AGENTS.md）（Issue #333） | done | [issue-333-t2-ci-gate-shadow-wiring.md](./issue-333-t2-ci-gate-shadow-wiring.md) |
| TASK-AacpjXZ02dbHUiacENEOSZ | T1 SinkDegraded Clone derive 追加（Issue #336） | done | [issue-336-t1-sink-degraded-clone.md](./issue-336-t1-sink-degraded-clone.md) |
| TASK-034MrsLrqKnu9N3YfxnSGl | L0 Shared スロット + endpoint inner（SharedSlot/SendCore・3 段 lock・epoch attach・catch_unwind 収束） | done | [l0-shared-slot-and-endpoint-inner.md](./l0-shared-slot-and-endpoint-inner.md) |
| TASK-034NlPHWwEG0sAKxw5XB88 | L1 bundle child final verification（Full Gate・AC 証拠・IR・closeout） | done | [l1-bundle-child-final-verification.md](./l1-bundle-child-final-verification.md) |
| TASK-034NlQnJQdVWzhM6WxhAM2 | L1 bundle E2E + Failed 粒度検証（decision table・inert・doorbell・fan-out・dedicated 非回帰） | done | [l1-bundle-e2e-and-granularity-verification.md](./l1-bundle-e2e-and-granularity-verification.md) |
| TASK-034NlQlLZkVZRtEVfF8Sdp | L1 子構造再編（Shared/Inner→L1Child+InstShared split・demux・converge split・Destroying gate） | done | [l1-child-instance-split.md](./l1-child-instance-split.md) |
| TASK-034LNY736TLX2w3k4RFFZM | plugin-core L1 基盤（契約移設 + Lifecycle Failed + wire codec） | done | [l1-core-contracts-and-wire-codec.md](./l1-core-contracts-and-wire-codec.md) |
| TASK-034OpsGOwquB9NPINeMKyz | T12. docs/ledger・最終検証（#120/#101 trigger・index・mise 全量・audit） | done | [l1-diagnostics-ledger-and-verify.md](./l1-diagnostics-ledger-and-verify.md) |
| TASK-034MrsLWkKNHnu7jX1ghp8 | L1 endpoint（leg 移出・arm channel・pending_arm・leg 返却・drain-before-early-return） | done | [l1-endpoint-arm-and-leg-return.md](./l1-endpoint-arm-and-leg-return.md) |
| TASK-034N3QIPDPFbB37OxHMZLy | Issue #123 L1 内部クリーンアップ（frame-I/O dedup・runner_program 解決・gui_open 除去・audio-core doc 修正） | done | [l1-internal-cleanup.md](./l1-internal-cleanup.md) |
| TASK-034LNYf8Qj1NeGAKmaP7B4 | plugin-host L1 プロキシ制御面（spawn/handshake/コマンド転送/クラッシュ検出） | done | [l1-proxy-control-plane.md](./l1-proxy-control-plane.md) |
| TASK-034NlQmTAKwSUzU49wWswM | L1 route pool + 隔離選択面（OutOfProcessIsolation・Route struct variant・pool lifecycle） | done | [l1-route-pool-and-isolation-surface.md](./l1-route-pool-and-isolation-surface.md) |
| TASK-034LNYmL2ezSeFFgddaeDX | L1 共有メモリデータ面 + RT 脚 + pass-through | done | [l1-shm-data-plane-and-rt-legs.md](./l1-shm-data-plane-and-rt-legs.md) |
| TASK-034NOXIiAgzm2Ur1Ud7nLt | L1 supervisor core（src/supervisor.rs — watch/poll state machine・attempt replay・checkpoint bound・host binding） | done | [l1-supervisor-core.md](./l1-supervisor-core.md) |
| TASK-034NOXIS4eJ3G8peEvr6pq | L1 supervisor E2E（実子 restart・bundle burst・crash-loop GaveUp・restore 分類・Send witness） | done | [l1-supervisor-e2e.md](./l1-supervisor-e2e.md) |
| TASK-034NOXJpTlwWqDQf7WFQFQ | L1 supervisor final verification（Full Gate・AC 証拠・IR・status 同期） | done | [l1-supervisor-final-verification.md](./l1-supervisor-final-verification.md) |
| TASK-034NOXyYOTNwDoH2c7VhrV | l1-test-worker fault flags（--fail-restore・--state-unsupported・--fail-initialize・--fail-start-processing） | done | [l1-test-worker-fault-flags.md](./l1-test-worker-fault-flags.md) |
| TASK-034NXuA0ENhlZxgP8zpnqd | L1 video docs sweep（crate README・親文書 re-engagement 注記・test-spec v3 reconcile・literal sweep） | done | [l1-video-docs-sweep.md](./l1-video-docs-sweep.md) |
| TASK-034NXuzqcpqV0zLNlPSHUR | L1 video fixture + 公開面 E2E（l1-test-video-worker + tests/l1_video_e2e.rs — Failed parity・frame 内容・cross-kind） | done | [l1-video-fixture-and-e2e.md](./l1-video-fixture-and-e2e.md) |
| TASK-034NXuq6kT7i5UmNviPjy6 | L1 video host surface（video() façade・Route/Slot video arm・descriptors kind-tag・create_video_instance・Creating ゲート・vshm::Segment・VideoL1Instance） | done | [l1-video-host-surface.md](./l1-video-host-surface.md) |
| TASK-034NXuAACPRKeBy1TSW1EO | L1 video/NONRT final verification（Full Gate・AC/R-V 証拠・IR・status 同期・closeout） | done | [l1-video-l1-final-verification.md](./l1-video-l1-final-verification.md) |
| TASK-034NXupvMzQElYkWehw8ln | L1 video runner + erased surface（erase_video・run_video・VideoEntry・vshm attach・reference bin・runner E2E） | done | [l1-video-runner-and-erased-surface.md](./l1-video-runner-and-erased-surface.md) |
| TASK-034NlQlfkFL52FRHRjhYmr | L1 wire v2 codec + 全系統 v2 cutover（envelope・新動詞・InstanceLimit・registry・harness） | done | [l1-wire-v2-codec-cutover.md](./l1-wire-v2-codec-cutover.md) |
| TASK-034NlPHnwYkRTWHq1nvYvo | L1 wire v2 docs/metadata sweep（README・bin doc・test-spec・親文書 addenda・literal sweep） | done | [l1-wire-v2-docs-sweep.md](./l1-wire-v2-docs-sweep.md) |
| TASK-034NXuhy2LmLb9qDBo7gj6 | L1 wire v3 codec + vshm 定盤 + InvalidValue（video 動詞 tag 12-22・Reply 4-7・video 型 codec・WIRE_MAX_VIDEO_*） | done | [l1-wire-v3-video-codec.md](./l1-wire-v3-video-codec.md) |
| TASK-034NhK97jcINkcEKDNbpum | L1 wire バージョン方針の記録と契約検証の補強 | done | [l1-wire-version-policy.md](./l1-wire-version-policy.md) |
| TASK-034Qfs0cxlLQ9xk1dN3NJM | T-1 typed buffer arena + io views + compiler (schedule/slots/PDC) | done | [m2-t1-typed-compiler-buffer-io.md](./m2-t1-typed-compiler-buffer-io.md) |
| TASK-034Qfs3QuOMXdVC8ZnuKz6 | T-2 typed RT runtime: GraphProcessor, GraphRuntime/RtRuntime, activation, faults | done | [m2-t2-typed-rt-runtime-activation.md](./m2-t2-typed-rt-runtime-activation.md) |
| TASK-034Qfs4U6jKNPhfoasgTFI | T-3 acceptance tests + ownership PoC + fault injection | done | [m2-t3-tests-ownership-poc-faults.md](./m2-t3-tests-ownership-poc-faults.md) |
| TASK-034Qfs62FTCiTj3vNuOwAz | T-4 golden legacy, EX/TA traceability, merge+lockfile+verification, impl record | done | [m2-t4-golden-traceability-verification.md](./m2-t4-golden-traceability-verification.md) |
| TASK-034JNwTZ8WalgowFahLc8E | Timecode API の FrameRate 移行（破壊的シグネチャ置換と呼び出し側移行） | done | [migrate-timecode-to-framerate.md](./migrate-timecode-to-framerate.md) |
| TASK-034KoQ7sXAxjPHQbdWqDVW | Issue #90 文書クローズと全量検証 | done | [music-time-closeout.md](./music-time-closeout.md) |
| TASK-gJ0wsCnJrG8Wo0JQSENFtz | 音楽時間 ABI 整合検証 + Issue #33 クローズ | done | [music-time-design-verify.md](./music-time-design-verify.md) |
| TASK-034KoQ1xQURHojhURlBqP6 | media-core::music モジュール実装 + テスト | done | [music-time-media-core-module.md](./music-time-media-core-module.md) |
| TASK-034KpKrywhQhq1kR5WqrgS | TransportInfo.musical + アダプタ写像実装 | done | [musical-transport-abi-impl.md](./musical-transport-abi-impl.md) |
| TASK-034KpKs0YVpCCF1YlXTyqt | Issue #91 文書クローズと全量検証 | done | [musical-transport-closeout.md](./musical-transport-closeout.md) |
| TASK-034JRcXSEdyHdLa42O9KKP | Issue #24 文書クローズと全量検証 | done | [offline-exec-closeout.md](./offline-exec-closeout.md) |
| TASK-034JRcXokykm0XlpOqcor0 | 並列ワーカープール駆動（有界キュー・順序再構成） | done | [offline-exec-parallel.md](./offline-exec-parallel.md) |
| TASK-034JRcWTeJbPQAh5fUwPZs | オフライン駆動プリミティブ（CancellationToken/Progress/OfflineError/逐次ドライバ） | done | [offline-exec-primitives.md](./offline-exec-primitives.md) |
| TASK-034L6ycSsF8VW32LLoNxXr | Implement video plugin core contract | done | [ofx-core-contract-implementation.md](./ofx-core-contract-implementation.md) |
| TASK-034L6yiJbB3boniBUWvfmC | OFX host closeout | done | [ofx-host-closeout.md](./ofx-host-closeout.md) |
| TASK-034L6yib4dmV9rd1MUZxbw | OFX host final verification | done | [ofx-host-final-verification.md](./ofx-host-final-verification.md) |
| TASK-034N3SaVVNHQRJjWEndfH7 | OFX host window lease + clip props clone 実装 | done | [ofx-host-window-lease-implementation.md](./ofx-host-window-lease-implementation.md) |
| TASK-034L6yeVFSaiVUoV4RiRtm | Implement native OFX runtime | done | [ofx-native-runtime-implementation.md](./ofx-native-runtime-implementation.md) |
| TASK-034L6yh0fBmRabh1TMLv2F | Implement OFX video rendering | done | [ofx-render-implementation.md](./ofx-render-implementation.md) |
| TASK-034N9p1bBWmE7AioxpndKq | OFX UAF residuals — non-catalog window lease + lifecycle hardening | done | [ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md](./ofx-uaf-residuals-non-catalog-window-lease-lifecycle-hardening.md) |
| TASK-034QidaAEbNmHe5Q6M5sRI | T-1 ChannelLevel::measure extraction + deps + compile binding centralization | done | [p185-t1-level-extract-and-binding-centralization.md](./p185-t1-level-extract-and-binding-centralization.md) |
| TASK-034QidbvzKgQyKTfniAhWG | T-2 RtRuntime probe plumbing: frame_cursor, published_plan, command drains, tap sites, deck channels | done | [p185-t2-runtime-probe-plumbing-taps.md](./p185-t2-runtime-probe-plumbing-taps.md) |
| TASK-034Qidce7Pq9AxEezYMaNE | T-3 typed::probe contract: capability resolver, ProbeBroker, ProbeDeck/RtProbe, ProbeCollector | done | [p185-t3-probe-contract-broker-collector.md](./p185-t3-probe-contract-broker-collector.md) |
| TASK-034QidcTgfNesKcwIF1aZ7 | T-4 PoC acceptance tests, rt alloc, perf evidence, impl record, merge+verify | done | [p185-t4-probe-poc-tests-verification.md](./p185-t4-probe-poc-tests-verification.md) |
| TASK-034Rp2aMrgIhyU5ZATHDQz | T-1 core primitives: Gauge/Histogram + RT-lane tests | done | [p277-t1-core-primitives-gauge-histogram.md](./p277-t1-core-primitives-gauge-histogram.md) |
| TASK-034Rp2bMkXB7Xa0FKSWPw9 | T-2 source contract: descriptors/SourceId/Registrar/Lease/lifecycle + tests + README | done | [p277-t2-source-contract-registration-lifecycle.md](./p277-t2-source-contract-registration-lifecycle.md) |
| TASK-034RTukbgTWDcKGHBdKYYN | P279-T1 l1/sources.rs 4 Descriptor + AudioSourceLeases + slot state + mod tests | done | [p279-t1-l1-sources-descriptors-leases.md](./p279-t1-l1-sources-descriptors-leases.md) |
| TASK-034RTukTevxIOK0EVaR8A3 | P279-T2 create_instance/destroy registrar wiring + PluginHost注入面 + tests | done | [p279-t2-create-destroy-registrar-wiring.md](./p279-t2-create-destroy-registrar-wiring.md) |
| TASK-034IdQiK2RW8ol4gpyUYLb | Rust toolchain pin と lockfile 確定 | done | [pin-rust-toolchain-and-lockfile.md](./pin-rust-toolchain-and-lockfile.md) |
| TASK-034OpskKobU4qEsX7F7IYc | T6. stderr piped + drain thread（案A・FFI 局所化・fixture knob） | done | [pipe-and-drain-l1-stderr.md](./pipe-and-drain-l1-stderr.md) |
| TASK-034MrsDzlHRpUHHLtYkZdN | plugin-core endpoint 前提面（EndpointUnavailable・fallible encode・Send capability・view impl） | done | [plugin-core-endpoint-primitives.md](./plugin-core-endpoint-primitives.md) |
| TASK-034LNYeFTd7EgCwbDA0GWD | ruviform-plugin-export 新設 + run() ランナー + l1-runner bin | done | [plugin-export-crate-and-l1-runner.md](./plugin-export-crate-and-l1-runner.md) |
| TASK-034Mrt1HhkO2vVKksdYxgA | endpoint process/process_audio shell + グラフ E2E + rt_ レーン（bound clip・in-bus-first・source-position 慣行） | done | [plugin-graph-e2e-and-rt-lane.md](./plugin-graph-e2e-and-rt-lane.md) |
| TASK-034JZPGUI0000000000006 | Issue #30 ADR + 文書クローズ | done | [plugin-gui-poc-closeout.md](./plugin-gui-poc-closeout.md) |
| TASK-034JZPGUI0000000000005 | plugin-gui PoC ハーネス + 実機実験 | done | [plugin-gui-poc-harness.md](./plugin-gui-poc-harness.md) |
| TASK-034LNYndN4rzshflcqGHt5 | PluginHost コンポジット + 選択 API + doc 整合 | done | [plugin-host-composite-and-l1-e2e.md](./plugin-host-composite-and-l1-e2e.md) |
| TASK-034MrsUoYlicb5uG8XpODT | PluginHost endpoint 公開面（prepare_endpoint・latency_frames・統一拒否表・crate doc） | done | [plugin-host-endpoint-surface.md](./plugin-host-endpoint-surface.md) |
| TASK-Cay381BHNWhN855kjvMcL1 | Issue #74 文書クローズと全量検証 | done | [plugin-host-l0-closeout.md](./plugin-host-l0-closeout.md) |
| TASK-AMKTeO5Inr8CXr3RzX3uOV | ruviform-plugin-host クレート実装 + テスト | done | [plugin-host-l0-crate.md](./plugin-host-l0-crate.md) |
| TASK-034JZLISO0000000000005 | plugin-host API 整合検証 + Issue #31 クローズ | done | [plugin-isolation-verify.md](./plugin-isolation-verify.md) |
| TASK-034LNYvBEBtmjJIUa6xJMi | Issue #76 プラグイン L1 分離 closeout | done | [plugin-l1-closeout.md](./plugin-l1-closeout.md) |
| TASK-034LNYv07MQpWKTmfQ3vX0 | Issue #76 プラグイン L1 分離 final verification | done | [plugin-l1-final-verification.md](./plugin-l1-final-verification.md) |
| TASK-034JfHz5u1uXcPkG8d8t4B | PoC 採否 ADR 確定（エビデンス集約・confirm/revise・accepted 遷移） | done | [poc-adr-finalize.md](./poc-adr-finalize.md) |
| TASK-034JgsGOtyuy2MkLRLZQI5 | clack 系による CLAP binding PoC（最小ゲインプラグイン） | done | [poc-clap-binding-adapter.md](./poc-clap-binding-adapter.md) |
| TASK-034JfHyxKSwvhiQ0sKqjXQ | PoC フロントエンド比較と配布検証（Solid 移植・単一バイナリ・OS 別方針） | done | [poc-frontend-and-dist.md](./poc-frontend-and-dist.md) |
| TASK-034JfH9TmzdaGlsimZsJiq | PoC メッセージ境界計測（JSON RTT・バイナリ pull・evaluate_script push） | done | [poc-message-boundary.md](./poc-message-boundary.md) |
| TASK-034JfHavtGmdqmRGyDAbch | PoC ネイティブサーフェス合成（wgpu 隣接構成・PR #1762 評価） | done | [poc-native-surface.md](./poc-native-surface.md) |
| TASK-034JfGDrzj4yLZMu9A4qHS | PoC 描画バックエンド計測（canvas2d/webgl2/webgpu フレーム統計） | done | [poc-render-backends.md](./poc-render-backends.md) |
| TASK-034JfGCUwtk0lzJvJPr36B | PoC UI ハーネスホスト（独立ワークスペース・wry+tao・計測契約） | done | [poc-ui-harness-host.md](./poc-ui-harness-host.md) |
| TASK-034JgsHeZt7Bm40uBMo9wg | coupler-rs vst3 による VST3 binding PoC（最小ゲインプラグイン） | done | [poc-vst3-binding-adapter.md](./poc-vst3-binding-adapter.md) |
| TASK-034MLSyEUOp70P1hRsepM9 | 実プラグイン L1 E2E + opt-in lane（tests/l1_real_adapter_e2e.rs 両規格・mise task・l1-real-verify job・OFX 境界明文化・doc 同期） | done | [real-adapter-e2e-and-lane.md](./real-adapter-e2e-and-lane.md) |
| TASK-034MLSzaZebh5KuCkE4guN | real-adapter final verification + closeout（Full Gate・opt-in lane 実測・AC 証跡・status 同期） | done | [real-adapter-final-verification.md](./real-adapter-final-verification.md) |
| TASK-034MLSxXJkDzlYvxqtGWDY | 規格別 runner bin（required-features=["host"]・DLL_PREFIX fixture 解決・RUVIFORM_L1_REAL_PLUGIN_DIR・compile-time 固定 factories） | done | [real-adapter-runner-bins.md](./real-adapter-runner-bins.md) |
| TASK-034Jf4gzf7OAwNOoW3t9WV | impl record と文書整合 | done | [record-impl-and-sync-docs.md](./record-impl-and-sync-docs.md) |
| TASK-034IwCiYXp6ub3O8Brcs3L | 設定ファイルのレビューと最小修正 | done | [review-config-files-minimal-edits.md](./review-config-files-minimal-edits.md) |
| TASK-034IwCiByN74V3hysJ9iTR | AGENTS.md の Ruviform 向け節単位改稿 | done | [rewrite-agents-for-ruviform.md](./rewrite-agents-for-ruviform.md) |
| TASK-034IwCbgksOGvhF5E4CQbp | README.md の Ruviform 向け全面改稿 | done | [rewrite-readme-for-ruviform.md](./rewrite-readme-for-ruviform.md) |
| TASK-034LFHlxNkMOGDilPsdP0f | 既存カーネルの RgbaF32 arm 拡張（CompositeOver/Crossfade/AffineTransform） | done | [rgba-f32-composite-affine-arms.md](./rgba-f32-composite-affine-arms.md) |
| TASK-034LFH4wXKj32ElWzaI2lb | RgbaF32 pixel format + transfer/primaries 内部基盤 | done | [rgba-f32-format-and-transfer-internals.md](./rgba-f32-format-and-transfer-internals.md) |
| TASK-034KV6AzIxXBCyJdl6BFjk | RT 観測計測ハーネスの実装（PoC scaffolding・criterion ベンチ・スモーク配線） | done | [rt-observability-bench-harness.md](./rt-observability-bench-harness.md) |
| TASK-034KV6AXTsAaxjMCc4m8px | RT 観測 PoC の計測実行・experiment/impl 記録・#103 判定フィードバック | done | [rt-observability-poc-run-and-report.md](./rt-observability-poc-run-and-report.md) |
| TASK-034JglHs8cNMDOvyZFeUpc | PoC example（list_devices・play_sine）実機手順と実験ログ | done | [run-device-poc-examples-and-exp-log.md](./run-device-poc-examples-and-exp-log.md) |
| TASK-034JPwYHzq8qU5fxLnMLlO | 低負荷再計測と生データ収集 | done | [run-dispatch-remeasure.md](./run-dispatch-remeasure.md) |
| TASK-034IwCjptfvbSFBQaSKigx | プレースホルダー最終スイープ・総合検証と ADR メタデータ編集 | done | [run-placeholder-sweep-and-verification.md](./run-placeholder-sweep-and-verification.md) |
| TASK-034Jf49f5gwAy5fpTPvXG9 | ベンチ実行と結果収集 | done | [run-simd-bench-and-collect-results.md](./run-simd-bench-and-collect-results.md) |
| TASK-H0PE3xMNpsPWNpf1zHdJtV | Issue #75 文書クローズと全量検証 | done | [ruviform-dsp-closeout.md](./ruviform-dsp-closeout.md) |
| TASK-mNC1TFjl14YiCKvImNjrB6 | ruviform-dsp クレート実装 + テスト | done | [ruviform-dsp-crate.md](./ruviform-dsp-crate.md) |
| TASK-6yBeOSA8bu0fh2nwosiwVj | 第 2 ターゲット設計の整合検証 + Issue #32 クローズ | done | [second-target-design-verify.md](./second-target-design-verify.md) |
| TASK-034JQH8VJI6x2EXHMsOJU9 | 公開カーネル API + ルーティング + panic-free 化 + 精度契約 doc | done | [simd-dispatch-api-routing.md](./simd-dispatch-api-routing.md) |
| TASK-034JQH9WjGTpvxbXgZ1sXR | Issue #53 文書クローズと全量検証 | done | [simd-dispatch-closeout.md](./simd-dispatch-closeout.md) |
| TASK-034JQH8r1Mfi5CItnm9Ebf | fearless_simd 昇格 + dsp モジュール新設（SimdToken + カーネル本体） | done | [simd-dispatch-token-and-kernels.md](./simd-dispatch-token-and-kernels.md) |
| TASK-034JQH9rC2GoOQC8WTqmpY | 本番経路の検証（forced 単体・matrix 列・RT no-alloc） | done | [simd-dispatch-verification.md](./simd-dispatch-verification.md) |
| TASK-034J6Pq9WM0XMkGqryZrvQ | フォローアップトリアージ由来の小修正（try_new・Custom 不変条件・丸め注記） | done | [small-fixes-from-followup-triage.md](./small-fixes-from-followup-triage.md) |
| TASK-034J5KRwAtAqF1xu0moYuS | Issue #18 ドキュメント同期と総合検証 | done | [sync-docs-and-verify-issue-18.md](./sync-docs-and-verify-issue-18.md) |
| TASK-034Jgi4vWCuaMUu3POps1c | Issue #20 ドキュメント同期と総合検証 | done | [sync-docs-and-verify-issue-20.md](./sync-docs-and-verify-issue-20.md) |
| TASK-034JNwUPMqjwynDi5XzU09 | ドキュメント同期と Issue #58 完了検証（AGENTS.md 型列挙・全量検証・監査） | done | [sync-docs-and-verify-issue-58.md](./sync-docs-and-verify-issue-58.md) |
| TASK-034JgsIynX51YyqUrzgppO | プラグイン基盤のドキュメント同期と総合検証 | done | [sync-docs-and-verify-plugin-foundation.md](./sync-docs-and-verify-plugin-foundation.md) |
| TASK-034JcD43sDKFAlPxYuAWmj | Issue #26 ドキュメント同期（baseline 採否表・AGENTS・README）と総合検証 | done | [sync-docs-issue-26.md](./sync-docs-issue-26.md) |
| TASK-034JxObKHc0v3kUhnQiePv | RT 基盤ドキュメント同期と総合検証 | done | [sync-rt-foundation-docs-and-verify.md](./sync-rt-foundation-docs-and-verify.md) |
| TASK-034JOul8zCtHMR49d9c78V | write_interleaved の per-chunk timestamp 合成 | done | [synthesize-per-chunk-timestamps.md](./synthesize-per-chunk-timestamps.md) |
| TASK-034Qhxbnndd80h2whxhIG4 | Issue 191 T1: MediaSink port + StagedOutput in ruviform-media-io | done | [task-191-t1-media-io-sink-staged.md](./task-191-t1-media-io-sink-staged.md) |
| TASK-034Qhxc5D3P3En9XfxxPSm | Issue 191 T2: ruviform-io-wav crate (encoder, muxer, capabilities) | done | [task-191-t2-wav-backend.md](./task-191-t2-wav-backend.md) |
| TASK-034QhxcLDQJmQx1cut9ge3 | Issue 191 T3: integration tests + verification lanes + impl record inputs | done | [task-191-t3-tests-verification.md](./task-191-t3-tests-verification.md) |
| TASK-voSb34RCLdA14Lc1kTfNlD | Issue 197 T1: Demuxer::seekable + make_decoder + DecodeMode boundary additions | done | [task-197-t1-demuxer-seekable-make-decoder.md](./task-197-t1-demuxer-seekable-make-decoder.md) |
| TASK-e5oaEOXS8FGE9PrXrLwkGi | Issue 197 T2: ruviform-media-reader crate — metadata, SourceReader, prefetch pipeline | done | [task-197-t2-media-reader-crate.md](./task-197-t2-media-reader-crate.md) |
| TASK-02vjFSak80ajcQNqOMIF0s | Issue 197 T3: RD-01..RD-10 acceptance tests + verification record | done | [task-197-t3-tests-verification.md](./task-197-t3-tests-verification.md) |
| TASK-eM0HvOCV03pTzfSqyt3jgm | Issue 198 T1: render core — OfflineRenderSupport, PreparedRender, Progress::bump_by, deps | done | [task-198-t1-render-core.md](./task-198-t1-render-core.md) |
| TASK-K54oAzr1E0TFOYRxcMd30t | Issue 198 T2: sequential driver + staged output lifecycle | done | [task-198-t2-driver-output.md](./task-198-t2-driver-output.md) |
| TASK-ubO5TxPGQDZSaLVHth3aaa | Issue 198 T3: OF-01..OF-07 acceptance tests + verification + merge + impl record | done | [task-198-t3-tests-verification.md](./task-198-t3-tests-verification.md) |
| TASK-034Qooz8UlxyXeGHTFwHXk | Issue 233 T1: prefetch contract doc rewrites — expected_first stream-position assertion + pump iteration contract | done | [task-233-t1-contract-docs.md](./task-233-t1-contract-docs.md) |
| TASK-034QoozSINn40AO2e1Ke1k | Issue 233 T2: discard surface — PrefetchConsumer::discard_pending + DiscardReport + seek_discard_pending pending purge | done | [task-233-t2-discard-pending.md](./task-233-t2-discard-pending.md) |
| TASK-034QooAu3EbKEEhgcUSJnv | Issue 233 T3: reopen_at fused reopen-to-seek-target | done | [task-233-t3-reopen-at.md](./task-233-t3-reopen-at.md) |
| TASK-034QooB7rhCVhK4sqW6lzZ | Issue 233 T4: verification sweep + implementation record | done | [task-233-t4-verify-ir.md](./task-233-t4-verify-ir.md) |
| TASK-034RoYsOdygNXYa3BanrU1 | Issue 276 T1: logging config/API types + feature wiring | done | [task-276-t1-config-and-wiring.md](./task-276-t1-config-and-wiring.md) |
| TASK-034RoYu0iSpPIN9pnjyhDU | Issue 276 T2: bounded writer + sinks + build result | done | [task-276-t2-writer-sinks-build.md](./task-276-t2-writer-sinks-build.md) |
| TASK-034RoYvGpykxcXUHa5X8VR | Issue 276 T3: contract tests + host example + docs + verification | done | [task-276-t3-tests-docs-verify.md](./task-276-t3-tests-docs-verify.md) |
| TASK-034S312gtUiTestT1GG77 | Issue 312 T1: ruviform-ui-test skeleton — proto types + journal + FakeHost + fixture/TestApp + HeadlessIo | done | [task-312-t1-contract-headless-core.md](./task-312-t1-contract-headless-core.md) |
| TASK-034S312gtUiTestT2HH88 | Issue 312 T2: control plane — TestAgent + transport frames/env + TestDriver + dummy_app bin | done | [task-312-t2-control-plane.md](./task-312-t2-control-plane.md) |
| TASK-034S312gtUiTestT3II99 | Issue 312 T3: agent gate unit tests + cross-process E2E + README/crates index | done | [task-312-t3-tests-docs.md](./task-312-t3-tests-docs.md) |
| TASK-034LFHdEfDbppnNeuh0u48 | ToneMapSdr カーネル実装（BT.2390 EETF） | done | [tone-map-sdr-kernel.md](./tone-map-sdr-kernel.md) |
| TASK-034IRZB7D249qUO1bLldy4 | AGENTS.md 暫定原則の確定契約化 | done | [update-agents-interim-principles-to-fixed-contracts.md](./update-agents-interim-principles-to-fixed-contracts.md) |
| TASK-034IwCbI6X4pBBby5Donzz | apm.yml 識別子更新と APM lockfile 再生成 | done | [update-apm-identity-and-lockfile.md](./update-apm-identity-and-lockfile.md) |
| TASK-034IRZk613ql8oL3Y2Yxli | README.md の Issue #1 ベースライン向け差分更新 | done | [update-readme-for-issue-1-baseline.md](./update-readme-for-issue-1-baseline.md) |
| TASK-034IRZFUgbTZlBX2kH7psN | Issue #1 リポジトリ反映の総合検証と spec/design バックリンク | done | [verify-issue-1-repo-reflection.md](./verify-issue-1-repo-reflection.md) |
| TASK-034OpsG2QXJ66gdCKuXWSJ | T11. video parity（stderr・cause・counter 非混入・fixture 同条件） | done | [verify-l1-video-diagnostics-parity.md](./verify-l1-video-diagnostics-parity.md) |
| TASK-034JO3XBRTT20ZNbIBDPRM | MSRV 1.89 実測検証と Issue #52 完了確認 | done | [verify-msrv-1-89-build.md](./verify-msrv-1-89-build.md) |
| TASK-034IdQxdPQUxXweHBN1ZxT | 再現インストール検証と #3 転記用作業メモ | done | [verify-reproducible-setup-and-work-memo.md](./verify-reproducible-setup-and-work-memo.md) |
| TASK-034JZVID00000000000070 | Issue #27 文書クローズと全量検証 | done | [video-engine-closeout.md](./video-engine-closeout.md) |
| TASK-034JZVID00000000000060 | ruviform-video クレート実装 + テスト | done | [video-engine-crate.md](./video-engine-crate.md) |
| TASK-034N3MMigFuy0YOIHJFklc | video-proc rt_process_* への assert_no_alloc 測定窓適用 + OFX 評価 | done | [video-proc-assert-no-alloc-lane.md](./video-proc-assert-no-alloc-lane.md) |
| TASK-034MLSxnXfu3lnbTX3LQzI | VST3 host bridge（host feature・Vst3Library/dlopen+class scan・COM host objects・lifecycle 写像・Vst3PluginFactory） | done | [vst3-host-bridge.md](./vst3-host-bridge.md) |
| TASK-034JP1sJ8ZJThe3Fj9w0mr | clack thread-check の有効化と process での is_audio_thread 照会 | done | [wire-clap-thread-check.md](./wire-clap-thread-check.md) |
| TASK-034OpsCV3XA7dtr4iHd2sg | T7. FailureCause 分類配線（first-observed・timeout-before-kill・通知伝搬） | done | [wire-failure-cause-classification.md](./wire-failure-cause-classification.md) |
| TASK-034OpshwEw34KxMBdhQ5mw | T4. audio counters 計装・query 面（InstShared・RtLeg::pass_through 内部） | done | [wire-l1-audio-counters.md](./wire-l1-audio-counters.md) |
| TASK-034LFHddlrfvXu5Jlmwdr0 | working space カーネル実装（ToWorking + FromWorking） | done | [working-space-kernels.md](./working-space-kernels.md) |
| TASK-034JPwZ8htMzYNPNgOceEI | 分析・レポート記述・文書整合 | done | [write-dispatch-analysis-report.md](./write-dispatch-analysis-report.md) |
| TASK-034Jf4g6600XCHwXaEZG26 | 採否 ADR | done | [write-simd-adoption-adr.md](./write-simd-adoption-adr.md) |
