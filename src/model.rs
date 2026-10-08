use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repo: String, // e.g., "extra", "aur", "system"
    pub arch: Option<String>,
    pub url: Option<String>,
    pub installed: bool,
    /// AUR votes (NumVotes) — primary field, used for sorting/display
    pub votes: Option<i32>,
    pub popularity: Option<f64>,
    pub out_of_date: Option<i64>,
    pub maintainer: Option<String>,
    /// Alias for `votes` kept for backwards compat with older JSON; always equals `votes` (aur-guides:aur-rpc)
    pub num_votes: Option<i32>,
    pub last_modified: Option<i64>,
    /// 0.5.0 maintenance+safety: dependency metadata (repo localdb; AUR info when available).
    /// All optional with serde defaults so older JSON still parses.
    #[serde(default)]
    pub depends: Option<Vec<String>>,
    #[serde(default)]
    pub optdepends: Option<Vec<String>>,
    #[serde(default)]
    pub required_by: Option<Vec<String>>,
    #[serde(default)]
    pub optional_for: Option<Vec<String>>,
    /// Install reason: "explicit" | "dependency" (localdb only).
    #[serde(default)]
    pub reason: Option<String>,
    /// True when installed as dependency with nothing requiring it (pacman -Qdt).
    #[serde(default)]
    pub orphan: bool,
}

impl Package {
    pub fn short_desc(&self) -> &str {
        self.description.as_deref().unwrap_or("-")
    }

    /// Minimal constructor — dependency metadata defaults to None/false.
    /// Use for search results where deps are not yet resolved.
    pub fn minimal(name: String, version: String, repo: String) -> Self {
        Self {
            name,
            version,
            description: None,
            repo,
            arch: None,
            url: None,
            installed: false,
            votes: None,
            popularity: None,
            out_of_date: None,
            maintainer: None,
            num_votes: None,
            last_modified: None,
            depends: None,
            optdepends: None,
            required_by: None,
            optional_for: None,
            reason: None,
            orphan: false,
        }
    }

    /// Safety badges for list UI: `[OOD] [ORPHAN] [installed]`-style tags.
    /// Pure, tested, no IO — ui-design single-signal per row.
    pub fn badges(&self) -> Vec<&'static str> {
        let mut b = Vec::with_capacity(3);
        if self.out_of_date.is_some() {
            b.push("[OOD]");
        }
        if self.orphan {
            b.push("[ORPHAN]");
        }
        if self
            .maintainer
            .as_deref()
            .map(|m| m.trim().is_empty())
            .unwrap_or(true)
            && self.repo == "aur"
        {
            b.push("[UNMAINTAINED]");
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_cover_ood_orphan_unmaintained() {
        let mut p = Package::minimal("x".into(), "1-1".into(), "aur".into());
        // Unmaintained AUR (no maintainer) shows by default.
        assert!(p.badges().contains(&"[UNMAINTAINED]"));
        p.out_of_date = Some(1);
        p.orphan = true;
        p.maintainer = Some("bob".into());
        let b = p.badges();
        assert!(b.contains(&"[OOD]"));
        assert!(b.contains(&"[ORPHAN]"));
        assert!(!b.contains(&"[UNMAINTAINED]"));
    }

    #[test]
    fn old_json_without_new_fields_parses() {
        let v = serde_json::json!({
            "name": "firefox", "version": "1-1",
            "description": null, "repo": "extra",
            "arch": null, "url": null, "installed": false,
            "votes": null, "popularity": null,
            "out_of_date": null, "maintainer": null,
            "num_votes": null, "last_modified": null
        });
        let p: Package = serde_json::from_value(v).unwrap();
        assert_eq!(p.name, "firefox");
        assert!(!p.orphan);
        assert!(p.depends.is_none());
    }
}
