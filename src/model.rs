use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repo: String, // e.g., "extra", "aur", "system"
    pub arch: Option<String>,
    pub url: Option<String>,
    pub installed: bool,
    pub votes: Option<i32>,
    pub popularity: Option<f64>,
    pub out_of_date: Option<i64>,
    pub maintainer: Option<String>,
    pub num_votes: Option<i32>,
    pub last_modified: Option<i64>,
}

impl Package {
    pub fn short_desc(&self) -> &str {
        self.description.as_deref().unwrap_or("-")
    }
}
