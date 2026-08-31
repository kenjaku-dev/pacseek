pub mod aur;
pub mod repo;

pub use aur::search_aur;
pub use repo::{search_repo, search_repo_fallback};
