//! Named theme registry: maps a theme name to its document stack.
//!
//! The registry owns no resolution rules. Each lookup delegates to
//! [`ThemeResolver::resolve`], so the precedence, mode and alias semantics
//! stay in one place.

use std::collections::BTreeMap;

use crate::doc::TokenDocument;
use crate::errors::ResolveError;
use crate::resolve::ThemeResolver;
use crate::snapshot::ResolvedTheme;

/// Registry of named token-document stacks (e.g. one per Product Theme).
#[derive(Debug, Default)]
pub struct ThemeRegistry {
    themes: BTreeMap<String, Vec<TokenDocument>>,
}

impl ThemeRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace the document stack for `name`.
    pub fn register(&mut self, name: impl Into<String>, documents: Vec<TokenDocument>) {
        self.themes.insert(name.into(), documents);
    }

    /// Resolve the theme `name` for `mode`.
    ///
    /// # Errors
    ///
    /// `UnknownTheme` when `name` was never registered; otherwise any error
    /// [`ThemeResolver::resolve`] reports for the registered stack.
    pub fn resolve(&self, name: &str, mode: &str) -> Result<ResolvedTheme, ResolveError> {
        let documents = self
            .themes
            .get(name)
            .ok_or_else(|| ResolveError::UnknownTheme {
                name: name.to_owned(),
            })?;
        ThemeResolver::resolve(documents, mode)
    }
}
