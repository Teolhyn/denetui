use axum::{Json, Router, routing::get};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEV_TO_API: &str = "https://dev.to/api";

// Response from /articles/latest
#[derive(Debug, Deserialize)]
struct ArticleListItem {
    id: u64,
    title: String,
    positive_reactions_count: i32,
    published_at: DateTime<Utc>,
    user: User,
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

// What we send to TUI
#[derive(Debug, Serialize, Clone)]
struct Article {
    id: u64,
    title: String,
    author: String,
    content: String,
}

struct AppState {
    client: Client,
    api_key: String,
}

async fn fetch_latest_articles(
    client: &Client,
    api_key: &str,
) -> Result<Vec<ArticleListItem>, Box<dyn std::error::Error>> {
    let mut all_articles = Vec::new();

    // Fetch multiple pages to get yesterday's articles
    for page in 1..=10 {
        let url = format!("{}/articles/latest?per_page=100&page={}", DEV_TO_API, page);
        let response = client
            .get(&url)
            .header("api-key", api_key)
            .header("User-Agent", "denetui/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            break;
        }

        let text = response.text().await?;
        let articles: Vec<ArticleListItem> = serde_json::from_str(&text)?;

        if articles.is_empty() {
            break;
        }

        all_articles.extend(articles);
    }

    Ok(all_articles)
}

async fn fetch_article_content(
    client: &Client,
    api_key: &str,
    id: u64,
) -> Result<ArticleFull, Box<dyn std::error::Error>> {
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
    let yesterday = Utc::now() - Duration::days(1);
    let yesterday_start = yesterday.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let yesterday_end = yesterday.date_naive().and_hms_opt(23, 59, 59).unwrap();

    articles
        .into_iter()
        .filter(|a| {
            let published = a.published_at.naive_utc();
            published >= yesterday_start && published <= yesterday_end
        })
        .collect()
}

fn get_top_articles(mut articles: Vec<ArticleListItem>, count: usize) -> Vec<ArticleListItem> {
    articles.sort_by(|a, b| b.positive_reactions_count.cmp(&a.positive_reactions_count));
    articles.into_iter().take(count).collect()
}

async fn get_articles(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<Vec<Article>> {
    // Fetch latest articles
    let latest = match fetch_latest_articles(&state.client, &state.api_key).await {
        Ok(articles) => articles,
        Err(e) => {
            eprintln!("Failed to fetch articles: {}", e);
            return Json(vec![]);
        }
    };

    // Filter to yesterday and get top 27
    let yesterday_articles = filter_yesterday_articles(latest);
    let top_articles = get_top_articles(yesterday_articles, 27);

    // Fetch full content for each
    let mut result = Vec::new();
    for article_item in top_articles {
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

    Json(result)
}

#[tokio::main]
async fn main() {
    // Load .env from project root (parent of backend dir)
    dotenvy::from_path("../.env").expect("Failed to load .env file");

    let api_key = std::env::var("DEV_TO_API_KEY").expect("DEV_TO_API_KEY not set in .env");

    let state = Arc::new(AppState {
        client: Client::new(),
        api_key,
    });

    let app = Router::new()
        .route("/articles", get(get_articles))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
