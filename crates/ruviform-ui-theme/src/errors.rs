//! Error classifications for the token pipeline.
//!
//! `TokenError` covers a single document's structural/type problems
//! (parse + validate phase). `ResolveError` covers merge semantics
//! (precedence, mode scope, alias resolution) across documents.

use std::fmt;

/// Structural or type-level problem inside one token document.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum TokenError {
    /// Input is not well-formed JSON.
    MalformedJson {
        /// 1-based line reported by the parser.
        line: usize,
        /// 1-based column reported by the parser.
        column: usize,
        /// Parser message.
        message: String,
    },
    /// Root `$extensions."dev.ruviform.dtcg-profile"` block is absent.
    MissingProfile,
    /// `profile` value is not `rvf-dtcg-profile.v1`.
    InvalidProfile {
        /// The profile string found in the document.
        found: String,
    },
    /// `layer` value is not one of `primitive|semantic|component|product`.
    UnknownLayer {
        /// The layer string found in the document.
        found: String,
    },
    /// `$type` value is not in the adopted v1 type set.
    UnknownTokenType {
        /// Token path, e.g. `color.primary`.
        path: String,
        /// The `$type` string found.
        found: String,
    },
    /// Value shape or range violates the declared `$type` (also used for a
    /// missing effective `$type` and malformed alias/mode values).
    InvalidValue {
        /// Token path or document field path.
        path: String,
        /// Human-readable reason.
        reason: String,
    },
    /// A `$`-prefixed key that is not part of the v1 profile was found
    /// (reserved for future profile versions).
    ReservedKey {
        /// Token/group path where the key appeared.
        path: String,
        /// The reserved key.
        key: String,
    },
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson {
                line,
                column,
                message,
            } => write!(f, "malformed JSON at {line}:{column}: {message}"),
            Self::MissingProfile => {
                write!(f, "missing $extensions.\"dev.ruviform.dtcg-profile\"")
            }
            Self::InvalidProfile { found } => {
                write!(
                    f,
                    "invalid dtcg profile {found:?} (expected \"rvf-dtcg-profile.v1\")"
                )
            }
            Self::UnknownLayer { found } => write!(f, "unknown document layer {found:?}"),
            Self::UnknownTokenType { path, found } => {
                write!(f, "unknown $type {found:?} at {path}")
            }
            Self::InvalidValue { path, reason } => write!(f, "invalid value at {path}: {reason}"),
            Self::ReservedKey { path, key } => {
                write!(f, "reserved $-prefixed key {key:?} at {path}")
            }
        }
    }
}

impl std::error::Error for TokenError {}

/// Merge-semantics problem across documents.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// A document failed parse/validate.
    Document(TokenError),
    /// The requested mode is not declared by a mode-scoped document.
    MissingMode {
        /// Document `id` that does not declare the mode.
        document: String,
        /// Requested mode.
        mode: String,
    },
    /// An override or alias target has a different `$type` than the base token.
    TypeMismatch {
        /// Token path.
        path: String,
        /// `$type` of the base (or referencing) token.
        base: String,
        /// `$type` of the override (or referenced) token.
        override_: String,
    },
    /// An alias references a token path that does not exist after merge.
    UnknownReference {
        /// Token path containing the alias.
        path: String,
        /// Referenced path that does not exist.
        reference: String,
    },
    /// Alias resolution found a reference cycle.
    AliasCycle {
        /// Token paths forming the cycle, in order.
        cycle: Vec<String>,
    },
    /// The same `(layer, id)` document identity was supplied twice.
    DuplicateSource {
        /// Duplicated document `id`.
        id: String,
    },
    /// `resolve` was called with no documents.
    EmptyDocuments,
    /// A [`crate::ThemeRegistry`] lookup named a theme that was never registered.
    UnknownTheme {
        /// Requested theme name.
        name: String,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(e) => write!(f, "document error: {e}"),
            Self::MissingMode { document, mode } => {
                write!(f, "mode {mode:?} is not declared by document {document:?}")
            }
            Self::TypeMismatch {
                path,
                base,
                override_,
            } => write!(
                f,
                "type mismatch at {path}: base {base:?} vs override {override_:?}"
            ),
            Self::UnknownReference { path, reference } => {
                write!(f, "unknown reference {reference:?} at {path}")
            }
            Self::AliasCycle { cycle } => write!(f, "alias cycle: {}", cycle.join(" -> ")),
            Self::DuplicateSource { id } => {
                write!(f, "duplicate document source {id:?}")
            }
            Self::EmptyDocuments => write!(f, "no token documents supplied"),
            Self::UnknownTheme { name } => write!(f, "unknown theme {name:?}"),
        }
    }
}

impl std::error::Error for ResolveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TokenError> for ResolveError {
    fn from(e: TokenError) -> Self {
        Self::Document(e)
    }
}
