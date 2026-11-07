//! The two-state component walk: TEXTUAL canonical-prefix tracking
//! for absolute link targets plus the FD/handle-relative walk that
//! actually touches the filesystem.
//!
//! Invariants (design §resolve):
//!
//! - `chain[0]` is always the capability root handle; every other
//!   entry is a *real directory* handle — links are never pushed.
//! - `..` pops resolved ancestors (kernel post-substitution
//!   semantics — suffix-fold is never used); popping below the
//!   capability-root floor is `EscapesRoot`.
//! - Absolute link targets restart the walk in TEXTUAL mode
//!   (`virtual` tracks the target position against the canonical
//!   root components). `virtual == R` re-enters FD at the root
//!   handle; a `virtual` that stops being a prefix of `R`, or an
//!   FD-floor `..`, is *tainted* — and tainted is sticky, so the
//!   outcome is decided at that moment: `EscapesRoot`, returned
//!   immediately (the design's "tainted wins" rule makes any
//!   earlier `NotFound`/`LoopDetected` impossible by construction).
//! - Budgets: `hops` (link expansions → `LoopDetected`), `total`
//!   (processed components → `LimitExceeded`), `chain.len()`
//!   (`max_open_dirs` → `LimitExceeded`), locator depth
//!   (`max_depth` → `LimitExceeded`).

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::sync::Arc;

use crate::error::{FileError, FileErrorKind};
use crate::vfs::{VfsDir, VfsKind, VfsProbe};

use super::{FollowMode, RelativePath, RootShared};

/// The resolve output consumed by the public API leaf rules
/// (boxed variants — `VfsProbe` payloads are large).
pub(super) enum Resolved {
    /// Resolved position is a directory.
    Dir(Box<ResolvedDir>),
    /// Resolved position is a non-directory leaf object.
    Leaf(Box<ResolvedLeaf>),
}

/// `Resolved::Dir` payload: the fully resolved ancestor chain —
/// `chain[0]` is the capability root and `chain.last()` the
/// resolved directory (self, a real dir, or a link that resolved
/// to a dir).
pub(super) struct ResolvedDir {
    pub chain: Vec<Arc<dyn VfsDir>>,
    /// The named leaf's own no-follow probe when the leaf name
    /// was a link (`via_symlink` report); `None` for `""`/plain
    /// dirs.
    pub leaf_probe: Option<VfsProbe>,
}

/// `Resolved::Leaf` payload.
pub(super) struct ResolvedLeaf {
    /// The directory handle the leaf name lives in.
    pub dir: Arc<dyn VfsDir>,
    /// The resolved leaf name inside `dir` — the name
    /// `open_regular_leaf` opens.
    pub name: OsString,
    /// Probe of the resolved object (a followed link's target).
    pub probe: VfsProbe,
    /// Probe of the *named* leaf itself (a link under
    /// `WithinRoot`, else the same object).
    pub leaf_probe: VfsProbe,
}

fn err(kind: FileErrorKind) -> FileError {
    FileError::from_kind(kind)
}

fn map_io(e: std::io::Error) -> FileError {
    super::ops_error(e)
}

fn is_dotdot(comp: &OsStr) -> bool {
    comp == ".."
}

fn is_dot(comp: &OsStr) -> bool {
    comp == "." || comp.is_empty()
}

/// The mode of the component walk.
enum Mode {
    /// Tracking the target textually against the canonical root
    /// components — no filesystem touches (the position may be
    /// outside the capability).
    Textual { virtual_comps: Vec<OsString> },
    /// Handle-relative walk inside the capability.
    Fd,
}

