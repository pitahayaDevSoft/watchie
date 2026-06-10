use anyhow::{Context, Result};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::config::Config;

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .timeout(std::time::Duration::from_secs(0)) // no timeout for downloads
            .build()?;
        Ok(Self { client })
    }

    /// Download a URL to `dest`, reporting progress via a callback.
    pub async fn download<F>(
        &self,
        url: &str,
        referer: Option<&str>,
        dest: &Path,
        on_progress: F,
    ) -> Result<PathBuf>
    where
        F: Fn(u64, Option<u64>) + Send + Sync + 'static,
    {
        let mut builder = self.client.get(url);
        if let Some(ref_val) = referer {
            builder = builder.header("Referer", ref_val);
        }
        let resp = builder.send().await
            .with_context(|| format!("Fetching {}", url))?;

        let total = resp.content_length();
        let mut downloaded: u64 = 0;

        let mut file = File::create(dest).await
            .with_context(|| format!("Creating file {}", dest.display()))?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Reading download chunk")?;
            file.write_all(&chunk).await.context("Writing chunk")?;
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total);
        }

        file.flush().await?;
        Ok(dest.to_path_buf())
    }

    /// Get the Content-Length of a URL without downloading it.
    pub async fn probe_size(&self, url: &str, referer: Option<&str>) -> Result<Option<u64>> {
        let mut builder = self.client.head(url);
        if let Some(ref_val) = referer {
            builder = builder.header("Referer", ref_val);
        }
        let resp = builder.send().await?;
        Ok(resp.content_length())
    }

    /// Build a destination path in the download directory.
    pub fn build_dest(config: &Config, filename: &str) -> PathBuf {
        config.download_dir.join(sanitize_filename(filename))
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

pub fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
