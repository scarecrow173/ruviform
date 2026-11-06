---
id: "DISC-034S9kMn2pQ7rT4vX8yB1w"
type: "discovery"
status: "resolved"
title: "Issue #294 Cross-Consumer候補Gate — consumer-verify.yml 現状監査と要件ギャップ"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/294"
  changes:
    added: []
    modified: []
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
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    - "docs/designs/issue-367-daily-scheduler.md"
    - "docs/designs/issue-327-cross-repo-ci-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #294 Cross-Consumer候補Gate — consumer-verify.yml 現状監査と要件ギャップ

## 監査対象と方法

`.github/workflows/consumer-verify.yml`（PR #273/#274 実装済み、main `49de02c` 時点）を、Issue #294 の 2026-11-05 改訂要件（Daily Full CI＋候補tuple判定、#365/#367/#368 が運用正本）および GUI Testing v1.0 追加 AC と突き合わせた。両 Consumer のローカル checkout（audio-lens・synvelis main）の mise task / tools も実測確認した。

## 既存機構（監査結果・有効）

- **candidate SHA scratch 注入**: `sed` で manifest の ruviform git 依存行を `rev = "$RUVIFORM_REV"` に正規化する。rev pin 置換・branch 除去・rev 未指定注入を同一ルールで処理。両 Consumer は現在「rev 未指定の既定ブランチ追従」形態（AudioLens は #140 で lockfile 正本化済み）で、注入なしでは `cargo fetch` が lockfile 残留 SHA を維持し候補を検証しない — **注入は lockfile 正本移行後も必要かつ有効**。
- **single-SHA lockfile guard**: `cargo fetch` 後に `Cargo.lock` の `scarecrow173/ruviform.git` source 行が全て `?rev=<候補>#<候補>` であることを grep で検査（0 行・混入を fail）。
- **access mode 解決**: preflight が deploy_key / anonymous HTTPS / not_run を lane 別に解決。秘密は runner temp のみ。
- **not_run≠pass**: summary job が「全 lane not run の run は失敗」「job failure/cancelled は失敗」を強制。
- **lane 独立性**: verify-audiolens / verify-synvelis は needs 非依存で片側失敗が他側を止めない。

## ギャップ（Issue 要件との差分）

1. **Consumer 解決 SHA の未捕捉**: `git clone --branch <ref>` は解決済み SHA を記録しない。Run API の `head_sha` は Ruviform 側のみで、tuple（Ruviform/AudioLens/Synvelis 各 main SHA）を run 証拠から再構成できない。→ 各 lane が clone 後に `git rev-parse HEAD` を捕捉する必要。
2. **期待 tuple 照合の不在**: dispatch 入力に期待 Consumer SHA を渡す契約がなく、判定〜checkout 間の Consumer main 漂移を検出できない。期待 SHA 入力 + fail-closed 照合が必要。
3. **構造化証拠の不在**: 証拠は Job Summary の Markdown 表のみで機械再取得不可能。「全 SHA と結果を構造化 JSON artifact/Job Summary へ記録」が必要。
4. **Consumer ref が branch/tag 限定**: `git clone --branch` は raw SHA を受け付けない。RF #305＋SV #84 候補 PR head SHA や AL #143 候補を直接検証する経路（候補tuple連携）がない。→ fetch＋checkout FETCH_HEAD 経路で branch/tag/SHA を統一受理。
5. **UI lane 未接続**: AudioLens `setup-ui`/`check-ui`/`test-ui`（headless tsc+happy-dom）と vendor REV 整合（`check-ruviform-rev --require-vendor`）、Synvelis `setup-ui`/`check-ui`/`test-ui`/`build-ui`（SolidJS 実 build＋assets manifest）と `check-ruviform-rev`/`test-rt`/`check-headless-deps` が workflow 未使用。「Headless pass のみで実 GUI pass にしない」「UI vendor/lockfile 整合」を両 Consumer で段階接続する余地がある。
6. **RF #306 接続点の不在**: Consumer 側が後続で定義する fixture/test command を候補 run に載せる汎用入口がない。Consumer repo 直接編集は本 Issue 外なので、workflow 側に「追加 mise task 注入入力」として接続点を置くのが既存機構内の解。
7. **`unsupported` 状態語彙の不在**: OS/capability 起因で lane が実行不能な場合、`not_run` か `fail` のどちらかに倒すしかない。AC が要求する Missing/NotRun/Unsupported/Fail の独立判定語彙が不足。
8. **Consumer 側 guard tool 未利用**: 両 Consumer が `check-ruviform-rev`（lockfile 単一 SHA＋vendor REV＋manifest 整合の fail-closed 検証）を持つが、workflow は自前 grep guard のみ。Consumer 側 authoritative 検証を lane 化すれば vendor/lockfile 整合を Consumer 自身の契約で担保できる。

