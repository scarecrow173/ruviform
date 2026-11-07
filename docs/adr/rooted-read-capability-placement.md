---
id: "ADR-kT9SrfiIkX357aVVTGRp91"
type: "adr"
status: "accepted"
title: "Rooted Read Capability の配置と OS primitive 共有方式"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/304"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/rooted-read-capability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-304-io-archive-extract.md"
    - "../designs/issue-304-io-archive-extract.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/rooted-read-capability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Rooted Read Capability の配置と OS primitive 共有方式

## Context and Problem Statement

SV85/PR180 を解消する rooted filesystem read capability（spec:
`docs/specs/rooted-read-capability.md`）は、IO-IMP-10 が archive の
私有に実装したものと同じ OS ルール（handle/fd 相対 traversal・
no-follow・identity 採取）を必要とする。同じルールを io-file に
複製すると恒久重複になり、一方で archive の write 側
（create/remove/staging）を公開 read API に混ぜると境界が崩れる。
「どこに read 面を置き、OS primitive をどう共有するか」を決める。

前提:

- Issue #304 設計は dir-relative primitive の他 backend との
  共通化を Deferred Design Concern として記録済み（成熟後に検討
  する条件が揃った）。
- `ruviform-io-file` はすでに `libc`/`windows-sys` の OS adapter
  を持つ最下位 file I/O クレート。`ruviform-io-archive` →
  `ruviform-io-file` は下向き依存として合法。
- cap-std 等の新依存は未承認。新 crate は無条件には導入しない
  （共有 file 変更を最小化する並行作業制約もある）。

## Decision Drivers

- 同一 OS ルールの authoritative 実装を一箇所に置く（恒久重複禁止）。
- read capability は最小 read-only 面（write op を公開しない）。
- archive staging の公開契約・session/records 非回帰。
- 層構造（下向き依存のみ）と backend 型非露出の規約。
- 並行 worker との共有 file（workspace members・lock・index）衝突
  の最小化。

## Considered Options

1. **io-file `rooted` 公開 module + `#[doc(hidden)] pub` 共有 backend**
   （本選択）— `ruviform-io-file` に read-only 公開面 `rooted` と、
   crate 内部共有の `vfs` 層（handle-relative ops・`NodeIdentity`・
   unix/windows/unsupported backend・`#[cfg(test)]` mock）を置く。
   `vfs` は `#[doc(hidden)]` の「workspace 内部共有面」として
   `ruviform-io-archive` の extract backend が consume する
   （`RtContext::mint` の drivers-only 先例と同じ約束事）。
   archive の `extract/{unix,windows}.rs` は削除し、`fs.rs` の
   `PlatformExtractOps`/`StagingDir`/`StagingFile`/`StagingRoot`
   trait は session 向け形を保ったまま vfs 呼び出しの薄い
   adapter にする。
2. **新規内部 crate `ruviform-io-vfs`** — primitive を独立
   crate として置き、io-file / io-archive が双方依存する。
   layering は最も明示的だが、workspace member・crate README・
   index・lock の共有 file 変更が増え、現状の利用者は 2 クレート
   のみでコストが見合わない。3 つ目の消費者や独立版管理が必要に
   なった時点で昇格を再評価する（deferred）。
3. **archive の私有実装を read 側に複製** — 同一 OS ルールの
   恒久重複であり仕様制約違反。却下。
4. **io-archive 側に共有面を置く** — io-file → io-archive は
   上位層への逆依存になり不可。却下。

## Decision Outcome

Option 1 を採用する。

- `ruviform-io-file` に `rooted` 公開 module を追加（read-only）。
- `ruviform-io-file::vfs`（`#[doc(hidden)]`）を
  `ruviform-io-file` と `ruviform-io-archive` の両方が使う
  workspace 内部共有 backend とし、handle/fd 相対・no-follow・
  identity・unix/windows/unsupported の OS ルールを一箇所に集約
  する。公開契約ではないため `#[doc(hidden)]` + 先頭 doc で
  「ruviform-io-archive staging backend 専用・semver 対象外」を
  明記する。
- `ruviform-io-archive` は `ruviform-io-file` に path 依存し、
  extract の OS backend を vfs 呼び出しに置き換える
  （session/records/cleanup の公開挙動は不変）。
- Option 2（分離 crate）は vfs の消費者が増えた時点で再評価する。

## Consequences

- 良い: OS ルールが一箇所、追加依存・新 crate なし、archive の
  非回帰が既存試験で守れる、共有 file 変更が最小。
- 悪い・受容する: `#[doc(hidden)]` は技術的には到達可能な API で、
  悪用は契約外とはいえ防げない（`publish = false` と doc 明記で
  緩和）。io-file が「staged publish + rooted read + 内部共有
  primitive」の複合責務になるが、いずれも OS file primitive の
  責務として一貫する。
