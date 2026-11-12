---
id: "DESIGN-ubGFwmTpu82MiFMYl0RSVo"
type: "design"
status: "approved"
title: "Issue #582 設計 — ci 3 lane 失敗の修正方針(冷キャッシュ fetch / libc pin / moxcms 0.9.2 境界適応)"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/582"
    - "docs/specs/issue-582-ci-lane-failures.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-582-ci-lane-failures-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/designs/README.md"
  implements:
    - "docs/specs/issue-582-ci-lane-failures.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-582-ci-lane-failures.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-582-ci-lane-failures.md"
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
    - "docs/specs/issue-125-icc-lut-fixture-coverage.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-582-ci-lane-failures.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/572"
  defers: []
  deferred-by: []
---

# Issue #582 設計 — ci 3 lane 失敗の修正方針

## Overview

run 43270011255(sha `246e364e`)の 3 系統失敗を、それぞれ失敗が最初に誤った地点で修正する。(1) ui-gpui-verify は guard 前に `cargo fetch --locked` を置き offline metadata の前提(全 locked crate が registry に存在)を CI 側で保証する。(2) offscreen-verify は `Cargo.lock` の `libc` を gt-05 pin の 0.2.189 に戻す。(3) icc-verify は moxcms 0.9.2 の spec 準拠 fallback に対し、`IccTransform::new` が Issue #125 AC3 の missing-intent-tag reject を境界で再強制する。

## Failure 1: ui-gpui-verify 冷キャッシュ cargo metadata 失敗

- **原因**: `check-ui-driver-boundary` → `gui-boundary-guard --offline` → `cargo metadata --offline`。offline metadata は lockfile 全 crate( dev-dep 含む)が `~/.cargo/registry` に揃っていることを要求する。actions/cache の restore-keys prefix(`*-cargo-ui-gpui-`)が別 lockfile の cache を復元した冷状態では欠落があり、最初の未取得 crate(`aho-corasick v1.1.5`)で失敗した。
- **先例**: `consumer-verify.yml` 472-481 行が run 42348131128 の同一実障害を文書化し、候補 tree で `mise install --locked rust && mise exec -- cargo fetch --locked` を guard 前に置いている。
- **修正**: `ci.yml` `ui-gpui-verify` の「Restore cargo registry/git index cache」直後・sccache セットアップ前に `Fetch locked cargo dependencies` step(`run: mise exec -- cargo fetch --locked`)を追加。matrix の ubuntu/windows 両脚が同一 job 定義を共有するため 1 箇所で両脚をカバーする。mise task 側には入れない — guard task は `--offline` 検査であり「lockfile が fetch 可能で全依存が揃う」前提を CI step が明示的に作る方が責務が明確(consumer-verify と同型)。
- **非対象**: `ui-gpui-offscreen-verify` は `cargo test` が online fetch するため同問題を持たない。追加しない。

## Failure 2: ui-gpui-offscreen-verify ENOATTR / libc pin 喪失

