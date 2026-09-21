pub mod aur;
pub mod repo;

pub use aur::{
    search_aur, search_aur_blocking, search_aur_blocking_with_config, search_aur_with_config,
};
pub use repo::{search_local, search_local_fallback, search_repo, search_repo_fallback};
