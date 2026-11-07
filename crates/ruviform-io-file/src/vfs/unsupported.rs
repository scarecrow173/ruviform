//! Backend for targets that cannot provide handle-relative,
//! no-follow primitives — every operation fails `Unsupported`
//! honestly rather than degrading to path-string traversal. Also
//! installed on Windows builds older than RS1, where
//! `OBJ_DONT_REPARSE` is silently ignored.
//!
//! **RT contract:** non-RT only.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use super::{ListingOutcome, NodeIdentity, VfsDir, VfsEntry, VfsOps, VfsProbe, VisitFlow};

/// Constructs the shared unsupported backend.
pub(crate) fn ops() -> std::sync::Arc<dyn VfsOps> {
    std::sync::Arc::new(UnsupportedVfsOps)
}

#[derive(Debug)]
pub(crate) struct UnsupportedVfsOps;

fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "ruviform-io-file: no handle-relative no-follow backend for this target",
    )
}

impl VfsOps for UnsupportedVfsOps {
    fn open_dir_path(&self, _path: &Path) -> io::Result<Box<dyn VfsDir>> {
        Err(unsupported())
    }

    fn stat_file(&self, _file: &File) -> io::Result<VfsProbe> {
        Err(unsupported())
    }
}

#[derive(Debug)]
struct UnsupportedDir;

impl VfsDir for UnsupportedDir {
    fn path(&self) -> &Path {
        Path::new("")
    }

    fn identity(&self) -> io::Result<NodeIdentity> {
        Err(unsupported())
    }

    fn stat_self(&self) -> io::Result<VfsProbe> {
        Err(unsupported())
    }

    fn canonicalize_handle(&self) -> io::Result<PathBuf> {
        Err(unsupported())
    }

    fn probe(&self, _name: &OsStr) -> io::Result<VfsProbe> {
        Err(unsupported())
    }

    fn open_dir(&self, _name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        Err(unsupported())
    }

    fn open_regular_leaf(&self, _name: &OsStr) -> io::Result<(File, VfsProbe)> {
        Err(unsupported())
    }

    fn read_link(&self, _name: &OsStr) -> io::Result<OsString> {
        Err(unsupported())
    }

    fn list_each(
        &self,
        _visit: &mut dyn FnMut(VfsEntry) -> VisitFlow,
    ) -> io::Result<ListingOutcome> {
        Err(unsupported())
    }

    fn create_dir(&self, _name: &OsStr) -> io::Result<Box<dyn VfsDir>> {
        Err(unsupported())
    }

    fn create_file(&self, _name: &OsStr) -> io::Result<File> {
        Err(unsupported())
    }

    fn remove_file(&self, _name: &OsStr) -> io::Result<()> {
        Err(unsupported())
    }

    fn remove_dir(&self, _name: &OsStr) -> io::Result<()> {
        Err(unsupported())
    }
}