- **原因**: `libc` が Cargo.lock で 0.2.191。`libc::ENOATTR` は libc 0.2.190 で Linux cfg から削除され、xattr 0.2.3(gpui `zed-async-tar` 依存)の Linux build が `E0425` で失敗。
- **drift 経緯**: gt-05 で `cargo update -p libc --precise 0.2.189` pin 済み( IR 記録)だったが、`4bcf1402`(PR #572、Issue #313 S2 windowed TestIo GPUI backend)の依存追加に伴う `Cargo.lock` 再生成で 0.2.191 に戻った。`git log` で `fd1d4d5f`=0.2.189 → `4bcf1402`=0.2.191 を確認。
- **修正**: `cargo update -p libc --precise 0.2.189` で lockfile のみ pin 復元。xattr 側の更新は upstream で ENOATTR 依存が解消されるまで待つ必要があり、pin が最小の整合的修正。pin の再喪失リスクは PR/IR に記録し、恒久策(constraint 化や xattr 更新追跡)は別 Issue 候補とする。

## Failure 3: icc-verify moxcms 0.9.2 drift

- **原因**: moxcms 0.9.2(2026-11-07 公開)で `get_device_to_pcs`/`get_pcs_to_device` に `lut.or(self.lut_*_perceptual.as_ref())` の fallback が追加された(ICCs.1:2022 §8.10.2 c) の実装 — intent 専用タグが無い場合 perceptual tag 0 を使う)。0.9.1 は intent タグ欠落で `None` を返し transform 生成が失敗していた。
- **契約**: Issue #125 spec AC3 は「A2B0 のみの profile が RelativeColorimetric で `InvalidValue("icc transform")`、Perceptual で受理」を要求。これは intent が LUT タグ選択を実際に制御することの裏付けであり、0.9.2 の fallback はこれを迂回する(A2B0-only + RC が perceptual 写像で黙って成功してしまう)。
- **選択: pin ではなく 0.9.2 適応**。根拠:
  1. 0.9.2 の fallback は ICC.1:2022 の規格準拠動作であり engine 側の退行ではない。pin(0.9.1 固定)は lockfile 再生成で容易に失われる — 本 Issue の libc pin 喪失がまさに同じ failure mode であり、pin は再発を先送りするだけ。
  2. ruviform 側の契約(missing intent tag → reject)は `IccTransform::new` が所有する境界の不変条件であり、engine の fallback 導入後も境界で再強制するのが責務的に正しい(engine error を `InvalidValue` に写像する既存構造と同じ層)。
  3. `Cargo.toml` の `moxcms = "0.9"` spec は変更不要。lock は 0.9.2 のまま。
- **実装**: `IccIntent` に `lacks_a_to_b_tag`/`lacks_b_to_a_tag` を追加(`rendering_intent()` と同じく exhaustive match で将来 variant 追加時に compile error → 意図的決定を強制)。`IccTransform::new` で RGB 判定後・executor bake 前に、src profile の A2B direction と dst profile の B2A direction について「いずれかの intent LUT タグを持つのに要求 intent のタグが無い」場合 `InvalidValue("icc transform")` を返す。LUT タグを全く持たない profile(matrix-shaper)は engine に委ねる — engine の `has_device_to_pcs_lut()`/`has_pcs_to_device_lut()` gate と同じ条件。
- **検証意図の維持**: 既存 `icc_transform_missing_intent_lut_tag_rejects` は無変更で pass(A2B0-only src + RC reject、B2A 全欠落 dst reject をそのまま検証)。dst 側 mirror として in-memory 生成の B2A0-only profile による `icc_transform_missing_intent_b2a_tag_rejects` を追加 — committed fixture を増やさず対称契約を固定。
- **fixture 影響なし**: `write_lut_entry` は 0.9.1/0.9.2 で同一のため `fix_mft1_tags` workaround は 0.9.2 でも必要かつ有効。committed `.icc` fixture は変更しない(全テストが 0.9.2 で parse・実行済み)。

## File map

- 変更: `.github/workflows/ci.yml`(ui-gpui-verify に fetch step 追加)、`Cargo.lock`(libc 0.2.189 pin)、`crates/ruviform-video-proc/src/icc.rs`(境界 check + dst-side test)
- 新規: doc-chain 5 件 + `docs/impl/ir/issue-582-ci-lane-failures.md`
- 再生成: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md`(管理 index)

## Risks

- `cargo fetch --locked` が lane 時間を伸ばす → lockfile 全体の .crate 取得のみで軽量。cache hit 時はほぼ no-op。
- 境界 check が engine 内部経路(katana vs 非 katana)の細部と厳密には一致しない可能性 → ruviform の公開契約(RGB profile のみ、IccIntent 2 variant)の範囲では `has_*_lut → intent tag 必須` が 0.9.1 の観測可能な reject 条件と一致。matrix-shaper + LUT tag 混在の exotic profile では 0.9.1 より厳しい reject になり得るが、契約上は「intent tag 欠落は reject」で一貫しており許容。
- libc/moxcms pin の将来の再 drift → 今回は lockfile pin(libc)と境界適応(moxcms)で対処。恒久的な constraint 管理は別 Issue 候補として PR に記録。
