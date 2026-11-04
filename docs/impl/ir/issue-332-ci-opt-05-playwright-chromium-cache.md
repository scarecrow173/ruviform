---
id: "IMPL-OVLf8tC84aP8OgMOOCOMve"
type: "impl"
status: "completed"
title: "Issue #332 CI-OPT-05 Playwright Chromium binary cache + 準備 step 分離"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/332"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-332-playwright-browser-prep-inventory.md"
      - type: "file"
        path: "docs/specs/issue-332-playwright-browser-cache.md"
      - type: "file"
        path: "docs/designs/issue-332-playwright-browser-cache.md"
      - type: "file"
        path: "docs/plans/issue-332-playwright-browser-cache.md"
      - type: "file"
        path: "docs/tasks/issue-332-t1-playwright-browser-cache.md"
      - type: "file"
        path: "docs/test-specs/issue-332-playwright-browser-cache.md"
      - type: "file"
        path: "docs/impl/ir/issue-332-ci-opt-05-playwright-chromium-cache.md"
      - type: "file"
        path: "docs/impl/exp/issue-332-playwright-browser-cache.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/issue-327-cross-repo-ci-baseline.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-332-t1-playwright-browser-cache.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-332-playwright-browser-cache.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-332-playwright-browser-cache.md"
    - "docs/designs/issue-332-playwright-browser-cache.md"
    - "docs/test-specs/issue-332-playwright-browser-cache.md"
    - "docs/discovery/issue-332-playwright-browser-prep-inventory.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #332 CI-OPT-05 Playwright Chromium binary cache + 準備 step 分離

## Summary

`ui/browser-test` の Playwright Chromium 準備を実テストから分離し、browser binary を `actions/cache` で再利用化した。`install-ui-browser-chromium` mise task を新設し `test-ui-browser` は depends 経由で解決（run は page build + suite のみに純化）。CI `ui-verify` では `PLAYWRIGHT_BROWSERS_PATH` を workspace 内 `.cache/ms-playwright` に pin し、`playwright-<OS>-<arch>-pw<version>-chromium<revision>` key（restore-keys なし）で復元 → 専用 install step → launch 検証 → 実 test step の構造とし、download/setup と test 時間を既存 lane 分類で分離計測可能にした。Linux system deps（`playwright install-deps chromium`）は cache と別レイヤーで毎 run 実行を維持。

## Context

task `TASK-O3Q2a0lPvct9PFKBgBig2j`（plan `PLAN-xLvFbRwmfJj1P2olT3eT9q`、spec `SPEC-x9aakFSwqMZzdqGoq2uyc3`、design `DESIGN-EJ86GHTwsKgyexnCdTpSJ5`、test-spec `TSPEC-EXjezAr9qVRhcJq18LPw8U`、discovery `DISC-k7Pjkn8XoebpncwaTSISdf`）として実施。Issue #332 CI-OPT-05、#327 の Before/After テンプレートに整合する計測を併記。依存の #327/#330 は両方 main 入り済み。

## Implementation

### Changes

- `mise.toml`
  - `[tasks.install-ui-browser-chromium]` 新設。`depends = ["install-ui-browser-test"]`、`run = "npm --prefix ui/browser-test exec -- playwright install chromium"`。mise `sources`/`outputs` freshness は意図的に付与しない — browser の実在・revision 一致は Playwright の revision-aware installer が authoritative に判定する（既インストール済みなら即 no-op）。
  - `test-ui-browser` の `depends` に `install-ui-browser-chromium` を追加し、`run` から `playwright install chromium` 行を除去（`npm run build` + `npm test` のみ）。ローカル実行は従来通り 1 コマンドで自足。
  - browser lane のコメントを更新（Chromium 準備の分離と Linux install-deps が task 外である旨を明記）。
- `.github/workflows/ci.yml`（`ui-verify`）
  - job env に `PLAYWRIGHT_BROWSERS_PATH: ${{ github.workspace }}/.cache/ms-playwright` を追加（3 OS 同一の workspace 内 path で cache 対象を固定）。
  - `Install Playwright browser system deps (Linux)` step は従来通り Ubuntu のみで実行（cache と分離した system deps レイヤーとして維持）。
  - `pw-meta` step 新設: installed `@playwright/test` の version と `playwright-core/browsers.json` の chromium revision を Node で読み出し output 化。
  - `Restore Playwright browser cache` step 新設: `actions/cache@v6.1.0`、key `playwright-${{ runner.os }}-${{ runner.arch }}-pw<version>-chromium<revision>`。`restore-keys` なし — 版不一致の stale binary を復元しない。
  - `Install Playwright Chromium (mise run install-ui-browser-chromium)` step 新設: cache hit 時は Playwright installer が存在確認して即終了、miss/部分復元時は不足分のみ download。
  - `Validate Playwright browser launch` step 新設: `playwright screenshot` で実 launch を fail-closed 検証。失敗時は guarded `rm -rf "${PLAYWRIGHT_BROWSERS_PATH:?...}"` で wipe → install task 再実行 → probe 再実行（2 回目も失敗なら step fail）。
  - `Run UI browser tests (mise run test-ui-browser)` は step 名・配置不変のまま、内部で browser install を行わなくなった。
  - Evidence upload（`ui/browser-test/test-results/`・30 日保持・if: always()）は不変。
