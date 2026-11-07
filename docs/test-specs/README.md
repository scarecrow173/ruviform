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
| TSPEC-034QfqUJnhyPPfGZsl7kXW | Issue #195 M2 typed graph compiler/runtime: schedule, buffers, PDC, ownership, activation, faults, legacy coexistence | approved | [issue-195-m2-typed-graph-compiler-runtime-tests.md](./issue-195-m2-typed-graph-compiler-runtime-tests.md) |
| TSPEC-7zOboSetinQdiV2b3AD6Yq | Issue #196 M4 Typed Graph Plugin Adapter テスト仕様 | approved | [issue-196-typed-graph-plugin-adapter-tests.md](./issue-196-typed-graph-plugin-adapter-tests.md) |
| TSPEC-JSe276zDPhy5bjOFWJMtgn | Issue #197 media reader / prefetch / RT consumer test specification | approved | [issue-197-media-reader-prefetch-rt-consumer.md](./issue-197-media-reader-prefetch-rt-consumer.md) |
| TSPEC-NstsDZYKTSF6CL9kQDoyXC | Issue #198 sequential offline render — OF-01..OF-07 test specification | approved | [issue-198-sequential-offline-render.md](./issue-198-sequential-offline-render.md) |
| TSPEC-034Qonw7qg8hdeWBuDqEqQ | Issue #233 prefetch discard/drain/reopen_at + contract-doc test specification | approved | [issue-233-media-reader-prefetch-improvements.md](./issue-233-media-reader-prefetch-improvements.md) |
| TSPEC-Jm8jpNK8kjkyBaizXBqwQo | Issue #238 テスト仕様 — l1_proxy_e2e 同期機構の不変条件 | approved | [issue-238-l1-proxy-e2e-sync.md](./issue-238-l1-proxy-e2e-sync.md) |
| TSPEC-034QxdOanEeX8da72nWdYo | Issue #244 prefetch boundary-first/dead-generation/End commit-once test specification | approved | [issue-244-media-reader-prefetch-warts.md](./issue-244-media-reader-prefetch-warts.md) |
| TSPEC-034RoYjtRWwL5tpZsCCp6T | Issue #276 generic logging adapter test specification | approved | [issue-276-generic-logging-adapter.md](./issue-276-generic-logging-adapter.md) |
| TSPEC-034Rp29Zp9BL0LmhcrU4IF | Issue #277 OBS-02 observability core/source contract tests | approved | [issue-277-observability-core-source-tests.md](./issue-277-observability-core-source-tests.md) |
| TSPEC-034RTrq34tAM6KDpPaC83H | Issue #279 OBS-04 plugin-host 4 Counter Source登録・寿命・degraded試験 | approved | [issue-279-plugin-host-counter-source-tests.md](./issue-279-plugin-host-counter-source-tests.md) |
| TSPEC-034JfAS5pKzR24eQWPln8d | Issue #28 UI PoC 検証仕様 | approved | [issue-28-ui-poc.md](./issue-28-ui-poc.md) |
| TSPEC-iF1sER54cUJ1FKmvvSy6W2 | Issue #280 OBS-05 RT record bridge・record vocabulary tests | approved | [issue-280-rt-record-bridge-tests.md](./issue-280-rt-record-bridge-tests.md) |
| TSPEC-034T2NeVNZOhwkPaIzj0Q7 | Issue #289 OBS-07 Collector/Metrics/Snapshot 試験仕様 | approved | [issue-289-observability-collector-metrics-tests.md](./issue-289-observability-collector-metrics-tests.md) |
| TSPEC-034S9zD5gJ8lN2qU6xA9eH | consumer-verify tuple証拠契約（解決SHA捕捉・期待照合・status導出・証拠JSON・workflow構造） | implemented | [issue-294-consumer-verify-tuple-evidence.md](./issue-294-consumer-verify-tuple-evidence.md) |
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
| TSPEC-rviIWrpuHyJUT11Wykzopn | Issue #306 汎用 I/O 合成 E2E テスト仕様 | approved | [issue-306-io-composition-e2e.md](./issue-306-io-composition-e2e.md) |
| TSPEC-XQ6Q9VkNnBlPRcMf4cqLFu | Issue #307 attach 時 frame_base 検証 context — テスト仕様 | approved | [issue-307-attach-frame-base-verified-context.md](./issue-307-attach-frame-base-verified-context.md) |
| TSPEC-034Rp62M5XwHUCClhVcEPk | Issue #310 Browser GUI Test Harness 検証仕様 | approved | [issue-310-browser-gui-test-harness.md](./issue-310-browser-gui-test-harness.md) |
| TSPEC-e0XfO8Zf3a26iGOk1UyquR | Issue #311 RF-GT-02 test specification — fixture conformance・record/replay determinism・fault matrix | approved | [issue-311-ipc-conformance-replay.md](./issue-311-ipc-conformance-replay.md) |
| TSPEC-034S312gtUiTestTspFF66 | Issue #312 ruviform-ui-test — lifecycle/control/fixture/clock/observation/fail-closed test specification | approved | [issue-312-test-gui-host-lifecycle.md](./issue-312-test-gui-host-lifecycle.md) |
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
| TESTSPEC-npkv0TXtBSoMiqPQl8ClVv | Issue #386 Driver-neutral GUI Host 接続試験仕様 | proposed | [issue-386-driver-neutral-gui-host.md](./issue-386-driver-neutral-gui-host.md) |
| TSPEC-0f0PqMFNljVvAz5wOtJEQu | Issue #388/#389 Native Dock 契約テスト仕様（slot/lease/session の headless 検証 + L0-L1 写像） | approved | [issue-388-389-native-dock-contract.md](./issue-388-389-native-dock-contract.md) |
| TESTSPEC-bnIEkLgoxp7oIJxODUlJnj | Issue #389 VST3/L0 実外部 Editor attach テスト仕様（adapter headless 契約 + 実 binary manual lane） | approved | [issue-389-vst3-l0-editor-attach.md](./issue-389-vst3-l0-editor-attach.md) |
| TSPEC-034TaI7Luee0T88mH1IYv2 | Issue #390 Driver Boundary/Lab 検証 | draft | [issue-390-driver-boundary-lab.md](./issue-390-driver-boundary-lab.md) |
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
