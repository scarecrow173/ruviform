# #262/#419 引き渡し文面 — Issue #388 OS-L1 CI 証拠（案）

本ファイルは Coordinator が Issue #262 / #419 へ投稿するための引き渡し文面（成果物）。投稿自体は Coordinator が行う。

---

## #262（GPUI Native First 採用Gate・OS/性能/互換性評価）向け

Issue #388 の OS-L1 slice として、対象 SHA `e93c438adc4af5b498e10926fe859ac1338ac390`（main HEAD・PR #599 merge）に対する既存 Full CI 証拠を照合しました。

- Run: https://github.com/scarecrow173/ruviform/actions/runs/43428172820 （`ci.yml` workflow_dispatch、conclusion=success、`ci-summary` verdict=PASS）
- 証拠レポート: `evidence/rf388-os-l1-ci/report.md`（本リポジトリ）

**L1 完了の可否:** OS-L1（Native Build/Link・必要 Feature・Headless/RT/CLI 試験）は Windows/macOS/Linux の 3 OS で Pass です。driver-neutral `NativeSurfaceSlot` 公開契約・Fake Driver 契約 fixture（NSC-01〜13）・2-phase move rollback・GPUI/OS private handle 境界ガード・`gpui`/`gpui-test`/`uitest` feature の Compile/Link（`windowed_test_app` bin link 含む）・RT 非干渉契約（rt-verify 166 tests + verify test-full の `rt_*`）が対象 SHA の CI で実走・成功しています。macOS/Linux は POLICIES.md どおり L1 のみ必須であり、実 GUI/物理 Device/実 Plugin は Deferred（NotRun≠Pass）です。

**実 Native Dock 非保証（重要）:** 本証拠は契約層・headless 層の Compile/Link/試験であり、**実 Native Dock の Pass ではありません**。GPUI メイン画面内への実プラグインネイティブ GUI Dock は未実証で、Windows では DComp topmost による画素合成問題（#532）が未解決のままです。`Floating Window` の Dock 位置への重ね合わせ、`Generic Parameter Editor` の表示、Fake/Mock Surface の成功は、いずれも実 Native Dock Pass として扱いません。採用 Gate の判定材料としては「L1 契約基盤は 3 OS で成立、実 Dock は OS/format ごとの capability 判定待ち」という位置づけです。

## #419（#262 PoC 判定に基づく後継 ADR 最終採用）向け

#388 OS-L1 の引き渡しに同じです。#419 の ADR 最終採用判断では、以下を区別して扱ってください。

- **確認済み（Pass）:** 3 OS の OS-L1 CI 証拠（上記 run・対象 SHA 固定・全 checkout leg の実測 SHA 一致）。
- **未保証（NotRun/Deferred）:** 実 Native Dock（全 OS/format）、Windows L2/L3 実 Plugin 画素（#532 待ち）、macOS/Linux の実 GUI/物理 Device/実 Plugin（利用要件発生まで Deferred）。
- **禁止事項の再確認:** Floating/Generic/Mock を実 Native Dock Pass として採用根拠にしないこと。OS/format capability matrix の NotRun/Unsupported 行を Pass に繰り上げないこと。

---

補足: 本文面は `evidence/rf388-os-l1-ci/report.md` の判定まとめと整合しています。Issue #388 自体は Windows L2/L3（#532 決裁・実 Plugin 受入）の残件があるため open 継続です。