/// Resolves `rel` from `start_chain` (capability root at index 0,
/// `self` at the end) under `mode`, honoring `shared` budgets.
pub(super) fn resolve(
    start_chain: &[Arc<dyn VfsDir>],
    rel: &RelativePath,
    mode: FollowMode,
    shared: &RootShared,
) -> Result<Resolved, FileError> {
    if u32::try_from(rel.depth()).unwrap_or(u32::MAX) > shared.options.max_depth {
        return Err(err(FileErrorKind::LimitExceeded));
    }
    let mut chain: Vec<Arc<dyn VfsDir>> = start_chain.to_vec();
    let mut queue: VecDeque<OsString> = rel
        .components()
        .map(OsString::from)
        .collect::<VecDeque<_>>();
    let mut hops = 0u32;
    let mut total = 0u32;
    let mut walk = Mode::Fd;
    // The no-follow probe of a leaf-position link expanded under
    // `WithinRoot` — the eventual result's `leaf_probe`.
    let mut leaf_link_probe: Option<VfsProbe> = None;

    while let Some(comp) = queue.pop_front() {
        total += 1;
        if total > shared.options.max_components_total {
            return Err(err(FileErrorKind::LimitExceeded));
        }
        let is_leaf = queue.is_empty();
        match &mut walk {
            Mode::Textual { virtual_comps } => {
                if is_dotdot(&comp) {
                    virtual_comps.pop();
                } else if !is_dot(&comp) {
                    virtual_comps.push(comp);
                }
                if comps_eq(virtual_comps, &shared.canonical_comps) {
                    // The target reached the canonical root — resume
                    // the handle-relative walk from the root handle.
                    walk = Mode::Fd;
                    chain.truncate(1);
                } else if !is_prefix_of(virtual_comps, &shared.canonical_comps) {
                    // Left the canonical root prefix — tainted is
                    // sticky, so EscapesRoot is already deterministic.
                    return Err(err(FileErrorKind::EscapesRoot));
                }
            }
            Mode::Fd => {
                if let Some(done) = fd_component(
                    &comp,
                    is_leaf,
                    mode,
                    &mut chain,
                    &mut queue,
                    &mut walk,
                    &mut hops,
                    &mut leaf_link_probe,
                    shared,
                )? {
                    return Ok(done);
                }
            }
        }
    }

    match walk {
        Mode::Fd => Ok(Resolved::Dir(Box::new(ResolvedDir {
            chain,
            leaf_probe: leaf_link_probe,
        }))),
        // An absolute target that never re-entered the canonical
        // root prefix ends TEXTUAL → EscapesRoot.
        Mode::Textual { .. } => Err(err(FileErrorKind::EscapesRoot)),
    }
}

/// One handle-relative component step.
///
/// Returns `Ok(Some(..))` when the walk terminates with a leaf
/// result, `Ok(None)` to continue the loop, `Err` on failure. `.`
/// is skipped; `..` pops the resolved ancestor chain (kernel
/// post-substitution semantics) and is `EscapesRoot` at the
/// capability-root floor.
#[expect(clippy::too_many_arguments)]
fn fd_component(
    comp: &OsStr,
    is_leaf: bool,
    mode: FollowMode,
    chain: &mut Vec<Arc<dyn VfsDir>>,
    queue: &mut VecDeque<OsString>,
    walk: &mut Mode,
    hops: &mut u32,
    leaf_link_probe: &mut Option<VfsProbe>,
    shared: &RootShared,
) -> Result<Option<Resolved>, FileError> {
    if is_dot(comp) {
        return Ok(None);
    }
    if is_dotdot(comp) {
        if chain.len() > 1 {
            chain.pop();
        } else {
            // `..` at the capability-root floor leaves the
            // root → tainted → deterministic EscapesRoot.
            return Err(err(FileErrorKind::EscapesRoot));
        }
        return Ok(None);
    }
    let top = chain.last().expect("chain is never empty");
    let probe = top.probe(comp).map_err(map_io)?;
    match probe.kind {
        VfsKind::Dir => {
            if u32::try_from(chain.len()).unwrap_or(u32::MAX) >= shared.options.max_open_dirs {
                return Err(err(FileErrorKind::LimitExceeded));
            }
            let dir = top.open_dir(comp).map_err(map_io)?;
            chain.push(dir.into());
            Ok(None)
        }
        VfsKind::Symlink => {
            if mode == FollowMode::NoFollow {
                if is_leaf {
                    return Ok(Some(Resolved::Leaf(Box::new(ResolvedLeaf {
                        dir: chain.last().expect("chain is never empty").clone(),
                        name: comp.to_os_string(),
                        probe,
                        leaf_probe: probe,
                    }))));
                }
                return Err(err(FileErrorKind::NotDirectory));
            }
            *hops += 1;
            if *hops > shared.options.max_hops {
                return Err(err(FileErrorKind::LoopDetected));
            }
            let raw = top.read_link(comp).map_err(map_io)?;
            if is_leaf && leaf_link_probe.is_none() {
                *leaf_link_probe = Some(probe);
            }
            match split_target(&raw) {
                Target::Terminal(kind) => Err(err(kind)),
                target => {
                    splice(target, queue, walk, chain, shared);
                    Ok(None)
                }
            }
        }
        VfsKind::File | VfsKind::Other => {
            if !is_leaf {
                return Err(err(FileErrorKind::NotDirectory));
            }
            Ok(Some(Resolved::Leaf(Box::new(ResolvedLeaf {
                dir: chain.last().expect("chain is never empty").clone(),
                name: comp.to_os_string(),
                probe,
                leaf_probe: leaf_link_probe.take().unwrap_or(probe),
            }))))
        }
    }
}