- `AGENTS.md`: 検証入口テーブルに `install-ui-browser-chromium` 行を追加し `test-ui-browser` 行を新構造へ更新。`ui-verify` 説明文に browser cache（version/revision/OS-arch key）と system deps 分離を明記。
- `docs/designs/issue-327-cross-repo-ci-baseline.md`: Deferred 項目「Playwright setup の cache 分類」を #332 で解決済みに更新（browser setup step が `playwright` lane へ分類可能になった旨）。

### Decisions During Implementation

- cache key に browser 種別を含めず `chromium<revision>` に固定 — 現在の suite は chromium のみ使用。version と revision の両方を含めることで、lockfile 内 Playwright 更新時に必ず新 key となり安全側に倒れる。
- `restore-keys` を付けない — 部分一致で旧 revision binary を復元すると、そのままでは test が旧 browser で走り得るため。miss 時は install step が正規 installer で取得する。
- launch 検証に `playwright screenshot` を採用 — 追加スクリプト不要・実 browser 起動を確認できる最小 probe。ファイル一覧チェックでは破損 binary を検出できないため実 launch 方式にした。
- 破損検知後の wipe は `:?` guard 付き `rm -rf` のみ（対象は workspace 内固定 path）。cache 復元内容が壊れている場合に部分修復より全消去+再 install が安全側。
- 並列化（Playwright `--workers` 変更や matrix 分割）は導入しない — 現行 30 件/6s 程度の suite で flake・runner 負荷を高める根拠がなく、issue でも optional。
- `test-ui-browser` の `depends` に `build-ui-showcase` を従来通り残す — page build に showcase dist が必要な契約は不変。

## Related Experiments

### Adopted

- `PLAYWRIGHT_BROWSERS_PATH` カスタム path 検証（ローカル Windows）: env pin で `chromium-1248`・`chromium_headless_shell-1248`・`ffmpeg-1013`・`winldd-1007` が指定 dir へ install されることを確認。
- 破損 fallback 実演: headless shell binary を破壊 → probe 失敗 → wipe → reinstall（実 download）→ probe 成功を end-to-end で確認。

### Rejected

- `actions/cache` の `restore-keys` による prefix fallback: stale revision 復元リスクのため不採用。
- system deps（apt）の cache 化: browser binary と混在不可・apt 層は runner image 管理のため不採用（issue 要件どおり分離維持）。
- Playwright version のみの key: chromium revision が playwright-core の browsers.json 由来で version 更新と連動するが、明示的に revision も key へ含めて安全側にした。

## Validation

実測環境（ローカル）: Windows 11 x64 / Git Bash、mise 管理 toolchain（node 26.12.1）。

| 項目 | 実行 | 結果 |
| --- | --- | --- |
| `mise run install-ui-browser-chromium` | depends（install-ui-browser-test）解決 + playwright install | Pass — browser 既存時は ~2s で no-op |
| `mise run test-ui-browser` | install-ui-browser-test + build-ui-showcase + install-ui-browser-chromium 解決後、page build + suite | Pass — 30/30（6.1s）、task 全体 ~16s |
| `mise run check-ui-browser-test` | tsc --noEmit | Pass |
| `mise exec -- actionlint .github/workflows/ci.yml` | — | Pass（exit 0） |
| `mise exec -- prek run --all-files` | 全 hook | Pass |
| `PLAYWRIGHT_BROWSERS_PATH` pin への install | env 指定 dir | Pass — 4 種 binary が custom path へ |
| 破損 browser fallback | binary 破壊 → probe fail → wipe → reinstall → probe | Pass — `PROBE-OK` |
| doc_status lint（issue-332 docs 全 6 種） | lint --type each | Pass — 新規 finding 0 |

CI（PR #341、run 41815446750 — cold・cache miss、2026-11-04）:

