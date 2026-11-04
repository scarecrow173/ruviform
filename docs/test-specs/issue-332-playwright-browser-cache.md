---
id: "TSPEC-EXjezAr9qVRhcJq18LPw8U"
type: "test-spec"
status: "approved"
title: "Issue #332 CI-OPT-05 test specification — browser cache・fallback・分離計測"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-332-playwright-browser-cache.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-332-playwright-browser-cache.md"
    - "docs/plans/issue-332-playwright-browser-cache.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-332-playwright-browser-cache.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #332 CI-OPT-05 test specification — browser cache・fallback・分離計測

## Purpose

spec AC-1〜AC-6 の検証契約を固定する。検証対象は「CI パイプラインの browser 準備の振る舞い」であり、Playwright suite 自体の意味的正しさ（30 spec が担保するもの）を再検証しない。

## Feature

Chromium バイナリが version/revision/OS-arch key の cache で再利用され、miss・破損・版変更で必ず正しいブラウザを再取得し、browser setup・download・実テスト時間が step 単位で分離記録されること。

## Rules

- Rule: cache key は `playwright-<os>-<arch>-pw<version>-chromium<revision>`。`@playwright/test` version と `playwright-core/browsers.json` の chromium revision、runner OS/arch に紐づく。key 算出は失敗時に job fail（fail-closed）。
- Rule: cache miss・版変更（key 変化）では `playwright install chromium` が正しい revision を全量取得する。restore-keys は使わない。
- Rule: 復元された binary が launch 不能な場合、検証 step が browsers path を全削除して再 install → 再検証する。再検証も失敗なら job fail（破損 binary でテストに進まない）。
- Rule: Linux の `playwright install-deps chromium`（apt system library）は browser cache とは独立に毎 run 実行される。browser cache hit でも省略しない。
- Rule: `test-ui-browser` は従来どおり単独実行で完走する（depends 経由で browser install を含む）。`run` は page build + test の検証本体のみ。
- Rule: Evidence 生成物（`test-results/`・evidence.json・trace・screenshot）・30 日 artifact 保持・fixture 範囲・`retries: 0` は不変。
- Rule: 新 step 名は `collect-ci-metrics.mjs` の既存 lane 分類に整合する（restore→`cache-restore`・deps/prepare/validate→`playwright`・test→`ui`）。
- Rule: Browser PASS は browser-lane 証拠であり OS WebView/native GPU/製品 E2E の成功として扱わない（証拠表記・コメントで明記維持）。

## Examples

- Example: 初回 run（cache 不在）→ `Cache not found` → prepare step が `Downloading Chrome for Testing … (playwright chromium v1248)` を出力 → post-job で key 保存。次回 run → `Cache hit for restore-key` → prepare step が download なしで終了。
- Example: `@playwright/test` を bump → `browsers.json` の revision 変化 → 新 key で miss → 新 revision を取得（古 cache は触らない）。
- Example: browsers path の binary を削除/破壊 → launch 検証 fail → `$PLAYWRIGHT_BROWSERS_PATH` 全削除 → install → 再検証 pass → test 進行。
- Example: `mise run test-ui-browser` 単独（fresh）→ install-ui-browser-test（npm ci）→ install-ui-browser-chromium（install）→ build-ui-showcase → page build → test 緑。

## Guarantees

- browser 取得の最適化が「検証の skip」ではなく「バイナリの再利用 + install の存在確認」で実現されていること。
- cache miss 経路が常に完全な install パス（install-deps + install + launch 検証 + test）を踏むこと。
- install/launch の失敗が prepare/validate step として、test 失敗と分離して報告されること。
- version/revision の実変更が必ず新しい browser 取得を引き起こすこと（key の保守性）。

## Non-goals

- Playwright suite の個別 spec の正しさ・並列化（既存 lane と #310 契約が担保）。
- Linux apt system deps の取得コスト最適化（Deferred）。
- cache 内容の暗号学的完全性（launch 検証 + test suite の意味的検知に委譲。cache は workflow と同一 review 境界内）。

## Risk

- これら保証を落とすと、stale/破損 browser による偽緑・外因 download 失敗の再発・browser 準備失敗と test 失敗の識別不能が入り得る。launch 検証と exact-key fail-closed は「壊れた場合に遅くなるが間違えない」側に寄せてあるため、崩れたときの症状は効率低下に留まり検証欠落にならない設計。