/// A classified link target.
enum Target {
    /// Relative empty (`.`) — resolves to the containing directory.
    Empty,
    /// Relative components (`..` kept, `.`/empty dropped).
    Relative(Vec<OsString>),
    /// Absolute components normalized to the same shape as
    /// `RootShared::canonical_comps` (may be empty for `"/"`).
    Absolute(Vec<OsString>),
    /// A target that fails deterministically without any resolution.
    Terminal(FileErrorKind),
}

/// Splices a classified link target into the walk. Relative targets
/// keep the current chain; absolute targets reset to the capability
/// root — TEXTUAL mode for prefix tracking, or directly FD (chain
/// truncated to the root handle) when the canonical root is the
/// degenerate `"/"`/`\\`-empty form with no components to match.
fn splice(
    target: Target,
    queue: &mut VecDeque<OsString>,
    walk: &mut Mode,
    chain: &mut Vec<Arc<dyn VfsDir>>,
    shared: &RootShared,
) {
    let comps = match target {
        Target::Empty => return,
        Target::Relative(comps) => comps,
        Target::Absolute(comps) => {
            if shared.canonical_comps.is_empty() {
                // Degenerate root (e.g. `/`): everything is inside —
                // restart the FD walk at the root handle.
                chain.truncate(1);
            } else {
                *walk = Mode::Textual {
                    virtual_comps: Vec::new(),
                };
            }
            comps
        }
        Target::Terminal(_) => unreachable!("terminal targets return before splicing"),
    };
    for comp in comps.into_iter().rev() {
        queue.push_front(comp);
    }
}

/// Classifies and splits a raw link target per the OS rules.
///
/// unix: `""` → `NotFound`; leading `/` → absolute; components split
/// on `/` with empty/`.` dropped, `..` kept.
#[cfg(unix)]
fn split_target(raw: &OsStr) -> Target {
    use std::os::unix::ffi::OsStrExt;
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return Target::Terminal(FileErrorKind::NotFound);
    }
    let absolute = bytes.first() == Some(&b'/');
    let comps: Vec<OsString> = bytes
        .split(|&b| b == b'/')
        .filter(|c| !c.is_empty() && *c != b".")
        .map(|c| OsStr::from_bytes(c).to_os_string())
        .collect();
    if absolute {
        Target::Absolute(comps)
    } else if comps.is_empty() {
        Target::Empty
    } else {
        Target::Relative(comps)
    }
}

