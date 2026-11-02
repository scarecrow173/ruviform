---
id: "ADR-YPgjjmV3ZLSeoH45bD5llz"
type: "adr"
status: "accepted"
title: "Issue #198 sequential offline render placement (ruviform-audio::render) + capability/staging contracts"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/198"
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/discovery/issue-198-sequential-offline-render.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-198-sequential-offline-render.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/offline-determinism-scope.md"
    - "docs/adr/m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
    - "docs/adr/transport-span-scheduler-applied-receipt.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-198-sequential-offline-render.md"
    - "docs/discovery/issue-198-sequential-offline-render.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ADR — Issue #198 placement + capability/staging contracts

## Status

Accepted (2026-11-02)

## Context

Issue #198 needs a sequential stateful render driver that uses the typed runtime (`ruviform-audio`), the transport scheduler (`ruviform-media-core`), and the media output lifecycle (`ruviform-media-io`), with job/cancellation primitives (`ruviform-exec`). The layer baseline forbids `ruviform-exec → ruviform-media-core`, so the driver cannot live beside `OfflineDriver`.

## Considered Options

- **Option A — `ruviform_audio::render` module (chosen)**: driver inside the audio engine crate, which owns `RtRuntime`/`GraphProcessor` and already depends on `media-core` (transport) and may depend on primitives-tier `media-io`/`exec`.
- **Option B — `ruviform-exec` beside `OfflineDriver`**: rejected — the layer table forbids `exec → media-core`, so transport semantics could not be expressed without duplicating the scheduler contract.
- **Option C — new crate `ruviform-offline`**: rejected — adds a new layer-table row and index churn for what is fundamentally an audio-engine RT-path concern; extraction remains possible later without API change.

## Decision

1. **Placement**: new public module `ruviform_audio::render` inside the audio engine crate — the engine owns the RT path, the `GraphProcessor` contract, and already depends on `media-core`/`graph`/`rt`. Two new dependency edges: `ruviform-audio → ruviform-media-io` and `ruviform-audio → ruviform-exec` (both primitives-tier from the engine's view; the layer table permits engine → foundations/primitives).
2. **Capability seam**: additive defaulted `GraphProcessor::offline_support() -> OfflineRenderSupport` (`Unknown`/`Declared`/`Verified`). Exact Render preflight requires `Verified` from every installed processor; `Unknown`/`Declared` are rejected with a typed preflight error. This is the mock/plugin separation point — the real-plugin gate (Issue #196 `ProcessConfig`/transport capability) plugs in behind `Verified` without changing the render API.
3. **Instance independence**: `PreparedRender` owns a *separate* `GraphRuntime`+`RtRuntime` compiled from the caller's factory; no processor/state is shared with any Live runtime. There is no "clone a live processor" operation — Live-independence is guaranteed by construction (fresh `New` outcomes).
4. **Staging contract**: the driver owns `StagedOutput` end-to-end. Muxer construction goes through `FnOnce(&mut dyn MediaSink) -> Result<M>` so tests can interpose fault-injecting sinks (`MediaSink` is object-safe-compatible: `&mut dyn MediaSink` itself implements `MediaSink`). Publish is a `FnOnce(StagedOutput, PublishOptions) -> Result<()>` closure defaulting to `StagedOutput::publish` — the injection point for publish-stage failure classes (sync/close) whose observable outcome (typed error + destination untouched + temp cleaned) is identical to a real failure.
5. **No degradation**: unsupported capability, transport fault, rejected command, or undeliverable span is a typed `RenderError` — never a silent degraded render reported as Exact.
6. **`OfflineDriver` unchanged**: the existing independent-chunk API keeps its documented determinism scope; the two drivers coexist as separate guarantees.

## Consequences

- `ruviform-audio` gains `ruviform-media-io` + `ruviform-exec` deps; dev-deps gain `ruviform-io-wav`/`ruviform-io-symphonia`/`ruviform-media-reader` for end-to-end tests.
- `ruviform-exec` gains one additive method `Progress::bump_by(n)` — no breaking change.
- Consumers (Synvelis) drive renders via `PreparedRender` + `RenderTransport` + `RenderOutput` and supply their own encoder/muxer/publish policy; AudioLens is unaffected (no API it uses changes).
- Deferred: real-plugin `Verified` attestation (#196), resume/retry policy (consumer), SRC/dither kernels (caller-provided encoder).
