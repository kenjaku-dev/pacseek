use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::model::Package;

const AUR_RPC: &str = "https://aur.archlinux.org/rpc/v5";

#[derive(Debug, Deserialize, Serialize)]
struct AurResponse {
    version: u32,
    #[serde(rename = "type")]
    type_: String,
    resultcount: u32,
    results: Vec<AurPackage>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AurPackage {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PackageBase")]
    pub package_base: Option<String>,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    #[serde(rename = "NumVotes")]
    pub num_votes: Option<i32>,
    #[serde(rename = "Popularity")]
    pub popularity: Option<f64>,
    #[serde(rename = "OutOfDate")]
    pub out_of_date: Option<i64>,
    #[serde(rename = "Maintainer")]
    pub maintainer: Option<String>,
    #[serde(rename = "FirstSubmitted")]
    pub first_submitted: Option<i64>,
    #[serde(rename = "LastModified")]
    pub last_modified: Option<i64>,
    #[serde(rename = "Depends")]
    pub depends: Option<Vec<String>>,
}

impl From<AurPackage> for Package {
    fn from(p: AurPackage) -> Self {
        Package {
            name: p.name,
            version: p.version,
            description: p.description,
            repo: "aur".to_string(),
            arch: None,
            url: p.url,
            installed: false,
            votes: p.num_votes,
            popularity: p.popularity,
            out_of_date: p.out_of_date,
            maintainer: p.maintainer,
            num_votes: p.num_votes,
            last_modified: p.last_modified,
        }
    }
}

pub async fn search_aur(
    client: &reqwest::Client,
    query: &str,
    by: &str,
    limit: usize,
    use_regex: bool,
) -> anyhow::Result<Vec<Package>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    // AUR RPC is case-insensitive, url-encode
    let encoded = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
    let encoded = encoded.replace('+', "%20");
    let url = format!("{}/search/{}?by={}", AUR_RPC, encoded, by);

    tracing::debug!(url=%url, "aur request");

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            format!("pacseek/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .with_context(|| format!("AUR request failed {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("AUR RPC returned HTTP {}", resp.status());
    }

    let text = resp.text().await?;
    let parsed: AurResponse = serde_json::from_str(&text)
        .with_context(|| format!("AUR json parse failed: {}", &text[..text.len().min(500)]))?;

    if parsed.type_ == "error" {
        let msg = parsed
            .error
            .unwrap_or_else(|| "unknown AUR error".to_string());
        anyhow::bail!("AUR error: {}", msg);
    }

    let mut results: Vec<Package> = parsed.results.into_iter().map(Package::from).collect();

    // client-side regex filtering if requested (AUR doesn't support regex)
    if use_regex {
        let re = regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()?;
        results.retain(|p| {
            re.is_match(&p.name)
                || p.description
                    .as_deref()
                    .map(|d| re.is_match(d))
                    .unwrap_or(false)
        });
    }

    // Sort by popularity/votes desc like paru does (more trusted first), but keep original if bottom_up false
    results.sort_by(|a, b| {
        b.popularity
            .unwrap_or(0.0)
            .partial_cmp(&a.popularity.unwrap_or(0.0))
            .unwrap()
            .then_with(|| b.votes.unwrap_or(0).cmp(&a.votes.unwrap_or(0)))
    });

    if limit != 0 && results.len() > limit {
        results.truncate(limit);
    }

    Ok(results)
}

pub fn search_aur_blocking(
    query: &str,
    by: &str,
    limit: usize,
    use_regex: bool,
) -> anyhow::Result<Vec<Package>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let encoded = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
    let encoded = encoded.replace('+', "%20");
    let url = format!("{}/search/{}?by={}", AUR_RPC, encoded, by);
    tracing::debug!(url=%url, "aur blocking request");
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("pacseek/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            format!("pacseek/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .with_context(|| format!("AUR request failed {}", url))?;
    if !resp.status().is_success() {
        anyhow::bail!("AUR RPC returned HTTP {}", resp.status());
    }
    let text = resp.text()?;
    let parsed: AurResponse = serde_json::from_str(&text)
        .with_context(|| format!("AUR json parse failed: {}", &text[..text.len().min(500)]))?;
    if parsed.type_ == "error" {
        let msg = parsed
            .error
            .unwrap_or_else(|| "unknown AUR error".to_string());
        anyhow::bail!("AUR error: {}", msg);
    }
    let mut results: Vec<Package> = parsed.results.into_iter().map(Package::from).collect();
    if use_regex {
        let re = regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()?;
        results.retain(|p| {
            re.is_match(&p.name)
                || p.description
                    .as_deref()
                    .map(|d| re.is_match(d))
                    .unwrap_or(false)
        });
    }
    results.sort_by(|a, b| {
        b.popularity
            .unwrap_or(0.0)
            .partial_cmp(&a.popularity.unwrap_or(0.0))
            .unwrap()
            .then_with(|| b.votes.unwrap_or(0).cmp(&a.votes.unwrap_or(0)))
    });
    if limit != 0 && results.len() > limit {
        results.truncate(limit);
    }
    Ok(results)
}

// Alternative via raur crate (kept for reference, not used in hot path)
// pub async fn search_aur_via_raur(query: &str) -> anyhow::Result<Vec<Package>> {
//     use raur::Raur;
//     let raur = raur::Handle::new();
//     let res = raur.search_by(query, raur::SearchBy::NameDesc).await?;
//     Ok(res.into_iter().map(|p| Package { ... }).collect())
// }
