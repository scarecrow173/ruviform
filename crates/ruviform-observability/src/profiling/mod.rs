//! Development/test allocation probe — Issue #290 (OBS-08) slice.
//!
//! This module provides a `GlobalAlloc` wrapper
//! ([`CountingAllocator`]) and thread-local measurement scopes
//! ([`AllocationScope`]/[`AllocationWindow`]) for investigating
//! allocation behavior during development and tests. It is only
//! compiled with the `allocation-probe` feature, which is **off by
//! default** so shipped builds carry no instrumentation and no
//! dependency cost.
//!
//! # What is measured
//!
//! - Per `GlobalAlloc` operation (`alloc`, `alloc_zeroed`, `dealloc`,
//!   `realloc`): call count, success/failure count, requested bytes.
//! - Live requested bytes (`current_bytes`) and the peak
//!   (`peak_bytes`) observed so far.
//!
//! Byte figures are *requested* bytes (`Layout::size()` /
//! `new_size`), summed with saturating arithmetic. They are **not**
//! RSS, heap occupancy, allocator-internal overhead, or an exact leak
//! count — an allocator may round sizes, keep free lists, or serve
//! small allocations from pools, none of which this probe sees.
//!
//! # Scope and coverage
//!
//! [`CountingAllocator::stats`] aggregates every thread that uses the
//! instance. [`AllocationScope`] counts only allocations performed on
//! the thread that entered it — other threads' allocations never
//! appear in a scope. A scope is thread-affine (`!Send`/`!Sync`) and
//! must be created on the measuring thread.
//!
//! # RT contract
//!
//! | Operation | Allocation | Lock | Panic | RT-suitable |
//! | --- | --- | --- | --- | --- |
//! | `CountingAllocator` `GlobalAlloc` methods | none added | none | none | yes* |
//! | `AllocationScope::new` / `measure` | one `Box` | none | on OOM abort | no |
//! | `AllocationScope::enter` / window drop | none | none | misuse only | yes* |
//! | `AllocationScope::stats` / `reset`, `window.stats` | none | none | none | yes* |
//!
//! \* "RT-suitable" means the probe adds no allocation, lock, or panic
//! beyond what the wrapped allocator `A` already does; `A`'s own RT
//! properties are unchanged and remain `A`'s responsibility. The
//! `GlobalAlloc` methods are also the functions a `#[global_allocator]`
//! install would run, so keeping them allocation-free matters for any
//! allocator consumer, not just RT threads.
//!
//! # Installing the allocator
//!
//! The crate never installs a global allocator. A host binary or test
//! does so explicitly:
//!
//! ```ignore
//! use ruviform_observability::profiling::CountingAllocator;
//! use std::alloc::System;
//!
//! #[global_allocator]
//! static ALLOC: CountingAllocator<System> = CountingAllocator::new(System);
//! ```
//!
//! `CountingAllocator::new` is `const`, so the `static` above works.
//! Note that a crate can only have one `#[global_allocator]`; test
//! binaries that already install one (for example
//! `ruviform-rt`'s test-only counter) cannot install a second.
//!
//! # Coexistence with `ruviform-rt::alloc_counter`
//!
//! `ruviform-rt` keeps its private test-only `alloc_counter`
//! (`Counting` over `System`, event counts only, no bytes, no public
//! API). It remains the lane for the RT allocation-budget tests in
//! `spsc`/`retire`/`snapshot`, which must not depend on an
//! observability feature. This module is the shared, feature-gated
//! implementation for development and cross-crate investigation; it
//! does not replace `alloc_counter`, and `alloc_counter` is not
//! migrated here. Migration of examples such as `play_sine`'s ad-hoc
//! counting is deferred until a third consumer or a broader-than-test
//! demand justifies touching those binaries (see
//! `docs/designs/issue-290-observability-allocation-probe.md`).
//!
//! # CPU sampling and non-RT profilers
//!
//! This probe counts allocation *events*; it does not sample the CPU.
//! For CPU-side profiling, run an external sampler (for example
//! `cargo instruments`, `perf`, or a sampling profiler) against a
//! **non-RT** build or test binary — RT threads must not pay sampler
//! or symbolization cost. Tracy or similar instrumentation belongs to
//! the non-RT layer: it may allocate, lock, and call into subscribers,
//! so it must never be invoked from the `GlobalAlloc` path or from
//! `AllocationScope` enter/exit. The probe's counters can be read from
//! non-RT code at any time; reading them never perturbs the measured
//! path.
//!
//! # Overhead
//!
//! Per allocator call the probe adds: two to four saturating atomic
//! RMWs for the global counters, plus a TLS head read and a short
//! chain walk per active scope (typically zero or one node). No heap
//! access, no formatting, no I/O. Expect single-digit to low-double-
//! digit nanoseconds per call on typical hardware; measure before
//! relying on the numbers for a specific target.

mod allocator;
mod scope;
mod stats;
#[cfg(test)]
mod testutil;

pub use allocator::CountingAllocator;
pub use scope::{AllocationScope, AllocationWindow};
pub use stats::{AllocationStats, OpStats};
