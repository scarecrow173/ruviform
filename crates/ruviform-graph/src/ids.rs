//! Stable logical IDs and runtime indexes for the typed graph IR.
//!
//! Logical IDs are issued by [`crate::GraphBuilder`] (and
//! [`crate::GraphRevision`] by [`crate::GraphProject`]) and are stable for
//! the lifetime of a `GraphSpec`/`ValidatedGraph`. Runtime indexes are
//! separate, deterministic positions assigned by
//! [`crate::StableHandleMap`] — the two spaces are never interchangeable.

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        ///
        /// Wraps a `u64` issued by the owning builder/container; `0` is
        /// never issued — issuance starts at `1`.
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// The raw issued value (always `>= 1` when produced by a
            /// builder).
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }

            /// Builds an ID from the next issuance counter.
            /// `pub(crate)` — external callers never fabricate IDs.
            pub(crate) const fn from_issued(raw: u64) -> Self {
                Self(raw)
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

macro_rules! index_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        ///
        /// Wraps a `usize` position assigned by
        /// [`crate::StableHandleMap`].
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(usize);

        impl $name {
            /// The raw index value.
            #[must_use]
            pub const fn get(self) -> usize {
                self.0
            }

            /// Builds an index from a raw value. `pub(crate)` — external
            /// callers obtain indexes only through
            /// [`crate::StableHandleMap`].
            pub(crate) const fn new(raw: usize) -> Self {
                Self(raw)
            }
        }
    };
}

id_type!(
    /// Stable logical identifier of a graph node.
    NodeId
);
id_type!(
    /// Stable logical identifier of a port. Globally unique within a
    /// `GraphSpec` — the owner node is tracked separately by
    /// [`crate::PortSpec::owner`].
    PortId
);
id_type!(
    /// Stable logical identifier of a connection.
    ConnectionId
);
id_type!(
    /// Project-local revision issued by [`crate::GraphProject::prepare`].
    /// `GraphSpec` itself carries no revision — issuance is the
    /// project's responsibility.
    GraphRevision
);

index_type!(
    /// Deterministic runtime position of a node (topological order).
    RuntimeNodeIndex
);
index_type!(
    /// Runtime position of a port (insertion order within the graph).
    RuntimePortIndex
);
index_type!(
    /// Runtime position of a connection (insertion order).
    RuntimeConnectionIndex
);
