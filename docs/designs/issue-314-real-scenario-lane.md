---
id: "DESIGN-034Vg9PXhXAWj20QVdvHWj"
type: "design"
status: "approved"
title: "Issue #314 slice 4 follow-up — real-scenario lane + matrix 詳細設計"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "docs/specs/issue-314-real-scenario-lane.md"
    - "docs/designs/issue-314-real-window-adapter.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-314-real-scenario-lane.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-real-scenario-lane.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# real-scenario lane + matrix 詳細設計

## Context

slice 4（PR #554）で `RealOsGuiTest` と `tests/real_window.rs` は
merge 済みだが、canonical な再現経路（mise lane）と story matrix
残 cell（scale 2.0・第 2 theme）が未整備。本設計はその 2 点だけを
扱い、adapter 本体（`gpui_driver/real_window.rs`）は変更しない。

## 決定 1: mise lane `test-ui-real-scenario`

既存 real-OS lane（`example-ui-gpui-real-os-*`）と同じ運用位置
づけ — 対話 desktop session 前提の manual lane で、CI 必須 lane
には追加しない。`test-ui-*` 命名は `tests/` target を呼ぶ lane の
既存規約（`test-ui-gpui` 等）に合わせる。

```toml
[tasks.test-ui-real-scenario]
description = "..."
env = {
  RUVIFORM_DRIVER_LAB_EVIDENCE = "evidence/issue-314-real-scenario/driver-lab-evidence.jsonl",
  RUVIFORM_GOLDEN_EVIDENCE = "evidence/issue-314-real-scenario/golden-evidence.jsonl",
  RUVIFORM_REAL_WINDOW_PNG_DIR = "evidence/issue-314-real-scenario/png",
}
run = "cargo test -p ruviform-ui-gpui --features real-scenario --test real_window"
```

- evidence env は `driver_lab::evidence_target` と同じ
  workspace-root 相対規則で解決される（JSONL 2 系は既にその規則）。
  `RUVIFORM_REAL_WINDOW_PNG_DIR` は現行 CWD 相対のため、本 slice で
  test 側を workspace-root 相対解決に揃える（`evidence_target` と
  同じ規則の局所実装 — 新規共有 helper は作らない）。
- `env` に証拠出力先を固定するのは「canonical lane で再現可能な
  実行経路」の要件 — 手動で env を付け忘れても証拠が `target/`
  止まりにならない。

## 決定 2: environment_blocked の区分

`cargo test` は exit code 3 を表現できないため、verdict JSON +
exit code の example lane 方式は採らない。区分は evidence JSONL
（`observed` field）と panic message で表現する:

- `RealOsGuiTest::run` が `Err(UiError::Host | UiError::BackendUnavailable)`
  を返した場合 = セッション基盤（window 生成・platform loop）が
  環境理由で成立しなかった → `driver_lab::emit_evidence` に
  `observed="environment_blocked"` を記録し、
  `environment_blocked:` prefix 付きで panic（test は fail のまま
  — 「走らなかった」を pass に見せない）。
- `UiError::Internal`/`Unsupported`・body 内 `ScenarioError`・
  assert 失敗は GUI/test レベルの失敗として従来どおり panic
  （environment_blocked を騙らない）。

`os_input::preflight` 本体は適用しない: 本 lane は SendInput を
一切注入せず（semantic dispatch + PrintWindow/BitBlt capture のみ）、
preflight が検証する「注入の実効」は本 lane の環境要件
（interactive desktop で window が開けるか）と一致しない。適用
するのは verdict 区分の考え方のみ。

## 決定 3: story matrix を 2×2 で実走

`tests/real_window.rs` を Fake/GPUI-simulated 層（`tests/lab_story.rs`）
と同じ matrix 構造に拡張する:

- `THEMES = ["synvelis-dark", "audiolens-dark"]`、
  `SCALES = [1.0, 2.0]`（`stories.json` `scaleFactor` と同一）。
- story ごとに 4 cell（2 theme × 2 scale）を順に実行。各 cell は
  独立した `RealOsGuiTest::run` セッション（実 window を開く）で、
  theme は `ViewMount.theme` に `with_scale(ScaleContext)` 付きで
  mount、scale は mapper-side hint（token 不変を cell ごとに assert
  — Fake/GPUI 層と同一規則）。
- `GoldenKey.theme` は cell の theme 名（fixture 宣言 theme ではなく
  実 mount theme）。`scale_factor` は従来どおり capture 実測値
  （window DPI）— theme scale cell は key に現れないが、cell ごとに
  fresh store で baseline_missing → promote → pass を独立検証する。
- PNG/meta evidence の file 名に cell（theme・scale）を含め、
  cell 間で上書きしない。
- semantic oracle（`expected_lab_story_trace()`）と pixel 層
  （idle 非 uniform・teardown capture・golden loop）は cell ごとに
  実施 — shared oracle が theme/scale 不変であることの real-os 層での
  証明になる。
- `stories.json` `drivers.real-os.mechanism` を canonical lane を
  指す `mise-task`/`test-ui-real-scenario` に更新し、note を
  matrix 実績どおりに書き換える。

## 構成要素

- `mise.toml`: `[tasks.test-ui-real-scenario]` 追加のみ（既存 task
  不変）。
- `crates/ruviform-ui-gpui/tests/real_window.rs`: matrix 化 +
  env-blocked 区分 + PNG dir の workspace-root 相対解決。
- `crates/ruviform-ui-test/fixtures/driver-lab/stories.json`:
  `drivers.real-os` の status/note/mechanism 更新。
- `evidence/issue-314-real-scenario/`: lane 実行の JSONL + PNG/meta。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-314-real-scenario-lane.md`: 文書チェーン + IR。

## Non-Functional Constraints

- manual lane のため実行時間は許容（story あたり 4 session、
  window open + settle + capture を含め数十秒オーダー）。
- headless/非対話 session では window が開けず environment_blocked
  として記録される — 無限待ち・ハングは `COMMAND_TIMEOUT`（15 s）
  と platform loop の既存 timeout が防ぐ。

## Risks and Trade-offs

- cell ごとの実 window open は時間がかかるが、cell 間で window/theme
  を使い回すと mount 世代・palette の分離が崩れるため独立 session
  を選ぶ（slice 4 と同じ構造の繰り返しで複雑性を増やさない）。
- `UiError::Host` を environment_blocked に分類するのは「session
  基盤の失敗 = 環境」を意味する — 対話 session 上で Host error が
  出た場合も environment_blocked と記録されるが、その場合こそ
  環境要因（GPU/driver/session）の調査対象であり、GUI assertion
  fail と混同しない区分としては正しい。

## Deferred Design Concerns

- `cargo test` target に exit-code 区分（0/1/2/3）を持たせる
  runner 化 — example lane の verdict JSON 方式への移行は将来
  scope。今回は JSONL `observed` + panic tag で区別する。
- `docs/development-environment.md` の lane 対応表への追記 —
  Issue 名を持たない管理文書のため本 slice 範囲外。

## Verification Notes

- [ ] `mise run test-ui-real-scenario` が対話 desktop session で
  pass し `evidence/issue-314-real-scenario/` に証拠を出力する
- [ ] `mise run test-ui-scenario-contract` 等の既存 lane が
  `stories.json` 更新後も pass する
