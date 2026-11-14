# Linux 配布設計 CI 照合 — 照合手法の試行記録（2026-11-14）

`docs/tasks/linux-distribution-task.md`（Issue #127・branch `docs/127-linux-distribution`）の CI 照合で試行した手法と結果の記録。コード変更を伴わない docs/evidence slice のため、実験は「照合手法の選択と判定の根拠」に限定される。正本の照合結果は
[`evidence/rf127/linux-ci-crosscheck.md`](../../../evidence/rf127/linux-ci-crosscheck.md)
を参照。

## 試行 1: `gh` による run/job/log/artifact の 4 段照合

- **目的**: 既存 CI lane の「定義」と「実績」を混同せず、対象 revision と成功 run を SHA 単位で対応付ける。
- **試行**: `gh run list --workflow ci.yml` で main 上の run を列挙 → `gh run view <id> --json jobs` で lane×OS の job 結果を取得 → `gh run view --job <id> --log` でテスト集計（`test result: ok`・passed/failed 数）を確認 → `gh api .../actions/artifacts` + zip 展開で `checkout-sha-<leg>` artifact の実測 SHA を照合。
- **観測**: run 43439434737（headSha `365c74ac`）は 20 job 全 success・`ci-summary` verdict=PASS。`verify (ubuntu-latest)` の `test-full` は `test result: ok` 201 block・`FAILED` 0。`ui-gpui-offscreen-verify` は `real_window` 3 passed / 0 failed（102.92s）。checkout-sha artifact は verify-ubuntu-latest・ui-gpui-verify-ubuntu-latest・ui-gpui-offscreen-verify の 3 leg で `365c74ac4b8e696044fb9bae386dfd03f86ecf32` と一致。
- **結論**: 4 段照合（run 一覧→job→log→artifact）で「job green = 対象 SHA の実測成功」を SHA 証拠付きで確認できた。採用（evidence §2・§3・§4）。

## 試行 2: GPUI lane の系譜復元（いつから・なぜ失敗したか）

- **目的**: `ui-gpui-verify`/`ui-gpui-offscreen-verify` の成功を「最近できた lane」なのか「以前からあった lane」なのか区別し、初回失敗と修正の因果を記録する。
- **試行**: `git log -S "ui-gpui-offscreen-verify" -- .github/workflows/ci.yml` で lane 追加 commit を特定（`ae70b57e`・PR #566・Issue #314 RF-GT-05）。`gh run list` で 2026-11-12 の失敗 run（43140254535・43270011255）を確認し、修正 commit `3692c6fa`（PR #593・Issue #582: cold-cache `cargo fetch --locked`・libc pin・moxcms 境界）との対応を取った。`git log -S` で `msrv-verify`（PR #595）・`ui-gpui-verify-macos`（PR #599）の追加時期も確認。
- **観測**: GPUI lane は 2026-11-12 に追加され、初期 main run で fail → Issue #582 修正後に初緑（run 43326874974 @ `3692c6fa`）。最新緑は run 43439434737 @ `365c74ac` で現行 lane 構成（msrv・macos 含む 20 job）を全てカバー。
- **結論**: lane 系譜を「追加→初期失敗→修正→初緑→最新緑」で記録することで、成功 run の代表性（どの lane 構成をカバーするか）を SHA 単位で説明できる。採用（evidence §2.2）。

## 試行 3: 現行 HEAD と最新成功 SHA の差分評価

- **目的**: 最新成功 run の SHA `365c74ac` を現行 main HEAD `621490e8` の証拠として使えるか評価する。
- **試行**: `git log 365c74ac..621490e8` と `git diff --name-only` で差分を確認。`gh run list --branch main` で HEAD の ci run 有無を確認。
- **観測**: 差分に `crates/`・`mise.toml`・`.github/workflows/ci.yml` を含む PR #608〜#618 があり、`621490e8` の ci run は存在しない（ci.yml は workflow_dispatch 専用・daily-ci scheduler が dispatch）。収集中に HEAD が `c49e2a81`→`621490e8` へ前進したが、いずれの SHA にも ci run はない。
- **結論**: `365c74ac` の CI 証拠は祖先 SHA の証拠であり HEAD の Pass ではない。HEAD は `NotRun(Pending)` として分離記録。採用（evidence §2.1・§6、design §1・§8）。

## 試行 4: 文書生成 script の gate と出力先

- **目的**: spec→design→test-spec→plan→task チェーンを script で生成する際の gate 条件と、scope 指定ファイル名への合わせ方を確認する。
- **試行**: `new_spec.js`/`new_design.js`/`new_test_spec.js` は `--name` で所定ファイル名を直接生成。`new_plan.js` は approved design を要求（`PLAN-DOC-GATE-001`）、`new_task.js` は approved/in-progress/completed の plan を要求（`TASK-DOC-GATE-001`）。`new_impl_record.js` は title slug で生成するため `mv` で `linux-distribution.md` に rename。exp は `.md` 試行記録として手動作成（`.jsonl` 実験ログではない — issue-532 precedent）。
- **観測**: plan/task の script gate を通すには design を approved にする必要がある。task contract の「ユーザー承認の代わりに review-protocol で blocker なしを承認とする」規定に従い、レビュー後に design status を approved に遷移させて plan/task を生成した。
- **結論**: skeleton 生成 → 内容記述 → review → design approved → plan/task 生成 → index 再生成の流れを採用。`docs/impl/exp/` の `.md` は試行記録、`.jsonl` は CLI 管理の実験ログとして区別。

## 試行 5: relation の reciprocal 整合と編集範囲外ファイルへの対応

- **目的**: 新規チェーンが先行 `issue-127-linux-dist-design.md` チェーン（編集範囲外）と relation で接続する際、reciprocal relation の audit finding を回避できるか確認する。
- **試行**: `audit_docs.js` の `inconsistent-reciprocal-relation` ルールを確認。target の逆方向フィールドが空なら finding は発火しないことを確認し、先行 spec/design/test-spec/plan/task の逆方向フィールド（`implemented-by`/`refined-by`/`derived-by`/`superseded-by` 等）が空であることを grep で確認。
- **観測**: 先行チェーンの逆方向フィールドは全て空のため、新規側から `refines`/`related`/`references`/`source` で参照しても reciprocal 整合の問題は生じない。ただし `verifies`/`verified-by` は旧 design の `verified-by` が非空（旧 test-spec を指す）のため、新規 test-spec から旧 design への `verifies` は避け `refines`/`related`/`references` を使用した。
- **結論**: 編集範囲外の先行ファイルを変更せずに、新規チェーンを `refines`（design/test-spec）+ `related`/`references`/`source` で接続できる。採用。
