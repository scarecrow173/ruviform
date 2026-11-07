//! `ruviform-ui-theme` — backend-neutral DTCG design-token interpreter.
//!
//! Single source of truth for design-token interpretation in Ruviform:
//! a deterministic pipeline from Git-managed DTCG JSON documents
//! (`parse → validate → alias resolution → mode/override merge`) into an
//! immutable, typed [`ResolvedTheme`].
//!
//! The crate depends only on `serde`/`serde_json`. It has no GPUI,
//! toolkit, DOM, wry, or `ruviform-ui` host-contract dependency; driver
//! mappers (e.g. the GPUI driver, Issue #386) consume [`ResolvedTheme`]
//! one-way into their private style types.
//!
//! # Example
//!
//! ```
//! let base = r##"
//! {
//!   "$extensions": {
//!     "dev.ruviform.dtcg-profile": {
//!       "profile": "rvf-dtcg-profile.v1",
//!       "layer": "primitive",
//!       "id": "base",
//!       "modes": ["light", "dark"]
//!     }
//!   },
//!   "color": {
//!     "$type": "color",
//!     "bg": {
//!       "$value": "#ffffff",
//!       "$extensions": { "dev.ruviform.modes": { "dark": "#101418" } }
//!     }
//!   }
//! }"##;
//! let doc = ruviform_ui_theme::TokenDocument::from_json(base)?;
//! let theme = ruviform_ui_theme::ThemeResolver::resolve(&[doc], "dark")?;
//! let bg = theme.color("color.bg").unwrap();
//! assert_eq!(bg.alpha, 1.0);
//! # Ok::<(), ruviform_ui_theme::error::ResolveError>(())
//! ```

mod doc;
mod errors;
mod registry;
mod resolve;
mod snapshot;

pub use doc::{DocumentLayer, TokenDocument};
pub use errors::{ResolveError, TokenError};
pub use registry::ThemeRegistry;
pub use resolve::ThemeResolver;
pub use snapshot::{
    ColorSpace, DurationUnit, LengthUnit, ResolvedColor, ResolvedDuration, ResolvedLength,
    ResolvedTheme, ResolvedValue, Rounding, ScaleContext, StrokeStyle, TokenSource,
};

/// Error module kept public for doctest/`?` ergonomics.
pub mod error {
    pub use crate::errors::{ResolveError, TokenError};
}

/// DTCG profile schema version implemented by this crate.
pub const SCHEMA_VERSION: &str = "rvf-dtcg-profile.v1";
