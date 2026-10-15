//! Source backends seam (P1 foundation for flatpak + future sources).
//!
//! `service-decomposition`: modular monolith — one binary, enforced module
//! boundaries. `repo`/`aur` keep their current paths in `src/search/` and
//! `src/install/` for now (no behavior change in P1); new sources implement
//! [`SourceBackend`] here. P2-P4 will migrate repo/aur behind the same trait.

pub mod flatpak;

pub use flatpak::{
    FlatpakBackend, check_flatpak_updates_cached, check_flatpak_updates_cached_for,
    check_flatpak_updates_for, check_flatpak_updates_with_config, clean_unused_flatpak,
    has_flatpak, install_flatpak_package, invalidate_flatpak_list_memo, invalidate_flatpak_memo,
    is_flatpak_installed, list_installed, list_installed_cached, remove_flatpak_package,
    search_flatpak_with_config, update_all_flatpak, update_flatpak_package, validate_app_id,
};

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

    /// P2: query remote. Empty when unavailable/disabled (fail-open).
    fn search(
        &self,
        query: &str,
        limit: usize,
        use_regex: bool,
        cfg: &crate::config::Config,
    ) -> anyhow::Result<Vec<Package>> {
        let _ = (query, limit, use_regex, cfg);
        Ok(vec![])
    }

    /// P3: install by backend-native id. Default refuses so wiring without an
    /// impl cannot silently succeed.
    fn install(&self, id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
        let _ = (id, cfg);
        anyhow::bail!("install not implemented for this backend")
    }

    /// P3: remove by backend-native id.
    fn remove(&self, id: &str, cfg: &crate::config::Config) -> anyhow::Result<()> {
        let _ = (id, cfg);
        anyhow::bail!("remove not implemented for this backend")
    }

    /// P4: available updates as (name, old, new). Default empty.
    fn check_updates(
        &self,
        cfg: &crate::config::Config,
    ) -> anyhow::Result<Vec<(String, String, String)>> {
        let _ = cfg;
        Ok(vec![])
    }
}
