---
id: "IMPL-732clapLifecycleAudit07"
type: "impl"
status: "completed"
title: "Issue #732 production CLAP adapter lifecycle 監査 — 実装記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/tasks/issue-732-clap-lifecycle-audit.md"
    - "docs/plans/issue-732-clap-lifecycle-audit.md"
    - "docs/specs/issue-732-clap-lifecycle-audit.md"
    - "docs/designs/issue-732-clap-lifecycle-audit.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "ci/plugin-abi-validator/run.mjs"
      - type: "file"
        path: "ci/plugin-abi-validator/tests/run.test.mjs"
      - type: "file"
        path: "evidence/rf732/lifecycle-audit.md"
      - type: "file"
        path: "evidence/rf732/abi-validator/run.json"
      - type: "file"
        path: "evidence/rf732/abi-validator/clap-production_gain.txt"
      - type: "file"
        path: "evidence/rf732/abi-validator-red/run.json"
      - type: "file"
        path: "evidence/rf732/abi-validator-red/clap-production_gain.txt"
      - type: "file"
        path: "docs/impl/ir/issue-732-clap-lifecycle-audit.md"
    modified: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #732 実装記録

## 実施内容

### 監査（AC1・AC2）

rf#722 の 3 パターン（terminalization・受理 phase 過狭・bus info
自己整合）を CLAP 契約（`clap/plugin.h`）・clack 0.2.0 binding・
共有中立機械（`lifecycle.rs`）・rf#630 fixture の実績契約に写像して
照合。詳細な判定表は `evidence/rf732/lifecycle-audit.md`。

**発見した欠陥 2 件（いずれも受理層の過剰拒否/不正 metadata —
clap-validator 0.4.1 の production leg 実走で検出）**:

1. **D1 — params `module: b"/"`**（`plugin.rs` `get_info`）:
   leading `/` の module path は clap-validator が拒否
   （param 系 9 case 失敗）。fixture は全 plugin `module: b""`。
   wire metadata を `gain_param_info()`（`plugin.rs`:262）に集約し
   `module: b""` に修正。`ParamInfoWriter` が unsafe 構築で直接
   test 不能な制約を、metadata 生成の分離で解消し
   `param_info_reports_validator_clean_metadata` で pin。
2. **D2 — fractional sample rate 拒否**（`adapt.rs::to_process_config`）:
   `rate.fract() != 0.0` を defined reject に含め、1234.57 Hz で
   `activate` false（`process-varying-sample-rates` 失敗）。
   fixture は `rate.round()` で整数 Hz に丸めて受理。
   fract 条件を除去し `rate.round() as u32` に変更（fixture
   `block.rs::to_process_config` と同一写像）。残りの reject 条件
   （非有限・非正・u32 超過・max_frames=0・min>max）は不変。

**欠陥なしと判定した経路**: terminalization（CLAP `destroy` は
instance 終端 — VST3 `terminate` との非対称は契約と整合）、
lifecycle 遷移の受理 state 集合、bus/port 宣言（stereo pair・
IS_MAIN・in_place_pair・f32-only）、latency/tail 非宣言の妥当性、
`map_process_result` の status 写像、`reset` の無遷移設計、
error channel なし callback の fail-closed 記録。

`crates/ruviform-plugin-core` は無変更（条件付き lease 不使用 —
共有機械自体の欠陥は発見されなかった）。

### validator lane 拡張（AC3）

`ci/plugin-abi-validator/run.mjs` に production CLAP leg を追加:

- `--clap-production`（opt-in flag）+ `--clap-production-dll <path>`
  / `CLAP_PRODUCTION_DLL`（default `target/debug/
  ruviform_plugin_clap.dll`）。
- `CLAP_PRODUCTION_PLUGINS` = `[{ id: "org.ruviform.clap-gain-poc",
  name: "production_gain" }]` — fixture の `CLAP_PLUGINS` とは別
  cdylib のため target 単位で leg set を分離。
- CLAP legs 節を target ループ化（fixture target + opt-in production
  target）。guard 順序・informational fault leg（fixture のみ）・
  leg 順・evidence ファイル名規約（`clap-<name>.txt`）は不変。
- `buildFixtureArtifacts` は `--clap-production` 時のみ
  `ruviform-plugin-clap` を build 対象に追加。
- `report.artifacts.clapProductionDll` に path/sha256 を記録
  （未指定時 `null`）。fail-closed 契約は不変。
- `run.test.mjs` に contract test 3 件追加（parseArgs・既定で
  production leg 非出力・missing artifact で NotRun + verdict 非
  Passed）。

## TDD red-green

- **red（外部 validator）**: `evidence/rf732/abi-validator-red/` —
  production leg `Failed`（10 failed: param 系 9 +
  process-varying-sample-rates 1）、verdict Failed (exit 1)。
- **red（unit）**: `to_process_config_rounds_fractional_sample_rate`
  ・`param_info_reports_validator_clean_metadata` を先に追加 —
  旧コード（fract reject・`module: b"/"`）では失敗する契約。
  ※ D1 は test と修正を同一編集で適用したため unit red の一次
  証拠は validator 出力。
- **green（unit）**: `cargo test -p ruviform-plugin-clap --lib` —
  50 tests pass。
- **green（外部 validator）**: `evidence/rf732/abi-validator/` —
  production leg `Passed`（26 passed, 17 skipped, 1 warning,
  0 failed）、verdict **Passed (exit 0)**。

## 検証結果

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | pass |
| `mise run check ruviform-plugin-clap` | 0 | pass |
| `mise run lint ruviform-plugin-clap` | 0 | clippy `-D warnings` pass |
| `mise run test ruviform-plugin-clap` | 0 | pass（unit+integration+doc） |
| `mise run check-l1-real-plugin` | 0 | pass |
| `mise run lint-l1-real-plugin` | 0 | pass |
| `mise run test-l1-real-plugin` | 0 | pass（adapter `l1_real_adapter_e2e` + fixture 全 E2E lane） |
| `node --test ci/plugin-abi-validator/tests/` | 0 | 21 tests pass |
| `mise run test-plugin-abi-validator`（引数なし） | 1 | verdict NotRun — validator パス未指定の fail-closed 設計どおり。実走は明示パス指定で実施（`evidence/rf630/abi-validator/run.json` は実行で上書きされるため実行後に restore） |
| `node run.mjs --clap-validator … --vst3-validator … --clap-production` | 0 | verdict Passed — production_gain 26p/17s/1w/0f + fixture 全 leg + VST3 235/235。`evidence/rf732/abi-validator/` |
| `node run.mjs … --clap-production`（修正前・red 記録） | 1 | verdict Failed — production leg 10 failed。`evidence/rf732/abi-validator-red/` |
| `mise exec -- prek run --all-files` | 0 | pass |

## review-protocol 結果

実装 diff・doc chain・evidence を対象にレビュー実施。blocker なし
（詳細は PR 本文）。

## 残課題・範囲外メモ

- clap-validator 0.4.1 の param-fuzz panic（rf#726 管轄）は本 slice
  では遭遇せず — production leg は param-fuzz 系を含め全 pass。
- denormal perf warning（"3.99x longer to process denormals"）は
  fixture 同種の観察事項 — fail ではなく DSP tuning は範囲外。
- `min_frames` 未満 frames の寛容受理は観察事項（host 違反入力・
  安全性影響なし）。
- docs index（`docs/**/README.md`）は未同期 — Coordinator が統合時に
  再生成。
