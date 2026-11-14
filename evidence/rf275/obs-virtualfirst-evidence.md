# 検証記録 — Issue #275 Virtual-first Owner/受入整合（docs-only slice）

- 日付: 2026-11-15
- 対象 Revision: `d5aa1b4b`（origin/main HEAD、clean tree）
- ブランチ: `scarecrow173/rf275-virtualfirst`
- 環境: Windows 11 x64（Orca worktree）、`gh` CLI 認証済み

## 1. 参照 Issue の実在照合（`gh issue view`、2026-11-15 実測）

| Issue | repo | state | 照合コマンド |
| --- | --- | --- | --- |
| #627 | scarecrow173/ruviform | CLOSED | `gh issue view 627 --repo scarecrow173/ruviform` |
| #628 | scarecrow173/ruviform | OPEN | 同上（628） |
| #629 | scarecrow173/ruviform | OPEN | 同上（629） |
| #630 | scarecrow173/ruviform | OPEN | 同上（630） |
| #631 | scarecrow173/ruviform | OPEN | 同上（631） |
| #281 | scarecrow173/ruviform | OPEN | 同上（281） |
| #294 | scarecrow173/ruviform | OPEN | 同上（294） |
| #313 | scarecrow173/ruviform | CLOSED（closedAt 2026-11-13T19:25:03Z） | 同上（313） |
| #314 | scarecrow173/ruviform | CLOSED（closedAt 2026-11-11T05:41:14Z） | 同上（314） |
| #127 | scarecrow173/audio-lens | OPEN | `gh issue view 127 --repo scarecrow173/audio-lens` |
| #28 | scarecrow173/synvelis | OPEN | `gh issue view 28 --repo scarecrow173/synvelis` |

全参照が実在し、状態は設計正本・ledger §7.2 の記述と一致。
#313/#314 の CLOSED は旧 real-window lane 参照の陳腐化根拠。

## 2. 方針突合

- `POLICIES.md` §2.1「仮想優先の受入と共通テスト基盤」
  （Effective 2026-11-15）を確認: 実 OS GUI・実 Device API・
  Plugin Host の L2/L3 は仮想環境で受入、未実走は `NotRun`、
  物理機材・市販 Plugin は通常必須としない。
- Issue #275 本文「2026-11-15 Virtual-first正式受入Owner整理」
  を `gh issue view 275` で確認: 「Windows提供GUI/実Device
  API/Plugin Hostの必要L2/L3をRuviform #628〜631共通Harness
  とRF #281/#294・製品独立Ownerへ委譲」「設計Closeと実装受入
  を分離」。
- #628〜631 の役割は各設計正本題名で確認:
  `docs/designs/issue-628-deterministic-test-runtime-design.md`
  （VirtualClock/PCM/MIDI trace/Oracle）、
  `issue-629-virtual-endpoint-survey.md`（OS 仮想 Endpoint
  probe）、`issue-630-test-plugin-suite.md`（VST3/CLAP Test
  Plugin 実 ABI）、`issue-631-plugin-dock-scenario.md`
  （GPUI 実 Window runner/plugin-dock scenario）。

## 3. 実行した検証

| コマンド | OS | toolchain | exit code | 結果 |
| --- | --- | --- | --- | --- |
| `mise run fmt-check` | Windows 11 x64 | cargo fmt（workspace） | 0 | Pass（`cargo fmt --all -- --check` 差分なし） |
| `mise exec -- prek run --all-files` | Windows 11 x64 | prek 0.5.7 | 0 | Pass（trailing-whitespace/end-of-file/check-yaml/check-toml/mixed-line-ending/byte-order-marker/detect-private-key 全 Passed、check-illegal-windows-names は対象なし Skipped、actionlint Passed） |
| `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json` | Windows 11 x64 | node v26.12.1 | 0（blocked 結果） | `focus-required`＋本 slice 無関係の既存 broken-relation で blocked。canonical docs を増やさない本 slice では内部 doc chain で代替（`obs-virtualfirst-task.md` 判断メモに記録） |

cargo 系検証は docs-only slice のため未実行（POLICIES §5
比例原則 — 文書のみの変更は軽量チェックが原則）。
`*-msrv`・`*-full` 系はユーザー指定どおり未実行（daily CI
管轄）。

## 4. 変更差分要約

- `docs/designs/issue-275-observability-integration-design.md`:
  front-matter `updated` を 2026-11-15 に更新。Deferred
  Owner/Trigger 表の実 OS GUI・実 Device 行の再開
  Owner/Trigger を #628〜631 Harness＋#627 へ再指向し、
  表直後に dated note（2026-11-15 追記）を追加。
  Consumer Gate 割当の Windows L2/L3 行を整合させ、箇条書き
  に dated note を追加。status・承認記録・他節は不変。
- `evidence/rf275/obs-acceptance-ledger.md`: §7
  「Virtual-first Owner/受入整合の再指向」を追記のみで
  追加。§1〜§6 は改変なし（`git diff` で追加行のみを確認）。
- `evidence/rf275/obs-virtualfirst-{spec,design,plan,task,
  evidence}.md`: 新規（内部 doc chain＋本検証記録）。

## 5. review-protocol

- 実施形態: docs-only 小 slice のため proportionate な
  self-review（quick cycle 相当）を実施。blocker 0。
- 確認観点: 参照 Issue の実在・状態一致（§1/§2）、方針
  本文との矛盾なし、`NotRun`/`Deferred` の非昇格、既存
  記録の非改変（ledger は追加行のみ・design doc は dated
  note＋参照差し替えのみ）、編集範囲外ファイルの差分なし
  （`git status` で確認）。
