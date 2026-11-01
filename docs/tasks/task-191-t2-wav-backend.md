---
id: "TASK-034Qhxc5D3P3En9XfxxPSm"
type: "task"
status: "done"
title: "Issue 191 T2: ruviform-io-wav crate (encoder, muxer, capabilities)"
created: "2026-11-01"
updated: "2026-11-01"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
  implemented-by:
    - "docs/impl/ir/issue-191-pcm-wav-encode-mux-backend-with-mediasink-stagedoutput.md"
  depends-on:
    - "docs/plans/issue-191-pcm-wav-output-backend.md"
    - "docs/tasks/task-191-t1-media-io-sink-staged.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-191-pcm-wav-output-backend.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 191 T2: ruviform-io-wav crate (encoder, muxer, capabilities)

## Work

Create the `ruviform-io-wav` backend crate: `wav.rs` (RIFF/WAVE LE serialization, WAVEFORMATEX/EXTENSIBLE), `convert.rs` (f32→wire sample conversion per declared policy), `encoder.rs` (`WavEncoder: Encoder` — interleave, packetize, drain), `muxer.rs` (`WavMuxer<S: MediaSink>: Muxer` — header, data append, backpatch `finalize()` → `WavReceipt`), `lib.rs` (`capabilities()`, `wav_capabilities()` → `WavCapabilities`, `make_encoder`, `make_muxer` → `Unsupported`, `open_muxer`). Wire the workspace (root `Cargo.toml` members, `Cargo.lock`, `crates/README.md`, crate README).

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-191-pcm-wav-output-backend.md`
- `docs/designs/issue-191-pcm-wav-output-backend.md`
- `docs/specs/issue-191-pcm-wav-output-backend.md`

## Dependencies

- `docs/tasks/task-191-t1-media-io-sink-staged.md` (needs `MediaSink`)

## Affected Files

- `crates/ruviform-io-wav/` (new crate: `Cargo.toml`, `README.md`, `src/{lib,wav,convert,encoder,muxer}.rs`)
- `Cargo.toml` (root `members`), `Cargo.lock`, `crates/README.md`

## Done When

- [x] `ruviform-io-wav` compiles; `WavEncoder`/`WavMuxer`/`open_muxer`/`capabilities`/`wav_capabilities` public
- [x] Capability set matches spec (MUX|ENCODE, Pcm only); unsupported inputs → `ErrorKind::Unsupported`
- [x] `mise run check` + `lint` green

## Verification

- [x] `cargo check -p ruviform-io-wav` + `cargo clippy -p ruviform-io-wav -- -D warnings` green
- [x] Workspace `mise run check` green; `Cargo.lock` regenerated
