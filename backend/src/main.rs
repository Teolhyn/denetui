use axum::{Json, Router, routing::get};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

const DEV_TO_API: &str = "https://dev.to/api";
const CACHE_DURATION_HOURS: i64 = 24;

// Response from /articles/latest
#[derive(Debug, Deserialize)]
struct ArticleListItem {
    id: u64,
    positive_reactions_count: i32,
    published_at: DateTime<Utc>,
}

// Response from /articles/{id}
#[derive(Debug, Deserialize)]
struct ArticleFull {
    id: u64,
    title: String,
    body_markdown: String,
    user: User,
}

#[derive(Debug, Deserialize, Clone)]
struct User {
    name: String,
}

// What we send to TUI (cached)
#[derive(Debug, Serialize, Clone)]
struct Article {
    id: u64,
    title: String,
    author: String,
    content: String,
}

struct Cache {
    articles: Vec<Article>,
    last_fetched: Option<DateTime<Utc>>,
}

struct AppState {
    client: Client,
    api_key: String,
    cache: RwLock<Cache>,
}

async fn fetch_latest_articles(
    client: &Client,
    api_key: &str,
) -> Result<Vec<ArticleListItem>, Box<dyn std::error::Error + Send + Sync>> {
    let mut all_articles = Vec::new();

    for page in 1..=10 {
        let url = format!("{}/articles/latest?per_page=1000&page={}", DEV_TO_API, page);
        println!("Fetching page {}...", page);
        let response = client
            .get(&url)
            .header("api-key", api_key)
            .header("User-Agent", "denetui/0.1.0")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            println!("Page {} failed with status: {}", page, status);
            break;
        }

        let text = response.text().await?;
        let articles: Vec<ArticleListItem> = match serde_json::from_str(&text) {
            Ok(a) => a,
            Err(e) => {
                println!("Failed to parse page {}: {}", page, e);
                break;
            }
        };

        println!(
            "Page {}: {} articles, oldest: {:?}",
            page,
            articles.len(),
            articles.last().map(|a| a.published_at)
        );

        if articles.is_empty() {
            break;
        }

        all_articles.extend(articles);
    }

    println!("Total fetched: {} articles", all_articles.len());
    Ok(all_articles)
}

async fn fetch_article_content(
    client: &Client,
    api_key: &str,
    id: u64,
) -> Result<ArticleFull, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/articles/{}", DEV_TO_API, id);
    let response = client
        .get(&url)
        .header("api-key", api_key)
        .header("User-Agent", "denetui/0.1.0")
        .send()
        .await?;

    let text = response.text().await?;
    let article: ArticleFull = serde_json::from_str(&text)?;
    Ok(article)
}

fn filter_yesterday_articles(articles: Vec<ArticleListItem>) -> Vec<ArticleListItem> {
    let now = Utc::now();
    let yesterday = now - Duration::days(1);
    let yesterday_start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let yesterday_end = yesterday.date_naive().and_hms_opt(23, 59, 59).unwrap();

    println!("Current time (UTC): {}", now);
    println!("Filtering for yesterday: {} to {}", yesterday_start, yesterday_end);

    let filtered: Vec<ArticleListItem> = articles
        .into_iter()
        .filter(|a| {
            let published = a.published_at.naive_utc();
            published >= yesterday_start && published <= yesterday_end
        })
        .collect();

    println!("Articles from yesterday: {}", filtered.len());
    filtered
}

fn get_top_articles(mut articles: Vec<ArticleListItem>, count: usize) -> Vec<ArticleListItem> {
    articles.sort_by(|a, b| b.positive_reactions_count.cmp(&a.positive_reactions_count));
    articles.into_iter().take(count).collect()
}

async fn refresh_cache(
    state: &AppState,
) -> Result<Vec<Article>, Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Starting cache refresh ===");
    println!("Fetching articles from dev.to API...");

    let latest = fetch_latest_articles(&state.client, &state.api_key).await?;
    let yesterday_articles = filter_yesterday_articles(latest);

    println!("Getting top 27 from {} articles", yesterday_articles.len());
    let top_articles = get_top_articles(yesterday_articles, 27);
    println!("Top articles to fetch: {}", top_articles.len());

    let mut result = Vec::new();
    for (i, article_item) in top_articles.iter().enumerate() {
        println!("Fetching article {}/{}: id={}", i + 1, top_articles.len(), article_item.id);
        match fetch_article_content(&state.client, &state.api_key, article_item.id).await {
            Ok(full) => {
                result.push(Article {
                    id: full.id,
                    title: full.title,
                    author: full.user.name,
                    content: full.body_markdown,
                });
            }
            Err(e) => {
                eprintln!("Failed to fetch article {}: {}", article_item.id, e);
            }
        }
    }

    println!("=== Cache refresh complete: {} articles ===", result.len());

    let mut cache = state.cache.write().await;
    cache.articles = result.clone();
    cache.last_fetched = Some(Utc::now());

    Ok(result)
}

async fn get_articles(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<Vec<Article>> {
    let should_refresh = {
        let cache = state.cache.read().await;
        match cache.last_fetched {
            None => true,
            Some(last_fetched) => {
                let age = Utc::now() - last_fetched;
                age > Duration::hours(CACHE_DURATION_HOURS)
            }
        }
    };

    if should_refresh {
        match refresh_cache(&state).await {
            Ok(articles) => Json(articles),
            Err(e) => {
                eprintln!("Failed to refresh cache: {}", e);
                let cache = state.cache.read().await;
                Json(cache.articles.clone())
            }
        }
    } else {
        println!("Serving from cache");
        let cache = state.cache.read().await;
        Json(cache.articles.clone())
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    let api_key = std::env::var("DEV_TO_API_KEY").expect("DEV_TO_API_KEY not set in .env");

    let state = Arc::new(AppState {
        client: Client::new(),
        api_key,
        cache: RwLock::new(Cache {
            articles: Vec::new(),
            last_fetched: None,
        }),
    });

    let app = Router::new()
        .route("/articles", get(get_articles))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