## 境界の確認（他 Issue 責務を取り込まない）

- **Daily tuple 判定・重複抑止・bounded retry**: #367 `ci/daily/decide.mjs`＋`daily-ci.yml` の責務（`DAILY_TARGET_WORKFLOW` で dispatch 先を差し替える設計済み）。本 Issue では「scheduler が dispatch に必要な期待 SHA 入力契約」と「tuple 証拠を run/artifact から回復できる構造」を提供し、scheduler 自体は作らない。
- **失敗 Issue 集約**: #368 tracker の責務。`ci-issue-tracker.yml` は現行 `workflows: [ci]` のみ監視で、consumer-verify 失敗の自動起票範囲は別 Issue 判断。本 workflow は失敗証拠を構造化記録するに留める。
- **実 GUI desktop/実機 lane**: RF #313 OS driver / RF #314 visual golden の Owner 作業。実窓 host 依存 lane は `unsupported`/`not_run` 理由付きで正直に記録し、skip を pass にしない。
- **Consumer 独立 CI（AL ci.yml / SV ci.yml）**: 置換しない。証拠の再利用は「完全に同一候補 revision・条件」のみ。

## 利用可能な Consumer 側 lane 実測（2026-11-07 ローカル checkout）

| Consumer | task | 内容 | 備考 |
| --- | --- | --- | --- |
| audio-lens | `check-ruviform-lock` | Cargo.lock 正本の ruviform 単一 SHA 検証 | #140 lockfile 主体 |
| audio-lens | `check-ruviform-rev` / `--require-vendor` | lockfile+manifest rev+vendor REV 整合 | vendor 必須化 flag あり |
| audio-lens | `sync-ui-deps` | ui/vendor を Cargo.lock SHA で再生成 | cargo checkout 依存 |
| audio-lens | `setup-ui` → `check-ui` / `test-ui` | npm ci → tsc typecheck + vendor 照合 / node:test | headless・ubuntu 可 |
| audio-lens | `check/lint/test-ui-webview` | webview feature（実窓 host） | 実機 lane・本 run 対象外 |
| synvelis | `check-ruviform-rev` / `--require-vendor` | lockfile+REV+assets manifest 整合 | manifest は build-ui 産物 |
| synvelis | `sync-ui-deps` → `setup-ui` → `check-ui`/`test-ui`/`build-ui` | vendor → npm ci → tsc/vitest/vite build | headless・ubuntu 可 |
| synvelis | `test-rt` / `check-headless-deps` | RT lane / headless 依存 guard | 軽量追加可能 |
| synvelis | `check/lint/test-app-webview`・`build-app` | webview / 本番 build | 実機・重 lane・本 run 対象外 |

## 判定

既存 workflow は骨格（注入・guard・lane 分離・not_run≠pass）が健全で、ギャップは「tuple 証拠」と「段階 lane 拡張」の 2 系統に限定される。ワークフロー全面書換ではなく、同一 job 構造内での入力拡張・証拠収集 step 追加・lane 追加で整合的に対応可能。spec へ進む。
