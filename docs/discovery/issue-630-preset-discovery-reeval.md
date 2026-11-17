---
id: "DSC-630presetDiscoveryReeval01"
type: "discovery"
status: "final"
title: "Issue #630 preset-discovery 外部検証再評価 — upstream fix pin 調査"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/discovery/issue-630-abi-validator-rerun.md"
    - "docs/specs/issue-630-abi-validator-rerun.md"
  related:
    - "../specs/issue-630-preset-discovery-reeval.md"
---

# Issue #630 preset-discovery 外部検証再評価 — 調査記録

前 slice は clap-validator 0.4.1 の `preset-discovery-(crawl|load)`
deadlock（upstream free-audio/clap-validator issue #41）を理由に
library-level 除外を宣言した。本 slice では upstream PR #41 merge
commit `47e242404df89f3c8454aaf1c02bb0caca6db2ce` を含む build を
pin し、除外を解除して実走する。

## 確認した事実

- 旧 pin: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/clap-validator.exe`
  version `0.4.1`、SHA-256
  `825439ad3ff98770a24656013eeb11591407687b97058e1db9549c4f7d53e38a`。
- 新 pin: `%LOCALAPPDATA%/ruviform/plugin-abi-validator/pin-47e2424/clap-validator.exe`
  source HEAD `47e242404df89f3c8454aaf1c02bb0caca6db2ce`、SHA-256
  `b4bd645877bba3d5abe4300a84c5785312ce16f7d1179a2fd3177eedc45afe26`。
  `--version` は `0.4.1` 表記だが source は PR #41 merge を含む。
- VST3 validator は前 slice と同一 pin
  （`vst3sdk-build/bin/Release/validator.exe`、SHA-256
  `5889a89685cd0362941659011ec82d8584e0ebb39d77030d1a2b2cbd2d68a4fd`）。

## 判断

除外は upstream fix を含まない validator への workaround であり、
fix 含有 build が用意できた時点で解除して実測するのが正しい境界。
`crates/` は変更せず、runner の宣言的除外だけを外す。
