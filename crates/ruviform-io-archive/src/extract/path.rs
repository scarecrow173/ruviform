//! `ArchiveName` → [`SafeRelativePath`] validation (IO-IMP-10 path
//! policy).
//!
//! [`ArchiveName`] preserves stored name bytes untouched; this module is
//! the *only* validation entry point that decides whether a stored name
//! may name a file or directory inside an extraction staging tree.
//!
//! The secure default accepts **only names that mean the same relative
//! tree on every supported OS**. Ambiguous names are *rejected*, never
//! rewritten: `a\b` (a Windows separator but a legal unix filename),
//! `a:b` (a Windows ADS/drive separator but legal on unix), `C:`, UNC
//! roots, and Windows reserved device names are not "converted" —
//! accepting them would let one archive silently produce different
//! trees on different platforms.
//!
//! This is a *name* policy only. It cannot and does not prove
//! filesystem safety against concurrent modification — that is the
//! staging backend's job (handle-relative, no-follow, exclusive create).
//!
//! **RT contract:** non-RT only — validation allocates.

use std::fmt;

use unicode_normalization::UnicodeNormalization;

use crate::entry::ArchiveName;
use crate::error::{ArchiveError, ArchiveErrorKind};

/// Maximum byte length of a single path component (POSIX `NAME_MAX`).
const MAX_COMPONENT_BYTES: usize = 255;

/// Windows device names rejected regardless of case (the stem before
/// the first `.` is compared — `con.txt` is still `CON`).
const RESERVED_STEMS: &[&str] = &["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"];

/// Characters Windows forbids in file names (`:` and `\` get their own
/// rejection reasons earlier in the pipeline).
const FORBIDDEN_CHARS: &[char] = &['<', '>', '|', '"', '?', '*'];

/// A validated relative path inside an extraction staging tree.
///
/// Produced only by [`SafeRelativePath::new`]. Guarantees: valid UTF-8,
/// non-empty, `/`-separated components with no `.`/`..`, no `\` or `:`,
/// no Windows-forbidden characters or reserved device names, no
/// leading/trailing whitespace or trailing `.`, each component at most
/// 255 bytes.
///
/// **This type performs no filesystem checks.** Whether the staged tree
/// can be created safely under concurrent modification is the staging
/// backend's contract — this type only guarantees the *name* cannot
/// express traversal, alternate streams, or platform-divergent names.
///
/// **RT contract:** non-RT only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SafeRelativePath {
    /// Validated components, in order (depth = `components.len()`).
    components: Vec<String>,
    /// `a/` directory-marker form preserved for display fidelity.
    had_trailing_separator: bool,
}

impl SafeRelativePath {
    /// Validates a stored archive name into a safe relative path.
    ///
    /// A single trailing `/` is accepted as a directory marker and
    /// stripped; every other empty component (`a//b`, `/a`, `a/`) —
    /// beyond that one marker — is rejected, as are `.`/`..`
    /// components, absolute prefixes, `\`, `:`, Windows reserved names,
    /// trailing `.`/whitespace, control characters, NUL, and non-UTF-8
    /// names.
    ///
    /// **RT contract:** non-RT only — allocates.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for every policy rejection and
    /// `LimitExceeded` when a component exceeds 255 bytes (depth and
    /// total-name budgets are enforced by the extraction session).
    pub fn new(name: &ArchiveName) -> Result<Self, ArchiveError> {
        let Some(text) = name.to_str() else {
            return Err(reject(
                "entry name is not valid UTF-8; extraction requires UTF-8 names",
            ));
        };
        if text.is_empty() {
            return Err(reject("entry name is empty"));
        }
        if text.contains('\0') {
            return Err(reject("entry name contains a NUL byte"));
        }
        if text.contains('\\') {
            return Err(reject(
                "entry name contains `\\` (a Windows separator); rejecting rather than reinterpreting",
            ));
        }
        if text.contains(':') {
            return Err(reject(
                "entry name contains `:` (drive/ADS separator); rejecting rather than reinterpreting",
            ));
        }
        if text.starts_with('/') {
            return Err(reject("entry name is absolute (starts with `/`)"));
        }
        let had_trailing_separator = text.ends_with('/');
        let stripped = text.strip_suffix('/').unwrap_or(text);
        let mut components = Vec::new();
        for part in stripped.split('/') {
            if part.is_empty() {
                return Err(reject(
                    "entry name contains an empty component (`//`, leading, or repeated `/`)",
                ));
            }
            validate_component(part)?;
            components.push(part.to_owned());
        }
        if components.is_empty() {
            return Err(reject(
                "entry name resolves to the staging root itself (`/` or empty)",
            ));
        }
        Ok(Self {
            components,
            had_trailing_separator,
        })
    }

    /// The validated components, in order.
    ///
    /// **RT contract:** non-RT only.
    pub fn components(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.components.iter().map(String::as_str)
    }

    /// Number of components (depth below the staging root, ≥ 1).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.components.len()
    }

    /// Component `index`, or `None` when out of range.
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn component(&self, index: usize) -> Option<&str> {
        self.components.get(index).map(String::as_str)
    }

    /// Whether the stored name ended in `/` (a directory marker).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub const fn had_trailing_separator(&self) -> bool {
        self.had_trailing_separator
    }

    /// Byte length of the joined relative path (`a/b/c`).
    ///
    /// **RT contract:** non-RT only.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.components.iter().map(String::len).sum::<usize>() + self.components.len() - 1
    }

    /// The canonical collision key: every component rendered in NFC
    /// then full-Unicode-lowercased. Two *different* stored names that
    /// would resolve to the same file on a case-insensitive or
    /// normalizing filesystem fold to the same key.
    ///
    /// This is a conservative approximation of OS-specific
    /// filename-equivalence rules, not an exact model of any one of
    /// them — collisions are reported honestly rather than resolved.
    pub(crate) fn canonical_key(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|component| {
                component
                    .nfc()
                    .flat_map(char::to_lowercase)
                    .collect::<String>()
            })
            .collect()
    }
}

