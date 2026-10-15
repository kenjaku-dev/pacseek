//! Source backends seam (P1 foundation for flatpak + future sources).
//!
//! `service-decomposition`: modular monolith — one binary, enforced module
//! boundaries. `repo`/`aur` keep their current paths in `src/search/` and
//! `src/install/` for now (no behavior change in P1); new sources implement
//! [`SourceBackend`] here. P2-P4 will migrate repo/aur behind the same trait.

pub mod flatpak;

pub use flatpak::{FlatpakBackend, has_flatpak, list_installed};

use crate::model::Package;

/// Minimal backend contract for P1. Extended in P2 (search), P3
/// (install/remove), P4 (check_updates) with default empty impls so existing
/// backends adopt incrementally without breaking the build.
pub trait SourceBackend {
    /// Stable id used as `Package.repo`, e.g. `"repo"`, `"aur"`, `"flatpak"`.
    fn id(&self) -> &'static str;

    /// Binary/config present and enabled. False means fail-open empty.
    fn is_available(&self) -> bool;

    /// Installed packages for remover/stats paths. Empty when unavailable.
    fn list_installed(&self) -> anyhow::Result<Vec<Package>>;

    /// P2: query remote. Default empty so P1 compiles before search lands.
    fn search(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<Package>> {
        Ok(vec![])
    }

    /// P3: install by backend-native id. Default refuses so wiring without an
    /// impl cannot silently succeed.
    fn install(&self, _id: &str) -> anyhow::Result<()> {
        anyhow::bail!("install not implemented for this backend")
    }

    /// P3: remove by backend-native id.
    fn remove(&self, _id: &str) -> anyhow::Result<()> {
        anyhow::bail!("remove not implemented for this backend")
    }

    /// P4: available updates as (name, old, new). Default empty.
    fn check_updates(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        Ok(vec![])
    }
}
