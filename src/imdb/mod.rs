use anyhow::Result;
use serde::{Deserialize, Serialize};

// ─── Data models ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Movie {
    pub id: String,         // tt1234567 or tmdb movie/tv identifier
    pub title: String,
    pub year: Option<u16>,
    pub rating: Option<f32>,
    pub votes: Option<u64>,
    pub genres: Vec<String>,
    pub runtime: Option<u32>, // minutes
    pub plot: Option<String>,
    pub director: Vec<String>,
    pub cast: Vec<String>,
    pub poster_url: Option<String>,
    pub imdb_url: String,
    pub content_type: ContentType,
    pub episodes: Option<u32>,
    pub seasons: Option<u32>,
    pub language: Vec<String>,
    pub country: Vec<String>,
    pub awards: Option<String>,
    pub budget: Option<String>,
    pub gross: Option<String>,
    pub release_date: Option<String>,
    pub metascore: Option<u8>,
    pub tagline: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ContentType {
    #[default]
    Movie,
    Series,
    MiniSeries,
    ShortFilm,
    Documentary,
    Unknown,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentType::Movie => write!(f, "Movie"),
            ContentType::Series => write!(f, "TV Series"),
            ContentType::MiniSeries => write!(f, "Mini-Series"),
            ContentType::ShortFilm => write!(f, "Short Film"),
            ContentType::Documentary => write!(f, "Documentary"),
            ContentType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub year: Option<u16>,
    pub content_type: ContentType,
    pub rating: Option<f32>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: &'static str,
    pub id: &'static str,
}

pub const CATEGORIES: &[Category] = &[
    Category { name: "Popular Movies", id: "moviemeter" },
    Category { name: "Top Rated Movies", id: "top" },
    Category { name: "Top Rated TV", id: "toptv" },
    Category { name: "Action", id: "genre/action" },
    Category { name: "Adventure", id: "genre/adventure" },
    Category { name: "Animation", id: "genre/animation" },
    Category { name: "Comedy", id: "genre/comedy" },
    Category { name: "Crime", id: "genre/crime" },
    Category { name: "Documentary", id: "genre/documentary" },
    Category { name: "Drama", id: "genre/drama" },
    Category { name: "Fantasy", id: "genre/fantasy" },
    Category { name: "Horror", id: "genre/horror" },
    Category { name: "Mystery", id: "genre/mystery" },
    Category { name: "Romance", id: "genre/romance" },
    Category { name: "Sci-Fi", id: "genre/sci-fi" },
    Category { name: "Thriller", id: "genre/thriller" },
    Category { name: "Western", id: "genre/western" },
    Category { name: "Box Office", id: "boxoffice" },
    Category { name: "Coming Soon", id: "comingsoon" },
    Category { name: "Award Winners", id: "oscar-winners" },
];

// ─── HTTP client wrapper (now powered by TMDB) ───────────────────────────────

pub struct ImdbClient {
    client: reqwest::Client,
    tmdb_key: String,
}

impl ImdbClient {
    pub fn new() -> Result<Self> {
        let config = crate::config::Config::load().unwrap_or_default();
        let client = reqwest::Client::builder()
            .user_agent(&config.network.user_agent)
            .timeout(std::time::Duration::from_secs(config.network.timeout_secs))
            .build()?;
        
        let tmdb_key = std::env::var("TMDB_API_KEY")
            .unwrap_or_else(|_| config.api.tmdb_key.clone());

        Ok(Self { client, tmdb_key })
    }

    fn get_api_key(&self) -> Result<String> {
        if self.tmdb_key.trim().is_empty() {
            anyhow::bail!(
                "TMDB API key is missing. Please configure it by running:\n\n\
                watchie config set-tmdb-key <key>\n\n\
                or by setting the TMDB_API_KEY environment variable."
            );
        }
        Ok(self.tmdb_key.trim().to_string())
    }

