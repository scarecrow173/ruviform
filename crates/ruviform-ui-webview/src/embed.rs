//! `rust-embed`-backed [`AssetSource`] — the production single-binary
//! layout from `poc/ui`: debug builds read the filesystem live,
//! release builds embed assets into the executable.
//!
//! Usage in an application crate:
//!
//! ```ignore
//! #[derive(rust_embed::RustEmbed)]
//! #[folder = "frontend/dist"]
//! struct UiAssets;
//!
//! let assets = EmbedAssets::<UiAssets>::new();
//! ```
//!
//! The application owns the `#[folder]` path so it controls where
//! `vite build` output lands; this adapter only maps lookups.

use std::marker::PhantomData;
use std::sync::Arc;

use rust_embed::RustEmbed;
use ruviform_ui::{Asset, AssetSource, mime_for};

/// [`AssetSource`] over a [`RustEmbed`] asset set.
pub struct EmbedAssets<T> {
    _marker: PhantomData<T>,
}

impl<T: RustEmbed + Send + Sync> EmbedAssets<T> {
    /// Creates the source. Cheap — embed data is static.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    /// Wraps the source in an [`Arc`] for [`crate::host::run`].
    #[must_use]
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

impl<T: RustEmbed + Send + Sync> Default for EmbedAssets<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: RustEmbed + Send + Sync> AssetSource for EmbedAssets<T> {
    fn get(&self, path: &str) -> Option<Asset> {
        T::get(path).map(|file| Asset {
            bytes: file.data,
            mime: mime_for(path),
        })
    }
}
