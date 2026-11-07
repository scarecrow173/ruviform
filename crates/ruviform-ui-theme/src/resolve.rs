//! Resolution pipeline: precedence merge + mode selection + alias
//! resolution into the immutable `ResolvedTheme`.
//!
//! Order of operations per the v1 contract:
//! 1. Documents are stably ordered by layer precedence
//!    (`primitive < semantic < component < product`); same-layer documents
//!    keep caller order (later wins).
//! 2. Tokens merge by path; an override must have the same declared
//!    `$type` (`TypeMismatch` otherwise) and affects only the same token
//!    key in the same mode scope.
//! 3. The requested mode selects each token's mode-scoped value, falling
//!    back to the base `$value`.
//! 4. Aliases resolve over the merged set (`UnknownReference`,
//!    `AliasCycle`, cross-type `TypeMismatch` are errors).

use std::collections::BTreeMap;

use crate::doc::{RawValue, TokenDocument, TokenType};
use crate::errors::ResolveError;
use crate::snapshot::{ResolvedTheme, ResolvedValue, TokenSource};

/// Deterministic resolver for token documents.
#[derive(Debug, Default)]
pub struct ThemeResolver;

/// Merge stage: declared type + selected (or placeholder) value.
struct Selected {
    ty: TokenType,
    value: RawValue,
}

impl ThemeResolver {
    /// Resolve `documents` into a `ResolvedTheme` for `mode`.
    ///
    /// Documents may be given in any order; they are evaluated in
    /// layer-precedence order (same layer: caller order, later wins).
    ///
    /// # Errors
    ///
    /// `ResolveError` for `EmptyDocuments`, `DuplicateSource`,
    /// `MissingMode`, `TypeMismatch`, `UnknownReference`, or `AliasCycle`.
    pub fn resolve(documents: &[TokenDocument], mode: &str) -> Result<ResolvedTheme, ResolveError> {
        if documents.is_empty() {
            return Err(ResolveError::EmptyDocuments);
        }

        let mut ordered: Vec<&TokenDocument> = documents.iter().collect();
        ordered.sort_by_key(|d| d.layer().rank());

        check_sources(&ordered)?;
        check_modes(&ordered, mode)?;

        // 2. Merge: later (higher-precedence) documents override the same
        // key wholesale (base value + mode map replace together).
        let mut merged: BTreeMap<String, &crate::doc::RawToken> = BTreeMap::new();
        for doc in &ordered {
            for (path, token) in &doc.tokens {
                if let Some(base) = merged.get(path)
                    && base.ty != token.ty
                {
                    return Err(ResolveError::TypeMismatch {
                        path: path.clone(),
                        base: base.ty.as_str().to_owned(),
                        override_: token.ty.as_str().to_owned(),
                    });
                }
                merged.insert(path.clone(), token);
            }
        }

        // 3. Mode selection: mode-scoped value overrides base for this mode.
        let mut selected: BTreeMap<String, Selected> = BTreeMap::new();
        for (path, token) in &merged {
            let value = token.modes.get(mode).unwrap_or(&token.value).clone();
            selected.insert(
                path.clone(),
                Selected {
                    ty: token.ty,
                    value,
                },
            );
        }

        // 4. Alias resolution with cycle detection (path-sorted DFS).
        let mut memo: BTreeMap<String, ResolvedValue> = BTreeMap::new();
        let mut stack: Vec<String> = Vec::new();
        for path in selected.keys() {
            resolve_token(path, &selected, &mut memo, &mut stack)?;
        }

        let sources = ordered
            .iter()
            .map(|d| TokenSource {
                layer: d.layer(),
                id: d.id().to_owned(),
                fingerprint: d.fingerprint,
            })
            .collect();
        Ok(ResolvedTheme::new(mode.to_owned(), sources, memo))
    }
}

fn check_sources(ordered: &[&TokenDocument]) -> Result<(), ResolveError> {
    let mut seen: Vec<(u8, &str)> = Vec::with_capacity(ordered.len());
    for doc in ordered {
        let key = (doc.layer().rank(), doc.id());
        if seen.contains(&key) {
            return Err(ResolveError::DuplicateSource {
                id: doc.id().to_owned(),
            });
        }
        seen.push(key);
    }
    Ok(())
}

fn check_modes(ordered: &[&TokenDocument], mode: &str) -> Result<(), ResolveError> {
    for doc in ordered {
        let declared = doc.modes();
        if !declared.is_empty() && !declared.iter().any(|m| m == mode) {
            return Err(ResolveError::MissingMode {
                document: doc.id().to_owned(),
                mode: mode.to_owned(),
            });
        }
    }
    Ok(())
}

fn resolve_token(
    path: &str,
    selected: &BTreeMap<String, Selected>,
    memo: &mut BTreeMap<String, ResolvedValue>,
    stack: &mut Vec<String>,
) -> Result<ResolvedValue, ResolveError> {
    if let Some(v) = memo.get(path) {
        return Ok(v.clone());
    }
    if let Some(start) = stack.iter().position(|p| p == path) {
        let mut cycle = stack[start..].to_vec();
        cycle.push(path.to_owned());
        return Err(ResolveError::AliasCycle { cycle });
    }
    let entry = selected
        .get(path)
        .ok_or_else(|| ResolveError::UnknownReference {
            path: stack.last().cloned().unwrap_or_else(|| path.to_owned()),
            reference: path.to_owned(),
        })?;
    let resolved = match &entry.value {
        RawValue::Literal(v) => v.clone(),
        RawValue::Alias(target) => {
            if let Some(ty) = selected.get(target).map(|t| t.ty)
                && ty != entry.ty
            {
                return Err(ResolveError::TypeMismatch {
                    path: target.clone(),
                    base: entry.ty.as_str().to_owned(),
                    override_: ty.as_str().to_owned(),
                });
            }
            stack.push(path.to_owned());
            let out = resolve_token(target, selected, memo, stack);
            stack.pop();
            match out {
                Ok(v) => v,
                Err(ResolveError::UnknownReference { .. }) => {
                    return Err(ResolveError::UnknownReference {
                        path: path.to_owned(),
                        reference: target.clone(),
                    });
                }
                Err(e) => return Err(e),
            }
        }
    };
    memo.insert(path.to_owned(), resolved.clone());
    Ok(resolved)
}
