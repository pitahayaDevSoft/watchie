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

pub struct PlayImdbClient {
    client: reqwest::Client,
    base_url: String,
}

impl PlayImdbClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            base_url: "https://playimdb.com".to_string(),
        })
    }

    async fn get_html(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await?;
        Ok(resp.text().await?)
    }

    /// Search playimdb.com for an IMDB ID and return stream info.
    pub async fn get_stream_info(&self, imdb_id: &str) -> Result<StreamInfo> {
        // playimdb.com accepts direct IMDB tt IDs
        let url = format!("{}/title/{}", self.base_url, imdb_id);
        let html = self.get_html(&url).await?;
        self.parse_stream_page(&html, &url)
    }

    /// Also try a generic search on playimdb.com by title
    pub async fn search_by_title(&self, title: &str, year: Option<u16>) -> Result<Vec<StreamInfo>> {
        let query = if let Some(y) = year {
            format!("{} {}", title, y)
        } else {
            title.to_string()
        };
        let encoded = urlencoding::encode(&query);
        let url = format!("{}/search?q={}", self.base_url, encoded);
        let html = self.get_html(&url).await?;
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

        Ok(StreamInfo {
            title,
            stream_url: page_url.to_string(),
            direct_url,
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