- ui-verify (ubuntu-latest): pass 4m19s — cache miss → download 8s → probe 3s → suite 10s・Evidence upload 済
- ui-verify (windows-latest): pass 4m49s — cache miss → download 16s → probe 2s → suite 20s
- ui-verify (macos-latest): pass 3m6s — cache miss → download 16s → probe 2s → suite 23s
- 他 9 required contexts（verify/l1-real-verify 各 3 OS・rt-verify・icc-verify・bench）+ consumer-impact classification: 全 pass・変更なし
- ubuntu log で `Cache not found for input keys: playwright-Linux-X64-pw1.64.1-chromium1248` → `Downloading Chrome for Testing 156.0.8078.4 (playwright chromium v1248)` → `.cache/ms-playwright` 配置 → post step で save を確認（cold=download 発生の証跡）

## Before / After

計測契約: `docs/designs/issue-327-cross-repo-ci-baseline.md`。収集: `ci/metrics/collect-ci-metrics.mjs`（raw: `docs/impl/exp/data/issue-332-playwright-browser-runs.jsonl`、詳細: `docs/impl/exp/issue-332-playwright-browser-cache.md`）。

### Before（代表 run 41780346388、main・変更前）

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | 6s | 0s(skip) | 0s(skip) |
| Run UI browser tests（page build + chromium download + 30 tests 同居） | 28s | 40s | 44s |
| Upload UI browser evidence | 1s | 2s | 2s |

- Chromium download は `test-ui-browser` 内で毎 run 発生（ubuntu job log 実測 ~9s）。download/setup と実 test の分離計測は不可能だった。
- Playwright cache: NotApplicable（専用 cache step なし）。

### After（PR #341 run 41815446750 — cold・cache miss）

| step | ubuntu | windows | macos |
| --- | --- | --- | --- |
| Install Playwright browser system deps (Linux) | 16s | 0s(skip) | 0s(skip) |
| Read Playwright browser version/revision | 1s | 1s | 1s |
| Restore Playwright Chromium cache | 0s(miss) | 1s(miss) | 1s(miss) |
| Prepare Playwright Chromium (install task) | 8s(download) | 16s(download) | 16s(download) |
| Validate Playwright Chromium launch | 3s | 2s | 2s |
| Run UI browser tests（page build + 30 tests のみ） | 10s | 20s | 23s |
| Upload UI browser evidence | 2s | 1s | 1s |
| Post restore（cache save） | 4s | 6s | 5s |

- cold でも test step 単体は Before より短い（ubuntu 28→10s / windows 40→20s / macos 44→23s）— download が install step へ分離され実 test 時間が可視化された。
- browser 準備合計（deps+meta+restore+install+validate）: ubuntu 28s / windows 20s / macos 20s。cold では Before の埋め込み準備とほぼ同等（差は download を cache save が相殺）。
- warm（cache hit）実測は次回 push run で採取 → `docs/impl/exp/issue-332-playwright-browser-cache.md` へ記録。期待値: install step が Playwright revision check で ~1-2s に短縮、download なし。

## Rollback

- `mise.toml`: `install-ui-browser-chromium` task 削除、`test-ui-browser` の depends から除去し run に `npm --prefix ui/browser-test exec -- playwright install chromium` 行を戻す。
- `ci.yml`: `PLAYWRIGHT_BROWSERS_PATH` env、pw-meta/cache-restore/install/validate の 4 step を削除（`test-ui-browser` step と install-deps step は残す）。
- 残存する GitHub cache エントリは key 分離（`playwright-*` prefix）のため従来経路と衝突しない。actions cache の削除は不要・自然失効でよい。
- 検証能力の削減なし — rollback 後も従来と同一の browser install + test 経路に戻る。

## Risks

- `PLAYWRIGHT_BROWSERS_PATH` は job env で全体に効くため、将来別 step が別 browser root を期待する場合は step env で上書きが必要。
- GitHub-hosted runner の cache 容量上限（10GB/repo）で他 cache と eviction 競合し得る。browser cache は ~0.4-1GB 程度で、eviction 時も miss → install fallback で正しく動く。
- `playwright screenshot` probe は headless shell 起動を確認するが、GUI mode の headed 起動までは保証しない（現行 suite は headless のみ使用）。
- Windows で restore した cache の ACL/attribution が install 直後と異なる可能性 — probe が実 launch を担保するため実害は検知される。

## Follow-ups

- PR #341 の CI 3 OS で cold（cache miss→download）と warm（cache hit→install no-op）の実測を採取し exp 記録へ反映（本 record の After 節も同時更新）。
- Playwright version 更新 PR では key の revision 部分が自動で切り替わるため、初回 run は cold になる（設計どおり）。warm 時間の再計測は不要。