    async fn get_json_text(&self, url: &str) -> Result<String> {
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                anyhow::bail!(
                    "TMDB API request failed: Invalid API key. Please check your key."
                );
            }
            anyhow::bail!(
                "TMDB API request failed with status: {}",
                resp.status()
            );
        }
        Ok(resp.text().await?)
    }

    // ─── Search ──────────────────────────────────────────────────────────────

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let key = self.get_api_key()?;
        let encoded = urlencoding::encode(query);
        let url = format!(
            "https://api.themoviedb.org/3/search/multi?api_key={}&query={}&language=en-US&page=1",
            key, encoded
        );
        let text = self.get_json_text(&url).await?;
        let v: serde_json::Value = serde_json::from_str(&text)?;
        
        let mut results = Vec::new();
        if let Some(arr) = v["results"].as_array() {
            for item in arr.iter() {
                if results.len() >= limit {
                    break;
                }
                let media_type = item["media_type"].as_str().unwrap_or("");
                if media_type != "movie" && media_type != "tv" {
                    continue;
                }
                
                let id = item["id"].as_i64().unwrap_or(0);
                let is_movie = media_type == "movie";
                let title = if is_movie {
                    item["title"].as_str().unwrap_or("")
                } else {
                    item["name"].as_str().unwrap_or("")
                }.to_string();
                
                let release_date = if is_movie {
                    item["release_date"].as_str().unwrap_or("")
                } else {
                    item["first_air_date"].as_str().unwrap_or("")
                };
                
                let year = release_date.split('-').next().and_then(|y| y.parse::<u16>().ok());
                let rating = item["vote_average"].as_f64().map(|r| r as f32);
                let poster_path = item["poster_path"].as_str();
                let poster_url = poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
                
                results.push(SearchResult {
                    id: format!("{}:{}", media_type, id),
                    title,
                    year,
                    content_type: if is_movie { ContentType::Movie } else { ContentType::Series },
                    rating,
                    poster_url,
                });
            }
        }
        Ok(results)
    }

    // ─── Movie details ────────────────────────────────────────────────────────

    pub async fn get_movie(&self, id: &str) -> Result<Movie> {
        if id.starts_with("tt") {
            let key = self.get_api_key()?;
            let find_url = format!(
                "https://api.themoviedb.org/3/find/{}?api_key={}&external_source=imdb_id",
                id, key
            );
            let resp_text = self.get_json_text(&find_url).await?;
            let find_val: serde_json::Value = serde_json::from_str(&resp_text)?;
            
            if let Some(movie) = find_val["movie_results"].as_array().and_then(|arr| arr.first()) {
                let tmdb_id = movie["id"].as_i64().unwrap_or(0);
                return self.get_movie_by_tmdb_id(tmdb_id, true).await;
            } else if let Some(tv) = find_val["tv_results"].as_array().and_then(|arr| arr.first()) {
                let tmdb_id = tv["id"].as_i64().unwrap_or(0);
                return self.get_movie_by_tmdb_id(tmdb_id, false).await;
            }
            anyhow::bail!("Title with IMDB ID {} not found on TMDB.", id)
        } else if id.starts_with("movie:") {
            let tmdb_id = id["movie:".len()..].parse::<i64>()?;
            self.get_movie_by_tmdb_id(tmdb_id, true).await
        } else if id.starts_with("tv:") {
            let tmdb_id = id["tv:".len()..].parse::<i64>()?;
            self.get_movie_by_tmdb_id(tmdb_id, false).await
        } else if let Ok(tmdb_id) = id.parse::<i64>() {
            self.get_movie_by_tmdb_id(tmdb_id, true).await
        } else {
            anyhow::bail!("Invalid ID format: {}", id)
        }
    }

    async fn get_movie_by_tmdb_id(&self, tmdb_id: i64, is_movie: bool) -> Result<Movie> {
        let key = self.get_api_key()?;
        let path = if is_movie { "movie" } else { "tv" };
        let url = format!(
            "https://api.themoviedb.org/3/{}/{}?api_key={}&append_to_response=credits,external_ids,keywords",
            path, tmdb_id, key
        );
        let text = self.get_json_text(&url).await?;
        let v: serde_json::Value = serde_json::from_str(&text)?;
        
        if is_movie {
            Ok(map_tmdb_movie_to_movie(&v))
        } else {
            Ok(map_tmdb_tv_to_movie(&v))
        }
    }

    // ─── Category listing ─────────────────────────────────────────────────────

    pub async fn get_category(&self, category_id: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let key = self.get_api_key()?;
        
        let (url, is_movie) = match category_id {
            "top" => (
                format!("https://api.themoviedb.org/3/movie/top_rated?api_key={}&page=1", key),
                true
            ),
            "toptv" => (
                format!("https://api.themoviedb.org/3/tv/top_rated?api_key={}&page=1", key),
                false
            ),
            "moviemeter" => (
                format!("https://api.themoviedb.org/3/movie/popular?api_key={}&page=1", key),
                true
            ),
            "boxoffice" => (
                format!("https://api.themoviedb.org/3/trending/movie/week?api_key={}", key),
                true
            ),
            "comingsoon" => (
                format!("https://api.themoviedb.org/3/movie/upcoming?api_key={}", key),
                true
            ),
            "oscar-winners" => (
                format!("https://api.themoviedb.org/3/discover/movie?api_key={}&sort_by=vote_count.desc", key),
                true
            ),
            id if id.starts_with("genre/") => {
                let genre_name = &id[6..];
                let genre_id = match genre_name {
                    "action" => 28,
                    "adventure" => 12,
                    "animation" => 16,
                    "comedy" => 35,
                    "crime" => 80,
                    "documentary" => 99,
                    "drama" => 18,
                    "fantasy" => 14,
                    "horror" => 27,
                    "mystery" => 9648,
                    "romance" => 10749,
                    "sci-fi" => 878,
                    "thriller" => 53,
                    "western" => 37,
                    _ => 28, // default fallback
                };
                (
                    format!("https://api.themoviedb.org/3/discover/movie?api_key={}&with_genres={}&sort_by=popularity.desc", key, genre_id),
                    true
                )
            }
            _ => (
                format!("https://api.themoviedb.org/3/movie/popular?api_key={}&page=1", key),
                true
            ),
        };

        let text = self.get_json_text(&url).await?;
        let v: serde_json::Value = serde_json::from_str(&text)?;
        
        let mut results = Vec::new();
        if let Some(arr) = v["results"].as_array() {
            for item in arr.iter().take(limit) {
                let id = item["id"].as_i64().unwrap_or(0);
                let title = if is_movie {
                    item["title"].as_str().unwrap_or("")
                } else {
                    item["name"].as_str().unwrap_or("")
                }.to_string();
                
                let release_date = if is_movie {
                    item["release_date"].as_str().unwrap_or("")
                } else {
                    item["first_air_date"].as_str().unwrap_or("")
                };
                
                let year = release_date.split('-').next().and_then(|y| y.parse::<u16>().ok());
                let rating = item["vote_average"].as_f64().map(|r| r as f32);
                let poster_path = item["poster_path"].as_str();
                let poster_url = poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
                
                results.push(SearchResult {
                    id: format!("{}:{}", if is_movie { "movie" } else { "tv" }, id),
                    title,
                    year,
                    content_type: if is_movie { ContentType::Movie } else { ContentType::Series },
                    rating,
                    poster_url,
                });
            }
        }
        
        Ok(results)
    }

    // ─── Poster download ──────────────────────────────────────────────────────

    pub async fn download_poster(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.client.get(url).send().await?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

// ─── Mapping helpers ──────────────────────────────────────────────────────────

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;
    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }
    result.chars().rev().collect()
}