/// Windows: namespace-bridge the raw reparse substitute name
/// (design table) then split on both separators:
///
/// - `\??\X:\…`, `\\?\X:\…`, `\\.\X:\…` → `X:\…` absolute
/// - `\??\UNC\s\sh\…`, `\\?\UNC\…` → UNC absolute
/// - `\\srv\sh\…` → UNC absolute; `X:\…` → DOS absolute
/// - `\??\Volume{guid}\…` → `Unsupported` (no drive mapping)
/// - `\foo\…`, `C:foo` → `EscapesRoot` (volume undeterminable)
/// - `""` → `NotFound`; everything else → relative
#[cfg(windows)]
fn split_target(raw: &OsStr) -> Target {
    let s = raw.to_string_lossy();
    if s.is_empty() {
        return Target::Terminal(FileErrorKind::NotFound);
    }
    let (rest, absolute): (&str, bool) = if let Some(r) = s
        .strip_prefix("\\??\\")
        .or_else(|| s.strip_prefix("\\\\?\\"))
        .or_else(|| s.strip_prefix("\\\\.\\"))
    {
        if r.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("UNC\\"))
            || r.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("UNC/"))
        {
            // `\??\UNC\srv\sh\…` → UNC absolute.
            (&r[4..], true)
        } else if r
            .get(..7)
            .is_some_and(|p| p.eq_ignore_ascii_case("Volume{"))
        {
            return Target::Terminal(FileErrorKind::Unsupported);
        } else {
            (r, true)
        }
    } else if let Some(r) = s.strip_prefix("\\\\") {
        // `\\srv\sh\…` UNC absolute.
        (r, true)
    } else if s.starts_with('\\') {
        // `\foo` — same-drive absolute, volume undeterminable.
        return Target::Terminal(FileErrorKind::EscapesRoot);
    } else {
        let b = s.as_bytes();
        if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            if b.len() >= 3 && (b[2] == b'\\' || b[2] == b'/') {
                (s.as_ref(), true)
            } else {
                // `C:foo` drive-relative — volume undeterminable.
                return Target::Terminal(FileErrorKind::EscapesRoot);
            }
        } else {
            (s.as_ref(), false)
        }
    };
    let comps: Vec<OsString> = rest
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
        .map(OsString::from)
        .collect();
    if absolute {
        Target::Absolute(comps)
    } else if comps.is_empty() {
        Target::Empty
    } else {
        Target::Relative(comps)
    }
}

#[cfg(not(any(unix, windows)))]
fn split_target(raw: &OsStr) -> Target {
    let s = raw.to_string_lossy();
    if s.is_empty() {
        return Target::Terminal(FileErrorKind::NotFound);
    }
    let absolute = s.starts_with('/');
    let comps: Vec<OsString> = s
        .split('/')
        .filter(|c| !c.is_empty() && *c != ".")
        .map(OsString::from)
        .collect();
    if absolute {
        Target::Absolute(comps)
    } else if comps.is_empty() {
        Target::Empty
    } else {
        Target::Relative(comps)
    }
}

/// Whether `a` is a component-wise prefix of `b`. unix: exact
/// component equality; windows: case-insensitive (NT
/// `OBJ_CASE_INSENSITIVE` name lookup — the per-directory
/// case-sensitive FS divergence is a documented in-root limit).
#[cfg(unix)]
fn is_prefix_of(a: &[OsString], b: &[OsString]) -> bool {
    a.len() <= b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

#[cfg(unix)]
fn comps_eq(a: &[OsString], b: &[OsString]) -> bool {
    a == b
}

#[cfg(windows)]
fn comp_eq(x: &OsStr, y: &OsStr) -> bool {
    x.to_string_lossy().to_lowercase() == y.to_string_lossy().to_lowercase()
}

#[cfg(windows)]
fn is_prefix_of(a: &[OsString], b: &[OsString]) -> bool {
    a.len() <= b.len() && a.iter().zip(b.iter()).all(|(x, y)| comp_eq(x, y))
}

#[cfg(windows)]
fn comps_eq(a: &[OsString], b: &[OsString]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| comp_eq(x, y))
}

#[cfg(not(any(unix, windows)))]
fn is_prefix_of(a: &[OsString], b: &[OsString]) -> bool {
    a.len() <= b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

#[cfg(not(any(unix, windows)))]
fn comps_eq(a: &[OsString], b: &[OsString]) -> bool {
    a == b
}