impl fmt::Display for SafeRelativePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.components.join("/"))
    }
}

fn validate_component(part: &str) -> Result<(), ArchiveError> {
    if part == "." || part == ".." {
        return Err(reject("entry name contains `.`/`..` traversal"));
    }
    if part.len() > MAX_COMPONENT_BYTES {
        return Err(ArchiveError::new(
            ArchiveErrorKind::LimitExceeded,
            format!(
                "entry name component exceeds {MAX_COMPONENT_BYTES} bytes ({})",
                part.len()
            ),
        ));
    }
    if part.starts_with(char::is_whitespace) || part.ends_with(char::is_whitespace) {
        return Err(reject(
            "entry name component has leading/trailing whitespace (Windows strips it)",
        ));
    }
    if part.ends_with('.') {
        return Err(reject(
            "entry name component ends with `.` (Windows strips it)",
        ));
    }
    if part.chars().any(char::is_control) {
        return Err(reject("entry name contains control characters"));
    }
    if part.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
        return Err(reject(
            "entry name contains a character forbidden on Windows (`< > | \" ? *`)",
        ));
    }
    if is_reserved_stem(part) {
        return Err(reject(
            "entry name component is a Windows reserved device name",
        ));
    }
    Ok(())
}

/// Whether `part`'s stem (before the first `.`) matches a Windows
/// device name: `CON`, `PRN`, `AUX`, `NUL`, `CLOCK$`, `CONIN$`,
/// `CONOUT$`, `COM1`–`COM9`, `LPT1`–`LPT9`. Comparison is
/// ASCII-case-insensitive and normalizes the superscript digits
/// `¹²³` (which NTFS maps onto `123`).
fn is_reserved_stem(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part);
    let mut upper = String::with_capacity(stem.len());
    for c in stem.chars() {
        match c {
            '\u{00B9}' => upper.push('1'),
            '\u{00B2}' => upper.push('2'),
            '\u{00B3}' => upper.push('3'),
            other => upper.extend(other.to_uppercase()),
        }
    }
    if RESERVED_STEMS.contains(&upper.as_str()) {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = upper.strip_prefix(prefix)
            && rest.len() == 1
            && rest
                .chars()
                .next()
                .is_some_and(|d| ('1'..='9').contains(&d))
        {
            return true;
        }
    }
    false
}

fn reject(message: impl Into<std::borrow::Cow<'static, str>>) -> ArchiveError {
    ArchiveError::new(ArchiveErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::ArchiveName;

    fn path(raw: &str) -> Result<SafeRelativePath, ArchiveError> {
        SafeRelativePath::new(&ArchiveName::new(raw.as_bytes().to_vec()))
    }

    #[test]
    fn accepts_plain_relative_names() {
        for ok in [
            "a",
            "a/b/c.txt",
            "dir/",
            "a.b/c-d_e f/x",
            "日本語/ファイル.txt",
        ] {
            let p = path(ok).unwrap_or_else(|e| panic!("{ok:?} rejected: {e}"));
            assert!(p.depth() >= 1);
        }
        assert_eq!(path("a/b/c.txt").unwrap().depth(), 3);
        assert!(path("dir/").unwrap().had_trailing_separator());
        assert_eq!(path("dir/").unwrap().to_string(), "dir");
    }

    #[test]
    fn rejects_traversal_and_absolute() {
        for bad in [
            "..",
            "../a",
            "a/..",
            "a/../b",
            "./a",
            "a/.",
            "/a",
            "/",
            "a//b",
            "a//",
            "//a",
            "a/../../b",
        ] {
            assert!(path(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn rejects_windows_divergent_names() {
        for bad in [
            "a\\b",
            "a\\..\\x",
            "C:",
            "C:/a",
            "c\\a",
            "a:b",
            "a:c/d",
            "\\\\server\\share",
            "con",
            "CON",
            "con.txt",
            "nul.txt",
            "Com1",
            "lpt9.log",
            "aux",
            "PRN.dir",
            "com\u{00B9}",
            "clock$",
            "conin$",
            "a<",
            "a>",
            "a|",
            "a\"",
            "a?",
            "a*",
            "a ",
            " a",
            "a.",
            "..a /b",
            "a\u{0007}b",
        ] {
            assert!(path(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn rejects_non_utf8_and_nul() {
        assert!(SafeRelativePath::new(&ArchiveName::new(vec![0xFF, 0xFE])).is_err());
        assert!(SafeRelativePath::new(&ArchiveName::new(b"a\0b".to_vec())).is_err());
    }

    #[test]
    fn canonical_key_folds_case_and_nfc() {
        let a = path("Foo.TXT").unwrap();
        let b = path("foo.txt").unwrap();
        assert_eq!(a.canonical_key(), b.canonical_key());
        let nfc = path("é.txt").unwrap();
        let nfd = path("e\u{0301}.txt").unwrap();
        assert_eq!(nfc.canonical_key(), nfd.canonical_key());
    }
}