fn map_tmdb_movie_to_movie(v: &serde_json::Value) -> Movie {
    let id = v["external_ids"]["imdb_id"].as_str().unwrap_or("").to_string();
    let tmdb_id = v["id"].as_i64().unwrap_or(0);
    
    let title = v["title"].as_str().unwrap_or("").to_string();
    let release_date = v["release_date"].as_str().map(|s| s.to_string());
    let year = release_date.as_ref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse::<u16>().ok());
    let rating = v["vote_average"].as_f64().map(|r| r as f32);
    let votes = v["vote_count"].as_u64();
    
    let genres = v["genres"].as_array()
        .map(|arr| arr.iter().filter_map(|g| g["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let runtime = v["runtime"].as_u64().map(|r| r as u32);
    let plot = v["overview"].as_str().map(|s| s.to_string());
    
    let director = v["credits"]["crew"].as_array()
        .map(|arr| arr.iter()
            .filter(|c| c["job"].as_str() == Some("Director"))
            .filter_map(|c| c["name"].as_str())
            .map(String::from)
            .collect())
        .unwrap_or_default();
        
    let cast = v["credits"]["cast"].as_array()
        .map(|arr| arr.iter()
            .filter_map(|c| c["name"].as_str())
            .map(String::from)
            .collect())
        .unwrap_or_default();
        
    let poster_url = v["poster_path"].as_str()
        .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
        
    let imdb_url = format!("https://www.imdb.com/title/{}/", id);
    
    let keywords = v["keywords"]["keywords"].as_array()
        .map(|arr| arr.iter().filter_map(|k| k["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let budget = v["budget"].as_u64().map(|b| if b > 0 { format!("${}", format_number(b)) } else { "–".to_string() });
    let gross = v["revenue"].as_u64().map(|r| if r > 0 { format!("${}", format_number(r)) } else { "–".to_string() });
    let tagline = v["tagline"].as_str().map(String::from);
    
    let language = v["spoken_languages"].as_array()
        .map(|arr| arr.iter().filter_map(|l| l["english_name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let country = v["production_countries"].as_array()
        .map(|arr| arr.iter().filter_map(|c| c["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();

    Movie {
        id: if id.is_empty() { format!("movie:{}", tmdb_id) } else { id },
        title,
        year,
        rating,
        votes,
        genres,
        runtime,
        plot,
        director,
        cast,
        poster_url,
        imdb_url,
        content_type: ContentType::Movie,
        episodes: None,
        seasons: None,
        language,
        country,
        awards: None,
        budget,
        gross,
        release_date,
        metascore: rating.map(|r| (r * 10.0) as u8),
        tagline,
        keywords,
    }
}

fn map_tmdb_tv_to_movie(v: &serde_json::Value) -> Movie {
    let id = v["external_ids"]["imdb_id"].as_str().unwrap_or("").to_string();
    let tmdb_id = v["id"].as_i64().unwrap_or(0);
    
    let title = v["name"].as_str().unwrap_or("").to_string();
    let release_date = v["first_air_date"].as_str().map(|s| s.to_string());
    let year = release_date.as_ref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse::<u16>().ok());
    let rating = v["vote_average"].as_f64().map(|r| r as f32);
    let votes = v["vote_count"].as_u64();
    
    let genres = v["genres"].as_array()
        .map(|arr| arr.iter().filter_map(|g| g["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let runtime = v["episode_run_time"].as_array()
        .and_then(|arr| arr.first())
        .and_then(|r| r.as_u64())
        .map(|r| r as u32);
        
    let plot = v["overview"].as_str().map(|s| s.to_string());
    
    let mut director = v["created_by"].as_array()
        .map(|arr| arr.iter().filter_map(|c| c["name"].as_str()).map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    
    if director.is_empty() {
        director = v["credits"]["crew"].as_array()
            .map(|arr| arr.iter()
                .filter(|c| c["job"].as_str() == Some("Director") || c["job"].as_str() == Some("Executive Producer"))
                .filter_map(|c| c["name"].as_str())
                .map(String::from)
                .collect())
            .unwrap_or_default();
    }
        
    let cast = v["credits"]["cast"].as_array()
        .map(|arr| arr.iter()
            .filter_map(|c| c["name"].as_str())
            .map(String::from)
            .collect())
        .unwrap_or_default();
        
    let poster_url = v["poster_path"].as_str()
        .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
        
    let imdb_url = format!("https://www.imdb.com/title/{}/", id);
    
    let keywords = v["keywords"]["results"].as_array()
        .map(|arr| arr.iter().filter_map(|k| k["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let tagline = v["tagline"].as_str().map(String::from);
    
    let language = v["spoken_languages"].as_array()
        .map(|arr| arr.iter().filter_map(|l| l["english_name"].as_str()).map(String::from).collect())
        .unwrap_or_default();
        
    let country = v["production_countries"].as_array()
        .map(|arr| arr.iter().filter_map(|c| c["name"].as_str()).map(String::from).collect())
        .unwrap_or_default();

    let episodes = v["number_of_episodes"].as_u64().map(|e| e as u32);
    let seasons = v["number_of_seasons"].as_u64().map(|s| s as u32);

    Movie {
        id: if id.is_empty() { format!("tv:{}", tmdb_id) } else { id },
        title,
        year,
        rating,
        votes,
        genres,
        runtime,
        plot,
        director,
        cast,
        poster_url,
        imdb_url,
        content_type: ContentType::Series,
        episodes,
        seasons,
        language,
        country,
        awards: None,
        budget: None,
        gross: None,
        release_date,
        metascore: rating.map(|r| (r * 10.0) as u8),
        tagline,
        keywords,
    }
}
