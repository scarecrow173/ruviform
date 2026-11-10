# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/test-specs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| TSPEC-034MLRB5YDVNIaDpjhYknO | 規格アダプタ host bridge 契約（descriptor/param 合成・lifecycle 写像・defined rejection・intern-once）テスト仕様 | approved | [adapter-host-bridge-contract-tests.md](./adapter-host-bridge-contract-tests.md) |
| TSPEC-034Kq8IMaEOJAfVjWYgJwq | ruviform-audio-analysis テスト仕様 | approved | [audio-analysis-tests.md](./audio-analysis-tests.md) |
| TSPEC-034JgliZJcJ6cl2OeZLkuZ | オーディオデバイス境界ポートと cpal adapter の契約保証 | approved | [audio-device-port-and-cpal-adapter.md](./audio-device-port-and-cpal-adapter.md) |
| TSPEC-034JRQpjZtjsWuJOB1WYN0 | 音声エンジン テスト仕様 | draft | [audio-engine-tests.md](./audio-engine-tests.md) |
| TSPEC-034Jeo1j5l7fBhYc8iYsTN | ベンチ・検証タスクと CI スモーク挙動の契約保証 | approved | [bench-tasks-and-ci-smoke.md](./bench-tasks-and-ci-smoke.md) |
| TSPEC-034JOtlBoKc1LUny4Iu67z | chunk timestamp 合成の契約テスト | approved | [chunk-timestamp-contracts.md](./chunk-timestamp-contracts.md) |
| TSPEC-034IdHsItEuo0TZWTPOJUu | ci プロファイルのビルド挙動と成果物分離 | proposed | [ci-profile-build-behavior.md](./ci-profile-build-behavior.md) |
| TSPEC-034LFChh51ZlHnSWXsKqDw | video-proc 色変換・working space カーネル テスト仕様 | approved | [color-working-space-kernel-tests.md](./color-working-space-kernel-tests.md) |
| TSPEC-034KSVh4JNdOEJMzGYbZic | video-proc コンポジット/遷移カーネル テスト仕様 | approved | [composite-transition-kernels-tests.md](./composite-transition-kernels-tests.md) |
| TSPEC-034QdTKuBv9EDEBUuGxUT9 | Consumer 影響判定・証拠テンプレート必須欄（PR/Issue template） | draft | [consumer-pr-issue-template.md](./consumer-pr-issue-template.md) |
| TSPEC-034QdTJK0E2ZE5tkSjRLwr | consumer-verify workflow の gate 意味論（not run≠pass・preflight gating・単一SHA guard・秘密非露出） | draft | [consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md](./consumer-verify-workflow-gate-not-run-pass-preflight-gating-sha-guard.md) |
| TSPEC-034JZDNRML000000000005 | denormal flush テスト仕様 | approved | [denormal-flush-tests.md](./denormal-flush-tests.md) |
| TSPEC-034KPgCJjA5nOIvEtscQj8 | ruviform-edit 編集データモデル テスト仕様 | approved | [edit-data-model-tests.md](./edit-data-model-tests.md) |
| TSPEC-034J6OIFK83GJIsST60Jck | 公開 enum 拡張性と時間演算精度の契約保証 | approved | [followup-enum-compat-and-time-arithmetic.md](./followup-enum-compat-and-time-arithmetic.md) |
| TSPEC-034LFCoZvNuuSaGpSTrbp0 | IccTransform ICC 経路 契約テスト仕様 | approved | [icc-transform-contract-tests.md](./icc-transform-contract-tests.md) |
| TSPEC-034NOX3x1mfWrROBAq4Wqu | L1 supervisor core tests（Issue #116 — watch/poll state machine・backoff・GaveUp・host binding・checkpoint bound） | approved | [issue-116-supervisor-core-tests.md](./issue-116-supervisor-core-tests.md) |
| TSPEC-034NOXcdTo4m1tfmRleM0V | L1 supervisor E2E tests（Issue #116 — 実子 restart・bundle burst・crash-loop GaveUp・restore 分類・lingering） | approved | [issue-116-supervisor-e2e-tests.md](./issue-116-supervisor-e2e-tests.md) |
| TSPEC-034NlQ9qbEN4prlkfCwvdl | L1 bundle E2E tests（Issue #117 — pool lifecycle・隔離 surface・runner registry・dedicated 非回帰） | approved | [issue-117-bundle-e2e-tests.md](./issue-117-bundle-e2e-tests.md) |
| TSPEC-034NlQ8vmaETbrprsDYNkH | L1 bundle Failed 粒度 tests（Issue #117 — child/instance scope 判定表・fan-out・Destroying doorbell） | approved | [issue-117-bundle-failed-granularity-tests.md](./issue-117-bundle-failed-granularity-tests.md) |
| TSPEC-034NlQ76ZnpNmAnyT0HgY9 | L1 wire v2 codec tests（Issue #117 — instance envelope・新動詞・scope 判定表・InstanceLimit） | approved | [issue-117-wire-v2-codec-tests.md](./issue-117-wire-v2-codec-tests.md) |
| TSPEC-034NXpUzrEtLN4lNJSzI77 | L1 video E2E tests（Issue #118 — video() façade・frame 輸送・param・cross-kind・Dedicated pin） | approved | [issue-118-video-e2e-tests.md](./issue-118-video-e2e-tests.md) |
| TSPEC-034NXpU1IxxAo36yJ3zGbW | L1 video Failed parity tests（Issue #118 — 子スコープ terminal・帰属可能 per-instance・inert・timeout の video 写像） | approved | [issue-118-video-failed-parity-tests.md](./issue-118-video-failed-parity-tests.md) |
| TSPEC-034NXpKKMs4FptRr5mjttQ | L1 wire v3 video codec tests（Issue #118 — video 動詞・Reply・video 型 codec・WIRE_MAX_VIDEO_*・InvalidValue） | approved | [issue-118-wire-v3-codec-tests.md](./issue-118-wire-v3-codec-tests.md) |
| TSPEC-034OpmKTAHuW8dcRI9Rv2Y | L1 audio diagnostics counters（Issue #121 — dropped/passthrough/failed/dropped-results 計数・寿命・RT 制約） | approved | [issue-121-l1-audio-counters-tests.md](./issue-121-l1-audio-counters-tests.md) |
| TSPEC-034OpmMkivhvwJxelIXIxZ | L1 diagnostics retention・diagnostics ID（Issue #121 — bounded retention・登録時採番・found/evicted/not-found・pre-mint phase） | approved | [issue-121-l1-diagnostics-retention-tests.md](./issue-121-l1-diagnostics-retention-tests.md) |
| TSPEC-034OpmNe3vUnEprFdOIHb6 | L1 FailureCause・通知互換（Issue #121 — 7 variant 分類・first-observed・InstanceFailed.cause・wire 不変） | approved | [issue-121-l1-failure-cause-notification-tests.md](./issue-121-l1-failure-cause-notification-tests.md) |
| TSPEC-034OpmNQsPNOBT99IHbrQz | L1 diagnostics parity・無退行（Issue #121 — video parity・supervisor pass-through・L0/wire/supervisor 契約維持） | approved | [issue-121-l1-parity-nonregression-tests.md](./issue-121-l1-parity-nonregression-tests.md) |
| TSPEC-034OpmLKqqBA5YuKQkRrIU | L1 child stderr drain（Issue #121 — piped stderr・64KiB tail ring・snapshot 不変・500ms grace・heir bounded teardown） | approved | [issue-121-l1-stderr-drain-tests.md](./issue-121-l1-stderr-drain-tests.md) |
| TSPEC-034udQraULNWJKR6z8GYjq | Issue #127 Linux 実配布方式 設計文書の検証仕様 — Matrix 完全性・判定語彙・分離・Owner 境界 | proposed | [issue-127-linux-dist-design.md](./issue-127-linux-dist-design.md) |
| TSPEC-ajLVDeHdxTEsvm0JMWptbM | Issue #181 Multi-Window Host — test spec | approved | [issue-181-multi-window.md](./issue-181-multi-window.md) |
| TSPEC-79iCQBCeCxmbN386U8wYgM | Issue #182 Dock Tree / View Move / Layout Restore — test spec | approved | [issue-182-dock-tree.md](./issue-182-dock-tree.md) |
| TSPEC-034QeeLKxXQyMBzA6HNOW6 | Issue #184 M1 typed graph IR: representation, multi-bus topology, validation failures, revisions, legacy coexistence | approved | [issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md](./issue-184-m1-typed-graph-ir-representation-multi-bus-topology-validation-failure.md) |
| TSPEC-034QibxcrYUagyZX3rtGKj | Issue #185 generic probe: capability, modes, metrics, bounded capture, metadata, RT safety, faults | approved | [issue-185-generic-probe-tests.md](./issue-185-generic-probe-tests.md) |
| TSPEC-034QkFscAwaBuZWB5aEeBy | Issue #186 Parameter Control Events — test spec | approved | [issue-186-param-control-events.md](./issue-186-param-control-events.md) |
| TSPEC-034Qx4POsptkCvTP9Kdtxr | Issue #187 Input Port contract tests | approved | [issue-187-input-port-tests.md](./issue-187-input-port-tests.md) |
| TSPEC-7XFD0mdqRqdtKwGpmWUa66 | Issue #188 Timed MIDI — test spec | approved | [issue-188-timed-midi.md](./issue-188-timed-midi.md) |
| TSPEC-034QfbY0KiEUPgYkq50kJl | Issue #189 M3 Rational Beat/PPQN golden + RT tests | approved | [issue-189-m3-rational-beat-ppqn-tests.md](./issue-189-m3-rational-beat-ppqn-tests.md) |
| TSPEC-034Qh4wErTyUiOpAsD | Issue #190 Transport Span Scheduler テスト仕様 | approved | [issue-190-transport-span-scheduler-tests.md](./issue-190-transport-span-scheduler-tests.md) |
| TSPEC-034Qhwm0HeEmVIU2j6ttxT | Issue #191 PCM WAV output test specification | approved | [issue-191-pcm-wav-output-backend.md](./issue-191-pcm-wav-output-backend.md) |
| TSPEC-034UC88veX939aXOGHd2sw | Issue #192 Synvelis-driven Target Architecture & Migration — umbrella migration 設計正本 | approved | [issue-192-synvelis-driven-target-architecture-migration.md](./issue-192-synvelis-driven-target-architecture-migration.md) |
| TSPEC-034UCSrsj3GYcyfu8KYMB2 | Issue #193 Migration Execution Plan v1.0 — M0–M7・依存DAG・受入Gate・互換移行 設計正本 | approved | [issue-193-migration-execution-plan.md](./issue-193-migration-execution-plan.md) |
| TSPEC-034QfqUJnhyPPfGZsl7kXW | Issue #195 M2 typed graph compiler/runtime: schedule, buffers, PDC, ownership, activation, faults, legacy coexistence | approved | [issue-195-m2-typed-graph-compiler-runtime-tests.md](./issue-195-m2-typed-graph-compiler-runtime-tests.md) |
| TSPEC-7zOboSetinQdiV2b3AD6Yq | Issue #196 M4 Typed Graph Plugin Adapter テスト仕様 | approved | [issue-196-typed-graph-plugin-adapter-tests.md](./issue-196-typed-graph-plugin-adapter-tests.md) |
| TSPEC-JSe276zDPhy5bjOFWJMtgn | Issue #197 media reader / prefetch / RT consumer test specification | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| TSPEC-NstsDZYKTSF6CL9kQDoyXC | Issue #198 sequential offline render — OF-01..OF-07 test specification | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| TSPEC-034Qonw7qg8hdeWBuDqEqQ | Issue #233 prefetch discard/drain/reopen_at + contract-doc test specification | approved | [issue-233-media-reader-prefetch-improvements.md](./issue-233-media-reader-prefetch-improvements.md) |
| TSPEC-Jm8jpNK8kjkyBaizXBqwQo | Issue #238 テスト仕様 — l1_proxy_e2e 同期機構の不変条件 | approved | [issue-238-l1-proxy-e2e-sync.md](./issue-238-l1-proxy-e2e-sync.md) |
| TSPEC-034UdQbwnBMc4dBhcTwfxI | Issue #243 GPUI 実窓 lifecycle 実機検証 試験仕様 | in-progress | [issue-243-gpui-realwin-windows.md](./issue-243-gpui-realwin-windows.md) |
| TSPEC-034V4sgGc35MAPIRgG7lmm | Issue #243 — Windows 実 OS multiwindow smoke 検証契約 | approved | [issue-243-windows-multiwindow-smoke.md](./issue-243-windows-multiwindow-smoke.md) |
| TSPEC-034QxdOanEeX8da72nWdYo | Issue #244 prefetch boundary-first/dead-generation/End commit-once test specification | approved | [issue-244-media-reader-prefetch-warts.md](./issue-244-media-reader-prefetch-warts.md) |
| TESTSPEC-rf253osmidibackend001 | Issue #253 OS MIDI input backend test specification | approved | [issue-253-os-midi-backend.md](./issue-253-os-midi-backend.md) |
| TST-rf253vst3midicanonpath001 | Issue #253 VST3 MIDI 正規経路 slice テスト仕様 | approved | [issue-253-vst3-midi-canonical-path.md](./issue-253-vst3-midi-canonical-path.md) |
| TSPEC-034UjQC8AnQxl52Q8wCT6N | Issue #257 Driver-neutral Reusable UI Contract — 蓄積契約の正本統合 | approved | [issue-257-driver-neutral-ui-contract.md](./issue-257-driver-neutral-ui-contract.md) |
| TSPEC-sFT08b6Qp306TbonlKfL28 | Issue #275 Observability統合設計正本 — 単一クレート・OBS-01〜11・両Consumer 設計 | approved | [issue-275-observability-integration-design.md](./issue-275-observability-integration-design.md) |
| TSPEC-034RoYjtRWwL5tpZsCCp6T | Issue #276 generic logging adapter test specification | approved | [issue-276-generic-logging-adapter.md](./issue-276-generic-logging-adapter.md) |
| TSPEC-034Rp29Zp9BL0LmhcrU4IF | Issue #277 OBS-02 observability core/source contract tests | approved | [issue-277-observability-core-source-tests.md](./issue-277-observability-core-source-tests.md) |
| TSPEC-034UBsnivDah67dJlEQhYq | Issue #278 OBS-03 tracing_export slice 2 試験仕様 | approved | [issue-278-observability-tracing-export-records-tests.md](./issue-278-observability-tracing-export-records-tests.md) |
| TSPEC-034Uprl9YF50Z6pFk0lPWo | Issue #278 OBS-03 tracing_export 試験仕様 | approved | [issue-278-observability-tracing-export-tests.md](./issue-278-observability-tracing-export-tests.md) |
| TSPEC-034RTrq34tAM6KDpPaC83H | Issue #279 OBS-04 plugin-host 4 Counter Source登録・寿命・degraded試験 | approved | [issue-279-plugin-host-counter-source-tests.md](./issue-279-plugin-host-counter-source-tests.md) |
| TSPEC-034JfAS5pKzR24eQWPln8d | Issue #28 UI PoC 検証仕様 | approved | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| TSPEC-iF1sER54cUJ1FKmvvSy6W2 | Issue #280 OBS-05 RT record bridge・record vocabulary tests | approved | [issue-280-rt-record-bridge-tests.md](./issue-280-rt-record-bridge-tests.md) |
| TSPEC-034UyJjriooXu9EsvaMpgj | Issue #284 IO-02 Structured Formats — Serde境界・Format別契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-284-io-02-structured-formats-serde-boundary.md](./issue-284-io-02-structured-formats-serde-boundary.md) |
| TSPEC-icPRYUQjj1ahQHHkGcK8vZ | Issue #285 IO-03 Compression 契約試験棚卸し テスト仕様 | approved | [issue-285-io-03-compression-contract-audit.md](./issue-285-io-03-compression-contract-audit.md) |
| TSPEC-ZpVwcAYhu5UaIR5HfPUNAb | Issue #285 IO-03 Compression — Serde境界・Format/Version契約・Consumer利用面・証拠項目対応 設計 | approved | [issue-285-io-03-compression-serde-boundary.md](./issue-285-io-03-compression-serde-boundary.md) |
| TSPEC-034UAuXWuQzQh1Tt9WnR1N | Issue #286 IO-04 ZIP/TAR Archive — Entry API・安全な展開・終了/公開契約 設計 | approved | [issue-286-io-04-zip-tar-archive.md](./issue-286-io-04-zip-tar-archive.md) |
| TSPEC-034UtKOMzAChsfMRSOpIdg | Issue #288 IO-06 Cross-Consumer/OS-FS/Security/性能/互換性 Gate 設計 | approved | [issue-288-io-06-cross-consumer-os-fs-security-gate.md](./issue-288-io-06-cross-consumer-os-fs-security-gate.md) |
| TSPEC-034T2NeVNZOhwkPaIzj0Q7 | Issue #289 OBS-07 Collector/Metrics/Snapshot 試験仕様 | approved | [issue-289-observability-collector-metrics-tests.md](./issue-289-observability-collector-metrics-tests.md) |
| TSPEC-034UoYH3LGuoCYBYsd5dXf | Issue #290 allocation-probe — CountingAllocator/AllocationScope 契約試験 | approved | [issue-290-observability-allocation-probe-tests.md](./issue-290-observability-allocation-probe-tests.md) |
| TSPEC-1nETjBT2y9V4C04J9pwXer | Issue #290 OBS-08 RT Timing 試験仕様 | approved | [issue-290-observability-rt-timing-tests.md](./issue-290-observability-rt-timing-tests.md) |
| TSPEC-034UnxESRAtEpUTP5RInqW | Issue #291 OBS-09 Diagnostics/Evidence/Finding/Health/Report 試験仕様 | approved | [issue-291-observability-diagnostics-tests.md](./issue-291-observability-diagnostics-tests.md) |
| TSPEC-a7qCcQ6m0LyKTiq4IswgJo | Issue #292 OBS-10 Debug Inspection/Invariant/Session/Report 試験仕様 | approved | [issue-292-observability-debug-inspection-tests.md](./issue-292-observability-debug-inspection-tests.md) |
| TSPEC-zq1tvuEsFrUjTnAjn3csSR | Issue #293 OBS-11 ObservabilityConfig/ObservationRunner 試験仕様 | approved | [issue-293-observability-integration-runner-tests.md](./issue-293-observability-integration-runner-tests.md) |
| TSPEC-9cHJxPHaQTlIrwbuTfhkPc | Issue #294 consumer-verify → ci-issue-tracker 配線 test spec — scope 分離・identity 照合・verdict 写像 | proposed | [issue-294-consumer-verify-tracker-wiring.md](./issue-294-consumer-verify-tracker-wiring.md) |
| TSPEC-034S9zD5gJ8lN2qU6xA9eH | consumer-verify tuple証拠契約（解決SHA捕捉・期待照合・status導出・証拠JSON・workflow構造） | implemented | [issue-294-consumer-verify-tuple-evidence.md](./issue-294-consumer-verify-tuple-evidence.md) |
| TSPEC-034Ule6zLxK0at8L5q0W7q | Issue #294 daily candidate-tuple trigger test spec — tuple 判定・証拠回復・dispatch 契約 | proposed | [issue-294-daily-tuple-trigger.md](./issue-294-daily-tuple-trigger.md) |
| TSPEC-qrIvFGA7o1j1g1gLAH1gW2 | consumer-verify GUI-CONTRACT lane — 候補 tree 評価・task probe・別結果記録の構造契約 | approved | [issue-294-gui-contract-lane-wiring.md](./issue-294-gui-contract-lane-wiring.md) |
| TSPEC-034RoRhmlBaEa2BqB4gYX1 | Issue #295 ruviform-io-file Mock-driven publish 契約テスト仕様 | approved | [issue-295-io-file-staged-publish.md](./issue-295-io-file-staged-publish.md) |
| TSPEC-034RBIVDSp1VYbb5Tt3x9B | Issue #296 ruviform-io-file 実 OS backend テスト仕様 | approved | [issue-296-io-file-os-backend.md](./issue-296-io-file-os-backend.md) |
| TSPEC-034RTeRrELQYdXaCBD66Jy | Issue #297 ruviform-io-structured テスト仕様 | approved | [issue-297-io-structured-json-yaml-toml.md](./issue-297-io-structured-json-yaml-toml.md) |
| TSPEC-034Mm7FbtxkfxmQCgVk1m4 | Issue #298 ruviform-io-structured — CSV Module 試験仕様（IO-IMP-04） | approved | [issue-298-io-structured-csv.md](./issue-298-io-structured-csv.md) |
| TSPEC-034S6M7jOVNgIiAWwNm2J6 | Issue #299 ruviform-io-structured — XML Module 試験仕様（IO-IMP-05） | approved | [issue-299-io-structured-xml.md](./issue-299-io-structured-xml.md) |
| TSPEC-c6xrG6sZjlJbmoJEzBYzJG | Issue #300 ruviform-io-compression Gzip・Raw Deflate テスト仕様 | approved | [issue-300-io-compression-gzip-deflate.md](./issue-300-io-compression-gzip-deflate.md) |
| TSPEC-9sy40kbL7d1xqwhmY2sDLw | Issue #301 ruviform-io-compression Zstd Frame 試験仕様 | approved | [issue-301-io-compression-zstd.md](./issue-301-io-compression-zstd.md) |
| TSPEC-034RT7OuFi1aZ0rhvdSFEt | Issue #302 ruviform-io-archive ZIP 検証・Budget・Writer テスト仕様 | approved | [issue-302-io-archive-zip.md](./issue-302-io-archive-zip.md) |
| TSPEC-EtHC4HLHJhXDwgG6UbrccI | Issue #303 ruviform-io-archive TAR 検証・Pending・Writer テスト仕様 | approved | [issue-303-io-archive-tar.md](./issue-303-io-archive-tar.md) |
| TSPEC-pWc14Y17FV06GN2600rX8h | Issue #304 ruviform-io-archive Secure Extraction・Staging/Recovery テスト仕様 | approved | [issue-304-io-archive-extract.md](./issue-304-io-archive-extract.md) |
| TSPEC-034UvWxgBRObLuwfTjwqMx | Issue #305 IO E2E WAV Golden/Fault テスト仕様 | approved | [issue-305-io-e2e-wav-golden-fault.md](./issue-305-io-e2e-wav-golden-fault.md) |
| TSPEC-0kY8BSgqOCJHezrud22f3h | Issue #305 Media I/O StagedFile 移行 テスト仕様 | approved | [issue-305-media-io-staged-file-cutover.md](./issue-305-media-io-staged-file-cutover.md) |
| TSPEC-rviIWrpuHyJUT11Wykzopn | Issue #306 汎用 I/O 合成 E2E テスト仕様 | approved | [issue-306-io-composition-e2e.md](./issue-306-io-composition-e2e.md) |
| TSPEC-034UtBs1z6PAEjR3T4EiV2 | Issue #306 IO E2E slice 2 テスト仕様 | approved | [issue-306-io-e2e-fault-fuzz.md](./issue-306-io-e2e-fault-fuzz.md) |
| TSPEC-7h5x0wTVw48oHC6pAm1VId | Issue #306 IO E2E slice 3 テスト仕様 | approved | [issue-306-io-msrv-dag-rt-audit.md](./issue-306-io-msrv-dag-rt-audit.md) |
| TSPEC-XQ6Q9VkNnBlPRcMf4cqLFu | Issue #307 attach 時 frame_base 検証 context — テスト仕様 | approved | [issue-307-attach-frame-base-verified-context.md](./issue-307-attach-frame-base-verified-context.md) |
| TSPEC-034Ui3L8Aap8bdwyrLbDGp | Issue #308 — Probe再Bindingの検証契約 | approved | [issue-308-probe-rebind-atomic.md](./issue-308-probe-rebind-atomic.md) |
| TSPEC-034UwjAIH8nOcoNlsyjent | Issue #309 PROBE-TIME slice — アライメント fixture 検証契約 | approved | [issue-309-probe-time-align-fixture.md](./issue-309-probe-time-align-fixture.md) |
| TSPEC-034Uja9CLADV2KHCRHEtFS | Issue #309 — Probe時間整列・欠落比較の検証契約 | approved | [issue-309-probe-time-alignment.md](./issue-309-probe-time-alignment.md) |
| TSPEC-034Rp62M5XwHUCClhVcEPk | Issue #310 Browser GUI Test Harness 検証仕様 | approved | [issue-310-browser-gui-test-harness.md](./issue-310-browser-gui-test-harness.md) |
| TSPEC-e0XfO8Zf3a26iGOk1UyquR | Issue #311 RF-GT-02 test specification — fixture conformance・record/replay determinism・fault matrix | approved | [issue-311-ipc-conformance-replay.md](./issue-311-ipc-conformance-replay.md) |
| TSPEC-034S312gtUiTestTspFF66 | Issue #312 ruviform-ui-test — lifecycle/control/fixture/clock/observation/fail-closed test specification | approved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
| TSPEC-034UkPTR1W1N5M0K3P03 | Issue #313 — Windows 実 OS SendInput ポインタ smoke 検証契約 | approved | [issue-313-windows-pointer-smoke.md](./issue-313-windows-pointer-smoke.md) |
| TSPEC-034UjMKOaa4DzlRpbwWOuL | Issue #313 — Windows 実 OS SendInput キーボード smoke 検証契約 | approved | [issue-313-windows-realos-smoke.md](./issue-313-windows-realos-smoke.md) |
| TSPEC-034Ug943CVpTG4Br56eTfE | Issue #314 slice 3 — GPUI/OS capture backend test spec | approved | [issue-314-golden-capture.md](./issue-314-golden-capture.md) |
| TSPEC-e1fQqoSXueHMyLvDmFV5Gz | Issue #314 slice 1 — Golden Utility コアのテスト仕様 | proposed | [issue-314-golden-core.md](./issue-314-golden-core.md) |
| TSPEC-suZHG6N60Cpt2NxAwyqq2G | Issue #314 slice 5 — Golden 環境正規化・registry 結合・runner 統合・証拠 lane のテスト仕様 | approved | [issue-314-golden-evidence.md](./issue-314-golden-evidence.md) |
| TSPEC-034UeyabB9HNuQvfyPVYdA | Issue #314 slice 2 — Golden × Scenario 能力契約接続のテスト仕様 | proposed | [issue-314-golden-scenario.md](./issue-314-golden-scenario.md) |
| TEST-314rwEt8pC4wHxJ7nQ3rS | Issue #314 slice 4 residual — real-window adapter test-spec | approved | [issue-314-real-window-adapter.md](./issue-314-real-window-adapter.md) |
| TSPEC-034V2hpT0kkEMk2hJtieRh | Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures のテスト仕様 | approved | [issue-314-semantic-oracle.md](./issue-314-semantic-oracle.md) |
| TSPEC-nB4WC4Q4CYMJmm6HVksOwr | Issue #326 ci.yml dispatch 専用化 test specification — trigger 不存在契約・dispatch 契約維持・PR 起動ゼロ実証 | approved | [issue-326-ci-manual-only.md](./issue-326-ci-manual-only.md) |
| TSPEC-034RBMKdNX5HX6vExF2WlS | Issue #327 CI計測Baseline 検証仕様 | approved | [issue-327-cross-repo-ci-baseline.md](./issue-327-cross-repo-ci-baseline.md) |
| TSPEC-oHAlZzKO4OjFLCg0SpIiws | Issue #328 テスト仕様 — CI concurrency の不変条件と証拠 | approved | [issue-328-ci-pr-concurrency.md](./issue-328-ci-pr-concurrency.md) |
| TSPEC-8FPRKgybhQfr9n7cWGNJoz | Issue #329 テスト仕様 — sccache 導入の不変条件・ケース・証拠 | approved | [issue-329-sccache-compile-cache.md](./issue-329-sccache-compile-cache.md) |
| TSPEC-034RBIMbJjTnaygGjWGrGx | Issue #330 CI-OPT-03 test specification — install 重複排除・dist 依存順・互換性 | approved | [issue-330-ui-npm-ci-dedup.md](./issue-330-ui-npm-ci-dedup.md) |
| TSPEC-034RJ01zO4VjU2gWOLQXgi | Issue #331 テスト仕様 — check/ci 統合の不変条件・注入実験・実測 | approved | [issue-331-check-clippy-test-dedup.md](./issue-331-check-clippy-test-dedup.md) |
| TSPEC-EXjezAr9qVRhcJq18LPw8U | Issue #332 CI-OPT-05 test specification — browser cache・fallback・分離計測 | approved | [issue-332-playwright-browser-cache.md](./issue-332-playwright-browser-cache.md) |
| TSPEC-zbT71fmv3NCZBzB3xof25n | Issue #333 CI-OPT-06 test specification — classifier 分類契約・gate 判定規則 | approved | [issue-333-impact-based-ci.md](./issue-333-impact-based-ci.md) |
| TSPEC-034SmE0Mm6CdyxKuBCPNAW | Issue #360 テスト仕様 — post-mint exit 順序不変条件 | approved | [issue-360-heir-exit-post-mint-ordering.md](./issue-360-heir-exit-post-mint-ordering.md) |
| TSPEC-GfyGGakuj069mKFQrVtF3e | Issue #367 テスト仕様 — daily scheduler 判定・dispatch 確認・fail-closed 契約 | approved | [issue-367-daily-scheduler.md](./issue-367-daily-scheduler.md) |
| TSPEC-3iZ6nd0UGy5yuVGAnGQI1y | Issue #368 CI failure Issue Tracker test specification — 有効 run 判定・plan 行列・close guard・sanitize | approved | [issue-368-ci-issue-tracker.md](./issue-368-ci-issue-tracker.md) |
| TSPEC-qvpqeonK5xwqk0hZpF3ARO | Issue #368 Tracker reconcile test specification — mode 分離・bot 履歴 reconcile・fail-closed 維持 | approved | [issue-368-tracker-reconcile.md](./issue-368-tracker-reconcile.md) |
| TSPEC-qAlLq79jiKf7NgITUKo6as | Issue #377 QueryBinding adopt-on-join テスト仕様 | approved | [issue-377-query-binding-epoch.md](./issue-377-query-binding-epoch.md) |
| TSPEC-034UhmzL4r0lLg4T5tUy74 | Issue #378 oracle late-create binding epoch テスト仕様 | approved | [issue-378-oracle-late-create-epoch.md](./issue-378-oracle-late-create-epoch.md) |
| TSPEC-7kQ2mWcR9dLx4VfT1hZa8N | Issue #385 — DTCG 解釈器・ResolvedTheme の golden / 分類エラー検証契約 | approved | [issue-385-dtcg-resolved-theme.md](./issue-385-dtcg-resolved-theme.md) |
| TESTSPEC-npkv0TXtBSoMiqPQl8ClVv | Issue #386 Driver-neutral GUI Host 接続試験仕様 | proposed | [issue-386-driver-neutral-gui-host.md](./issue-386-driver-neutral-gui-host.md) |
| TESTSPEC-034T6RWTu4Bx7Ng3hV0dK2 | Issue #387 Component/View + Headless Behavior + GPUI Renderer 試験仕様 | proposed | [issue-387-component-renderer.md](./issue-387-component-renderer.md) |
| TSPEC-0f0PqMFNljVvAz5wOtJEQu | Issue #388/#389 Native Dock 契約テスト仕様（slot/lease/session の headless 検証 + L0-L1 写像） | approved | [issue-388-389-native-dock-contract.md](./issue-388-389-native-dock-contract.md) |
| TSPEC-rf388DockLayoutPoc01 | Issue #388 GPUI Panel/Tab レイアウト内 Editor 埋め込みの実機テスト仕様（Windows / VST3 / L0） | approved | [issue-388-native-dock-layout-poc.md](./issue-388-native-dock-layout-poc.md) |
| TSPEC-rf388DockWindowing01 | Issue #388 slice 3 windowing 挙動の実機テスト仕様（Windows / VST3 / L0） | approved | [issue-388-native-dock-windowing-slice3.md](./issue-388-native-dock-windowing-slice3.md) |
| TSPEC-Vqv91J1zDZ6Qvi57khd2TT | Issue #389 Plugin Editor Session headless fixture テスト仕様（DSP/RT 非干渉 + contract edge + focus 境界） | approved | [issue-389-editor-session-headless.md](./issue-389-editor-session-headless.md) |
| TSPEC-Q5Bwy4OQ5KsQyE0Z4uhWg6 | Issue #389 Plugin Editor Session fake host driver 状態機械 fixture テスト仕様（terminal-event・stale・timeout・degrade・crash） | approved | [issue-389-editor-session-statemachine.md](./issue-389-editor-session-statemachine.md) |
| TSPEC-034UnAjuxNxMXzDMasVYeO | Issue #389 Plugin Editor Session hardening テスト仕様（fake-clock timeout・degrade・GraphicsLost contract） | approved | [issue-389-session-hardening.md](./issue-389-session-hardening.md) |
| TESTSPEC-bnIEkLgoxp7oIJxODUlJnj | Issue #389 VST3/L0 実外部 Editor attach テスト仕様（adapter headless 契約 + 実 binary manual lane） | approved | [issue-389-vst3-l0-editor-attach.md](./issue-389-vst3-l0-editor-attach.md) |
| TSPEC-034TaI7Luee0T88mH1IYv2 | Issue #390 Driver Boundary/Lab 検証 | draft | [issue-390-driver-boundary-lab.md](./issue-390-driver-boundary-lab.md) |
| TSPEC-UiEoMhS5QMTr3jBiurbTtC | Issue #400 借用 readback → demuxer bounded 接続の test 仕様 | approved | [issue-400-borrowed-readback-decoder-bridge.md](./issue-400-borrowed-readback-decoder-bridge.md) |
| TESTSPEC-XADte5w8EHAalnm33WCVyn | Issue #404/#405 NativeSurfaceSlot host 境界修復 test 仕様 | draft | [issue-404-405-native-surface-host-boundary.md](./issue-404-405-native-surface-host-boundary.md) |
| TSPEC-034TnnBwjsmwufsaEZZ0RG | Issue #409 scoped re-root + bounded enumeration テスト仕様 | approved | [issue-409-scoped-enumeration.md](./issue-409-scoped-enumeration.md) |
| TSPEC-034Uwdiu2k3CqklLUCSLe0 | Issue #410 P2 — view ledger retention & correlation floor guarantees | approved | [issue-410-p2-view-ledger-correlation-hardening.md](./issue-410-p2-view-ledger-correlation-hardening.md) |
| TSPEC-034UmKFzwIT1TdxeUWwDrp | Issue #410 — view command ledger retention & correlation guarantees | approved | [issue-410-view-command-ledger-retention-correlation-guarantees.md](./issue-410-view-command-ledger-retention-correlation-guarantees.md) |
| TSPEC-CeWNpGq9NFmtzCCp3dkysw | Issue #411 — cargo_metadata MSRV 修復の検証契約 | approved | [issue-411-msrv-cargo-metadata-pin.md](./issue-411-msrv-cargo-metadata-pin.md) |
| TSPEC-bN4yZaDfS6xOvJpM8qTu5H | Issue #414 — projection→controlled display の検証契約 | approved | [issue-414-projection-controlled-display.md](./issue-414-projection-controlled-display.md) |
| TSPEC-034UohaTnl4VwJLJCSfXgJ | Issue #418 — theme-codegen の検証契約 | approved | [issue-418-theme-codegen.md](./issue-418-theme-codegen.md) |
| TSPEC-Pw8DcTn5KgBy2VaQr7ZmFu | Issue #421 — Meter/TimeAxis GPUI 描画と Fake/GPUI parity の検証契約 | proposed | [issue-421-meter-timeaxis-gpui-parity.md](./issue-421-meter-timeaxis-gpui-parity.md) |
| TSPEC-Vm4QrJc7LsBk2NwYg9DeHa | Issue #429 — Consumer 観測 seam の検証契約 | proposed | [issue-429-consumer-observation-seam.md](./issue-429-consumer-observation-seam.md) |
| TSPEC-Bw6Qk3RnZtLp9VcSj4MdGa | Issue #434 — Spectrogram kind の検証契約 | proposed | [issue-434-spectrogram-kind.md](./issue-434-spectrogram-kind.md) |
| TSPEC-Mv4HxFc7BqKe1NwRj9TsZa | Issue #435 — Label/Table の契約・Fake/GPUI parity の検証契約 | approved | [issue-435-label-table-kinds.md](./issue-435-label-table-kinds.md) |
| TEST-Cz6GhM2TwPq9RkLd4VnXa7 | Issue #436 — 描画 plan 公開のテスト仕様 | proposed | [issue-436-observer-drawn.md](./issue-436-observer-drawn.md) |
| TSPEC-Ek8Nr3Gu6LvBx4Cq9TjMaZ | Issue #442 — 描画 plan への Spectrogram / Label / Table 追加のテスト仕様 | approved | [issue-442-observer-drawn-spectrogram-label-table.md](./issue-442-observer-drawn-spectrogram-label-table.md) |
| TSPEC-Nd4Gq8VzMr2TkLb6XjPw1E | Issue #443 — Tile 列区間と count の検証契約 | approved | [issue-443-tile-column-contract.md](./issue-443-tile-column-contract.md) |
| TSPEC-Qm7Lw3Rt9Xc2Hb5Vd8Ns4Ke | Issue #446 — 空 frame 範囲の Tile 検証契約 | proposed | [issue-446-tile-empty-frames.md](./issue-446-tile-empty-frames.md) |
| TSPEC-Bn6Xc1RqLw3KtYv8ZmHd2G | Issue #448 — 行選択と command 起動の検証契約 | approved | [issue-448-table-selection-command.md](./issue-448-table-selection-command.md) |
| TSPEC-EcUAfWYYG6G0Npqen48ITk | Issue #450 — Seek/Fader canvas bounds の検証契約 | approved | [issue-450-control-canvas-size.md](./issue-450-control-canvas-size.md) |
| TSPEC-034Ue3Xudk0zUKfypVisTg | Issue #451 FU-1b — Tab/Shift-Tab フォーカス巡回のテスト仕様 | approved | [issue-451-focus-next-wiring.md](./issue-451-focus-next-wiring.md) |
| TSPEC-Yn5Gk3Rt9MvQc2BxLw7DeJ | Issue #451 FU-1 — Tabs のキーボード切替の検証契約 | approved | [issue-451-keyboard-tab-nav.md](./issue-451-keyboard-tab-nav.md) |
| TSPEC-Js8Lv3XqRb6NwKt1YcMg5F | Issue #451 — Tabs pointer 切替と descriptor 更新の検証契約 | approved | [issue-451-tabs-pointer-descriptor.md](./issue-451-tabs-pointer-descriptor.md) |
| TSPEC-WsywxRxhFuHBznTRP361Ie | Issue #453 — paint canvas bounds の検証契約（kind 別監査） | approved | [issue-453-canvas-size-audit.md](./issue-453-canvas-size-audit.md) |
| TSPEC-Vc4Dk1PhGj6WsQy9NrTm5A | Issue #456 — job 終端 receipt の検証契約 | approved | [issue-456-job-command-receipt.md](./issue-456-job-command-receipt.md) |
| TSPEC-034TXoqpr4Ij3WOJxlAjic | Issue #461 — canvas painter paint batch の検証契約 | approved | [issue-461-canvas-paint-layer.md](./issue-461-canvas-paint-layer.md) |
| TSPEC-034U1MLtHiia8IiRjMgkW9 | Issue #461 残課題 — TimeAxis label canvas 描画の検証契約 | approved | [issue-461-timeaxis-label-cost.md](./issue-461-timeaxis-label-cost.md) |
| TSPEC-034U0AQAAk59m51vJU5F4G | Issue #463 — Spectrogram cell intensity の paint 色検証契約 | approved | [issue-463-spectrogram-intensity.md](./issue-463-spectrogram-intensity.md) |
| TSPEC-034UcapDuO3SYzYw2r2tHz | Issue #464 — Consumer-neutral GUI Scenario Contract | approved | [issue-464-consumer-neutral-gui-scenario-contract.md](./issue-464-consumer-neutral-gui-scenario-contract.md) |
| TSPEC-wOxWNRLHg5C2rotQihPFys | Issue #464 slice — Scenario Runner・Report 組み立て契約のテスト仕様 | approved | [issue-464-scenario-runner.md](./issue-464-scenario-runner.md) |
| TSPEC-034UewQGei6BGVolWZlKjI | Issue #478 Fader/Knob 縦寸法契約の検証 | approved | [issue-478-fader-knob-height-contract.md](./issue-478-fader-knob-height-contract.md) |
| TSPEC-LqfidO2iIqcMjOeeHeLMd2 | Issue #479 — 値制御キーボード操作の検証契約 | approved | [issue-479-keyboard-value-controls.md](./issue-479-keyboard-value-controls.md) |
| TSPEC-034UiRtw5zDZciI3peq94F | Issue #480 tab_focus 解放テスト仕様 | approved | [issue-480-tab-focus-release.md](./issue-480-tab-focus-release.md) |
| TSPEC-hojj4XgrGAgMW3wK6Tr5Ok | Issue #490 oracle late-create query epoch テスト仕様 | approved | [issue-490-oracle-addquery-epoch.md](./issue-490-oracle-addquery-epoch.md) |
| TSPEC-034VedyL5Ej4Yc3P62YgUl | Issue #552 — SendInput preflight 検証契約 | approved | [issue-552-sendinput-preflight.md](./issue-552-sendinput-preflight.md) |
| TSPEC-034MLRCbJ8DILg5vtGVsQB | 実プラグイン L1 opt-in lane（mise task・`#[ignore]` ゲート・fixture 欠落 fail・CI `l1-real-verify` job）テスト仕様 | approved | [l1-real-plugin-lane-tests.md](./l1-real-plugin-lane-tests.md) |
| TSPEC-034IdHmKPCUPXNnJh1Y7f7 | lockfile による再現セットアップ | proposed | [lockfile-reproducible-setup.md](./lockfile-reproducible-setup.md) |
| TSPEC-034IdHt7P0ega4N0bbNANg | 手動ディスパッチ専用の CI ワークフロー | proposed | [manual-ci-workflow.md](./manual-ci-workflow.md) |
| TSPEC-034J5K0lC6EBv1PrsliYd2 | メディアデータモデル API の契約保証 | approved | [media-data-model-api.md](./media-data-model-api.md) |
| TSPEC-034Jg7SJdGz3QogvORe8QT | メディア I/O 抽象境界とバックエンド PoC の契約保証 | approved | [media-io-abstraction-and-backend-poc.md](./media-io-abstraction-and-backend-poc.md) |
| TSPEC-034IdHshzpNiPTqyyvwL6J | mise タスクによるローカル・CI 共通検証 | proposed | [mise-verification-tasks.md](./mise-verification-tasks.md) |
| TSPEC-034KoNprsmdPREx6TtcMB6 | 音楽時間モデル（media-core::music）テスト仕様 | approved | [music-time-media-core-tests.md](./music-time-media-core-tests.md) |
| TSPEC-034KpKr9gVqJJNxYSJVaQF | 音楽時間 ABI 拡張テスト仕様 | approved | [musical-transport-abi-tests.md](./musical-transport-abi-tests.md) |
| TSPEC-034JRcGrYYZnirStTwM7Z9 | オフライン実行ドライバ テスト仕様 | draft | [offline-execution-driver-tests.md](./offline-execution-driver-tests.md) |
| TSPEC-034L6w60ZWfGC89xBPgw4L | OFX Host Native Contract Tests | proposed | [ofx-host-native-contract-tests.md](./ofx-host-native-contract-tests.md) |
| TSPEC-034MrsawZsMkT7wzC5cbuK | plugin endpoint 契約（prepare/bounds・Shared セル・arm 経路・leg 返却・Send 証拠）テスト仕様 | approved | [plugin-endpoint-contract-tests.md](./plugin-endpoint-contract-tests.md) |
| TSPEC-034Jg6YSP6CLjJHnFGG5oL | プラグイン基盤 API の契約保証 | approved | [plugin-foundation.md](./plugin-foundation.md) |
| TSPEC-034MrshMb3JSLSRiyAbWcK | plugin グラフ E2E（L0/L1 endpoint → FnMut AudioNode → pull グラフ → mock device）テスト仕様 | approved | [plugin-graph-e2e-tests.md](./plugin-graph-e2e-tests.md) |
| TSPEC-034JZPGUI0000000000004 | プラグイン GUI PoC 判定規則 | approved | [plugin-gui-poc.md](./plugin-gui-poc.md) |
| TSPEC-5ibPChaXqhbfYjL713aqKw | plugin-host L0 テスト仕様 | approved | [plugin-host-l0-tests.md](./plugin-host-l0-tests.md) |
| TSPEC-034LNXHpBkMIVTg6nAQUVh | plugin L1 プロキシ契約（コマンド parity・クラッシュ Failed・pass-through・RT 脚）テスト仕様 | approved | [plugin-l1-proxy-contract-tests.md](./plugin-l1-proxy-contract-tests.md) |
| TSPEC-034LNXHUv8qjXSdWNUU2Ps | plugin L1 runner（ruviform-plugin-export）E2E テスト仕様 | approved | [plugin-l1-runner-e2e-tests.md](./plugin-l1-runner-e2e-tests.md) |
| TSPEC-034LNXxXa2vencBWSvgemD | plugin L1 ワイヤ codec・ライフサイクル Failed 拡張 テスト仕様 | approved | [plugin-l1-wire-codec-tests.md](./plugin-l1-wire-codec-tests.md) |
| TSPEC-034Mrsijk0b3tEOMrMG0MN | plugin/audio RT 契約 + 退役（assert_no_alloc・arm_apply・leg 返却・plan 退役 drain）テスト仕様 | approved | [plugin-rt-retirement-tests.md](./plugin-rt-retirement-tests.md) |
| TSPEC-034MLRBDZVBiSYnXZ5dTrr | 実規格アダプタ L1 E2E（spawn/handshake/lifecycle/shm block/state defined 応答/destroy + 異常系継承）テスト仕様 | approved | [real-adapter-l1-e2e-tests.md](./real-adapter-l1-e2e-tests.md) |
| TSPEC-wJA20tmm9AN7JyNH6uMDQ6 | ruviform-io-file Rooted Read Capability テスト仕様 | approved | [rooted-read-capability.md](./rooted-read-capability.md) |
| TSPEC-034JxNwt3ReMphshEaeFVe | SPSC・退役・スナップショット交換の RT 契約保証 | approved | [rt-channel-and-snapshot-contracts.md](./rt-channel-and-snapshot-contracts.md) |
| TSPEC-034JxNxcYxa29rPklpknWh | RtContext・Processor 契約・MasterClock の RT 契約保証 | approved | [rt-context-and-processor-contract.md](./rt-context-and-processor-contract.md) |
| TSPEC-034KV6hs4k3mZ6jVBgDaKf | RT 観測コスト計測 PoC ハーネスの計測保証 | approved | [rt-observability-poc-tests.md](./rt-observability-poc-tests.md) |
| TSPEC-034JP1iphLdAZkyRbPkol1 | RT 検証レーン契約 | approved | [rt-verification-contracts.md](./rt-verification-contracts.md) |
| TSPEC-Sn3LD1xCxNBxTN3FHLJUFa | ruviform-dsp テスト仕様 | approved | [ruviform-dsp-tests.md](./ruviform-dsp-tests.md) |
| TSPEC-034JQFftbYwM1JhPvmCvI1 | SIMD dispatch 層の等価性・RT 検証 | approved | [simd-dispatch-layer-tests.md](./simd-dispatch-layer-tests.md) |
| TSPEC-034JenWCuRngWvxWm00Ls3 | SIMD 等価性 matrix の契約保証 | approved | [simd-equivalence-matrix-contract.md](./simd-equivalence-matrix-contract.md) |
| TSPEC-034JxNxYK4O6s8NA8gnaah | スレッド優先度ライフサイクルの契約保証 | approved | [thread-priority-lifecycle.md](./thread-priority-lifecycle.md) |
| TSPEC-034JNskaqcN34Ki4layRNK | FrameRate 型・Timecode シグネチャ・時間変換精度の契約保証 | approved | [time-type-extension-contracts.md](./time-type-extension-contracts.md) |
| TSPEC-034JZVID00000000000050 | 動画エンジン テスト仕様 | approved | [video-engine-tests.md](./video-engine-tests.md) |
| TSPEC-034JcCwISRw7E9Q8XoNvD8 | 動画基盤 API の契約保証（所有権・RT・色変換カーネル） | approved | [video-foundation-api.md](./video-foundation-api.md) |
| TSPEC-034L6w5zHELr4fPxPNpBsk | Video Plugin Plain Contract Tests | proposed | [video-plugin-plain-contract-tests.md](./video-plugin-plain-contract-tests.md) |
| TSPEC-42XLbuUif4jmek9sQGVA3S | ViewLink authoritative generation テスト仕様 | approved | [viewlink-authoritative-generation-tests.md](./viewlink-authoritative-generation-tests.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| TSPEC-034WvK9mNqR2sTxYcAbEfG | consumer-impact-check workflow の PR 分類機械検査（記入検査・unknown 解消先・draft 除外） | superseded | [consumer-impact-check-pr-classification.md](./consumer-impact-check-pr-classification.md) |
