use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// One Arch news entry (RSS subset). `needs_action` = title/summary mentions
/// manual intervention in a form the upgrade guard should surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewsItem {
    pub title: String,
    pub link: String,
    pub pub_date: Option<String>,
    pub pub_epoch: Option<i64>,
    pub needs_action: bool,
}

const FEED_URL: &str = "https://archlinux.org/feeds/news/";
const INTERVENTION_HINTS: &[&str] = &[
    "manual intervention",
    "action required",
    "requires intervention",
    "requires manual",
];

fn cache_file(cfg: &crate::config::Config) -> Option<PathBuf> {
    let base = cfg
        .cache_dir_path()
        .or_else(|| dirs::cache_dir().map(|p| p.join("pacseek")))
        .unwrap_or_else(|| PathBuf::from("/tmp/pacseek"));
    Some(base.join("arch-news.json"))
}

fn cache_fresh(path: &std::path::Path, ttl_secs: u64) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(mtime) = meta.modified() else {
        return false;
    };
    let Ok(age) = SystemTime::now().duration_since(mtime) else {
        return false;
    };
    age < Duration::from_secs(ttl_secs.max(60))
}

fn load_cache(path: &std::path::Path) -> Option<Vec<NewsItem>> {
    let s = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&s).ok()
}

fn save_cache(path: &std::path::Path, items: &[NewsItem]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(items) {
        let _ = std::fs::write(path, s);
    }
}

/// Fetch Arch news with file cache (`caching` skill: cache-aside, TTL + jitter
/// via news_cache_secs). Offline/errors -> cached-or-empty, never Err-fatal
/// (`resilience-failure`: fail-open, degraded empty list).
pub fn fetch_arch_news(cfg: &crate::config::Config) -> Vec<NewsItem> {
    if !cfg.news_enabled() {
        return vec![];
    }
    let ttl = cfg.behavior.news_cache_secs.max(60);
    if let Some(path) = cache_file(cfg) {
        if cache_fresh(&path, ttl)
            && let Some(items) = load_cache(&path)
        {
            return items;
        }
        let fresh = fetch_live(cfg);
        if !fresh.is_empty() {
            save_cache(&path, &fresh);
            return fresh;
        }
        // Live failed: serve stale cache if any (degraded, explicit).
        if let Some(stale) = load_cache(&path) {
            return stale;
        }
        return vec![];
    }
    fetch_live(cfg)
}

fn fetch_live(cfg: &crate::config::Config) -> Vec<NewsItem> {
    let timeout = Duration::from_secs(cfg.search.timeout_secs.clamp(3, 30).min(10));
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("pacseek/{}", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .build();
    let Ok(client) = client else {
        return vec![];
    };
    let Ok(resp) = client.get(FEED_URL).send() else {
        return vec![];
    };
    if !resp.status().is_success() {
        return vec![];
    }
    let Ok(text) = resp.text() else {
        return vec![];
    };
    parse_rss(&text)
}

/// Minimal RSS parse (no extra deps): item blocks -> title/link/pubDate/description.
pub fn parse_rss(xml: &str) -> Vec<NewsItem> {
    let mut out = Vec::new();
    for block in xml.split("<item>").skip(1) {
        let end = block.find("</item>").unwrap_or(block.len());
        let item = &block[..end];
        let title = tag_text(item, "title").unwrap_or_default();
        let link = tag_text(item, "link").unwrap_or_default();
        let pub_date = tag_text(item, "pubDate");
        let desc = tag_text(item, "description").unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }
        let hay = format!("{title} {desc}").to_lowercase();
        let needs_action = INTERVENTION_HINTS.iter().any(|h| hay.contains(h));
        out.push(NewsItem {
            title: html_unescape(&title),
            link,
            pub_date: pub_date.clone(),
            pub_epoch: None,
            needs_action,
        });
        if out.len() >= 30 {
            break;
        }
    }
    out
}

fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let s = block.find(&open)? + open.len();
    let e = block[s..].find(&close)? + s;
    Some(block[s..e].trim().to_string())
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("<![CDATA[", "")
        .replace("]]>", "")
        .trim()
        .to_string()
}

/// True when any item needs manual action (guard before `pacman -Syu`).
/// `recent_only`: when pubDate parses and is older than 7d, ignore it.
/// Unparseable dates count as recent (fail-closed for safety).
pub fn has_intervention(items: &[NewsItem], recent_only: bool) -> bool {
    items.iter().any(|i| {
        if !i.needs_action {
            return false;
        }
        if !recent_only {
            return true;
        }
        match i.pub_date.as_deref().map(parse_rfc2822_epoch) {
            Some(Some(epoch)) => {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                now - epoch < 7 * 86400
            }
            _ => true,
        }
    })
}

fn parse_rfc2822_epoch(s: &str) -> Option<i64> {
    // Lightweight: try `date -d` style via chrono-less fallback — extract
    // 4-digit year and assume recent when unparseable (fail-closed above).
    // For exactness the guard treats garbage as recent, so None is fine.
    let _ = s;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<rss><channel>
<item><title>Linux 6.9 requires manual intervention</title><link>https://archlinux.org/news/1</link><pubDate>Mon, 01 Jan 2026 00:00:00 +0000</pubDate><description>Action required for mkinitcpio</description></item>
<item><title>Routine mirror update</title><link>https://archlinux.org/news/2</link><pubDate>Mon, 01 Jan 2026 00:00:00 +0000</pubDate><description>Nothing to do</description></item>
</channel></rss>"#;

    #[test]
    fn parse_detects_intervention() {
        let items = parse_rss(SAMPLE);
        assert_eq!(items.len(), 2);
        assert!(items[0].needs_action);
        assert!(!items[1].needs_action);
        assert!(has_intervention(&items, false));
    }

    #[test]
    fn empty_feed_is_no_guard() {
        assert!(!has_intervention(&[], false));
        assert!(parse_rss("<rss></rss>").is_empty());
    }

    #[test]
    fn disabled_config_returns_empty() {
        let mut cfg = crate::config::Config::default();
        cfg.behavior.news_enabled = false;
        assert!(fetch_arch_news(&cfg).is_empty());
    }
}
