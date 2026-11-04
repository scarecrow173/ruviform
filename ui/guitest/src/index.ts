/**
 * Issue #311 — `ui/guitest`: shared deterministic GUI-test
 * infrastructure for the `rvf.ipc.v1` / `rvf://data` / Binding
 * contracts. One versioned fixture corpus drives Rust golden tests
 * and TypeScript conformance + replay lanes; the replay oracle is an
 * independent contract model, not recorded responses.
 */

export * from "./timing.ts";
export * from "./fixtureload.ts";
export * from "./ipcwire.ts";
export * from "./bindingwire.ts";
export * from "./jsonl.ts";
export * from "./recorder.ts";
export * from "./seed.ts";
export * from "./consumers.ts";
export * from "./oracle.ts";
export * from "./replay.ts";
