use anyhow::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};


// ─── Data models ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Title of the content on playimdb.com
    pub title: String,
    /// URL of the embed/stream page
    pub stream_url: String,
    /// Direct video URL (if extractable)
    pub direct_url: Option<String>,
    /// Estimated file size (if known)
    pub file_size: Option<u64>,
    /// Available quality options
    pub qualities: Vec<Quality>,
    /// Magnet / torrent links if available
    pub torrent_links: Vec<TorrentLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quality {
    pub label: String,    // "1080p", "720p", etc.
    pub url: String,
    pub size_bytes: Option<u64>,
    pub format: String,   // "mp4", "mkv", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentLink {
    pub label: String,
    pub magnet: Option<String>,
    pub torrent_url: Option<String>,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
}

// ─── PlayIMDB client ──────────────────────────────────────────────────────────

const PLAYIMDB_MIRRORS: &[&str] = &[
    "https://playimdb.com",
    "https://runimdb.com",
    "https://streamimdb.com",
    "https://directimdb.com",
    "https://fastimdb.com",
];

pub struct PlayImdbClient {
    client: reqwest::Client,
    base_url: String,
    is_custom_url: bool,
    active_mirror_index: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl PlayImdbClient {
    pub fn new() -> Result<Self> {
        if let Ok(cfg) = crate::config::Config::load() {
            Self::new_with_config(&cfg)
        } else {
            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
                .cookie_store(true)
                .timeout(std::time::Duration::from_secs(30))
                .build()?;
            Ok(Self {
                client,
                base_url: "https://playimdb.com".to_string(),
                is_custom_url: false,
                active_mirror_index: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            })
        }
    }

    pub fn new_with_config(config: &crate::config::Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(&config.network.user_agent)
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(config.network.timeout_secs))
            .build()?;
        let (base_url, is_custom_url) = if let Some(ref url) = config.api.playimdb_url {
            (url.trim_end_matches('/').to_string(), true)
        } else {
            ("https://playimdb.com".to_string(), false)
        };
        Ok(Self {
            client,
            base_url,
            is_custom_url,
            active_mirror_index: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    async fn get_html(&self, path: &str) -> Result<(String, String)> {
        if self.is_custom_url {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.client.get(&url).send().await?;
            let text = resp.text().await?;
            return Ok((url, text));
        }

        let index = self.active_mirror_index.load(std::sync::atomic::Ordering::Relaxed);
        let mut mirrors = PLAYIMDB_MIRRORS.to_vec();
        if index < mirrors.len() {
            mirrors.rotate_left(index);
        }

        let mut last_err = None;
        for (i, &mirror) in mirrors.iter().enumerate() {
            let url = format!("{}{}", mirror, path);
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.client.get(&url).send()
            ).await {
                Ok(Ok(resp)) => {
                    if resp.status().is_success() {
                        if let Ok(text) = resp.text().await {
                            let original_index = (index + i) % PLAYIMDB_MIRRORS.len();
                            self.active_mirror_index.store(original_index, std::sync::atomic::Ordering::Relaxed);
                            return Ok((url, text));
                        }
                    }
                }
                Ok(Err(e)) => {
                    last_err = Some(anyhow::anyhow!(e));
                }
                Err(_) => {
                    last_err = Some(anyhow::anyhow!("Timeout connecting to {}", mirror));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("All playimdb mirrors failed")))
    }

    /// Search playimdb.com for an IMDB ID and return stream info.
    pub async fn get_stream_info(&self, imdb_id: &str) -> Result<StreamInfo> {
        let path = format!("/title/{}", imdb_id);
        let (resolved_url, html) = self.get_html(&path).await?;
        self.parse_stream_page(&html, &resolved_url)
    }

    /// Also try a generic search on playimdb.com by title
    pub async fn search_by_title(&self, title: &str, year: Option<u16>) -> Result<Vec<StreamInfo>> {
        let query = if let Some(y) = year {
            format!("{} {}", title, y)
        } else {
            title.to_string()
        };
        let encoded = urlencoding::encode(&query);
        let path = format!("/search?q={}", encoded);
        let (_resolved_url, html) = self.get_html(&path).await?;
        self.parse_search_results(&html)
    }

    fn parse_stream_page(&self, html: &str, page_url: &str) -> Result<StreamInfo> {
        let document = Html::parse_document(html);

        let title = document
            .select(&Selector::parse("h1.title, h1, .entry-title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        // Collect quality links — common pattern: anchors with resolution labels
        let mut qualities: Vec<Quality> = Vec::new();
        let link_sel = Selector::parse("a[href]").unwrap();

        for el in document.select(&link_sel) {
            let href = el.value().attr("href").unwrap_or("").to_string();
            let text = el.text().collect::<String>().trim().to_string();

            let is_quality = text.contains("1080") || text.contains("720") || text.contains("480")
                || text.contains("4K") || text.contains("HD") || text.contains("BluRay")
                || text.contains("WEB") || href.ends_with(".mp4") || href.ends_with(".mkv");

            if is_quality && !href.is_empty() {
                let format = if href.ends_with(".mkv") { "mkv" } else { "mp4" }.to_string();
                qualities.push(Quality {
                    label: if text.is_empty() { "Stream".to_string() } else { text },
                    url: href,
                    size_bytes: None,
                    format,
                });
            }
        }

        // Extract torrent / magnet links
        let mut torrent_links: Vec<TorrentLink> = Vec::new();
        for el in document.select(&link_sel) {
            let href = el.value().attr("href").unwrap_or("");
            let text = el.text().collect::<String>().trim().to_string();
            if href.starts_with("magnet:") {
                let seeders = extract_seeders_from_text(&text);
                let size_bytes = parse_size_from_text(&text);
                torrent_links.push(TorrentLink {
                    label: text,
                    magnet: Some(href.to_string()),
                    torrent_url: None,
                    size_bytes,
                    seeders,
                });
            } else if href.ends_with(".torrent") {
                torrent_links.push(TorrentLink {
                    label: text,
                    magnet: None,
                    torrent_url: Some(href.to_string()),
                    size_bytes: None,
                    seeders: None,
                });
            }
        }

        // Extract embed iframes (some sites use embedded players)
        let iframe_sel = Selector::parse("iframe[src]").unwrap();
        let direct_url = document
            .select(&iframe_sel)
            .next()
            .and_then(|el| el.value().attr("src"))
            .map(String::from);

        let mut qualities = qualities;
        if qualities.is_empty() {
            if let Some(ref url) = direct_url {
                qualities.push(Quality {
                    label: "Web Embed / Default Stream".to_string(),
                    url: url.clone(),
                    size_bytes: None,
                    format: "html".to_string(),
                });
            }
        }

        Ok(StreamInfo {
            title,
            stream_url: page_url.to_string(),
            direct_url: direct_url.clone(),
            file_size: qualities.first().and_then(|q| q.size_bytes),
            qualities,
            torrent_links,
        })
    }

    fn parse_search_results(&self, html: &str) -> Result<Vec<StreamInfo>> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        let item_sel = Selector::parse("article, .result-item, .movie-item, li.result").unwrap();
        let link_sel = Selector::parse("a[href]").unwrap();
        let title_sel = Selector::parse("h2, h3, .title").unwrap();

        for el in document.select(&item_sel).take(10) {
            if let Some(link) = el.select(&link_sel).next() {
                let href = link.value().attr("href").unwrap_or("").to_string();
                let title = el
                    .select(&title_sel)
                    .next()
                    .map(|t| t.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();

                if !href.is_empty() && !title.is_empty() {
                    results.push(StreamInfo {
                        title,
                        stream_url: href,
                        direct_url: None,
                        file_size: None,
                        qualities: vec![],
                        torrent_links: vec![],
                    });
                }
            }
        }

        Ok(results)
    }

    /// Open the stream URL in the system's default browser
    pub async fn open_in_browser(&self, url: &str) -> Result<()> {
        open::that(url)?;
        Ok(())
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn parse_size_from_text(text: &str) -> Option<u64> {
    let re = regex::Regex::new(r"(\d+\.?\d*)\s*(GB|MB|GiB|MiB)").ok()?;
    let caps = re.captures(text)?;
    let num: f64 = caps[1].parse().ok()?;
    let unit = &caps[2];
    let bytes = match unit {
        "GB" | "GiB" => (num * 1_073_741_824.0) as u64,
        "MB" | "MiB" => (num * 1_048_576.0) as u64,
        _ => return None,
    };
    Some(bytes)
}

fn extract_seeders_from_text(text: &str) -> Option<u32> {
    let re = regex::Regex::new(r"(\d+)\s*[Ss]eed").ok()?;
    let caps = re.captures(text)?;
    caps[1].parse().ok()
}

pub fn format_size(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

pub fn extract_origin(url_str: &str) -> Option<String> {
    if let Ok(u) = url::Url::parse(url_str) {
        if let Some(host) = u.host_str() {
            let scheme = u.scheme();
            return Some(format!("{}://{}/", scheme, host));
        }
    }
    None
}
