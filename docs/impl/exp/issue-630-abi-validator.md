---
id: "EXP-630abiValidator08"
type: "impl"
status: "completed"
title: "Issue #630 外部 ABI validator slice — 実験記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-abi-validator.md"
    - "docs/impl/exp/issue-630-abi-validator-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-abi-validator-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-630-abi-validator.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-abi-validator.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 外部 ABI validator slice — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-abi-validator-exp.jsonl`
（`experiment_event.v1` schema）を参照。

## 要約

1. clap-validator 0.4.1 の pinned release binary を provisioning
   （sha256 を release page 記載値と照合）。
2. VST3 SDK を clone（commit `3cdf9ca`）し CMake+MSVC で
   `validator` target を build。Git Bash PATH に cmake が無かったため
   VS2022 BuildTools 同梱 cmake の絶対 path で configure/build。
3. 初回実走で CLAP/VST3 双方に複数の契約違反を検出（discovery 文書の
   表を参照）。各違反を validator 側コード・fixture 側実装の双方から
   診断し、fixture crate 内で根本原因を修正（TDD: 先行失敗 test）。
4. `test_stateful` の state-reproducibility 失敗は、`state.load` が
   param 値を変えても `clap_host_params.rescan` を要求しなかった
   CLAP 契約違反と特定。host handle を `PluginStateImpl` に保持させ
   rescan を要求する形に修正。
5. `param-fuzz-sample-accurate` で clap-validator 自体が panic
   （exit 0xc0000409 / "a scoped thread panicked"）— tool 側 bug と
   特定し、fault fixture の param-mutating case を宣言的除外 +
   informational leg 記録で対処。
6. VST3 は staging layout（`Contents/x86_64-win/`・inner=outer 名）
   を整え、lifecycle/bus 契約を修正して `235 passed, 0 failed` を達成。
7. runner を fail-closed 契約で実装し、contract test 18 件で固定。
   canonical 実走で verdict Passed / exit 0。
