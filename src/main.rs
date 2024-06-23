use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Serialize;
use serde::Deserialize;
use urlencoding::encode;
use sha2::{Sha256, Digest};
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use std::io::Read;
use log::{info, warn, error};
use std::process::exit;
use actix_files as fs;

fn read_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    if let Ok(file) = File::open("config.txt") {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    config.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                }
            }
        }
    }
    config
}

#[derive(Serialize, Clone)]
struct Channel {
    name: String,
    url: String,
    icon_url: String,
}

#[derive(Serialize)]
struct Category {
    name: String,
    channels: Vec<Channel>,
}

#[derive(Deserialize)]
struct PaginationQuery {
    page_size: Option<usize>,
    page: Option<usize>,
}

async fn index(categories: web::Data<HashMap<String, Category>>) -> impl Responder {
    let mut sorted_categories: Vec<(&String, &Category)> = categories.iter().collect();
    sorted_categories.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let html = format!(r#"
        <html>
        <head>
		<link rel="stylesheet" href="/static/styles.css">

            <title>M3U Playlist</title>
        </head>
        <body>
            <h1>M3U Playlist</h1>
            <form action="/search" method="get">
                <input type="text" name="q" placeholder="Search channels...">
                <input type="submit" value="Search">
            </form>
            <ul id="categoryList">
                {}
            </ul>
        </body>
        </html>
    "#, 
        sorted_categories.iter()
            .filter(|(_, category)| !category.channels.is_empty())
            .map(|(name, category)| {
                let display_name = if name.is_empty() { "No Category" } else { name };
                format!("<li><a href=\"/category/{}\">{} ({})</a></li>", encode(name), display_name, category.channels.len())
            })
            .collect::<String>()
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}


async fn cache_image(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cache_dir = "image_cache";
    tokio::fs::create_dir_all(cache_dir).await?;

    let mut hasher = Sha256::new();
    hasher.update(url);
    let filename = format!("{:x}.jpg", hasher.finalize());
    let cache_path = Path::new(cache_dir).join(&filename);

    if !cache_path.exists() {
        let response = reqwest::get(url).await?;
        
        // Check if the content type is an image
        if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
            if !content_type.to_str()?.starts_with("image/") {
                return Err("Downloaded content is not an image".into());
            }
        }

        let bytes = response.bytes().await?;
        
        // Check if the downloaded content is not empty
        if bytes.is_empty() {
            return Err("Downloaded image is empty".into());
        }

        let mut file = TokioFile::create(&cache_path).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;
    }

    Ok(format!("/image_cache/{}", filename))
}




async fn cached_image(path: web::Path<String>) -> impl Responder {
    let cache_dir = "image_cache";
    let file_path = Path::new(cache_dir).join(path.into_inner());

    if file_path.exists() {
        match File::open(&file_path) {
            Ok(mut file) => {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    HttpResponse::Ok()
                        .content_type("image/jpeg")
                        .body(actix_web::web::Bytes::from(buffer))
                } else {
                    HttpResponse::InternalServerError().finish()
                }
            }
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

async fn lazy_load_image(query: web::Query<HashMap<String, String>>) -> impl Responder {
    if let Some(url) = query.get("url") {
        if url.is_empty() || url == "/static/placeholder.png" {
            // Return the placeholder image if the URL is empty or already the placeholder
            return HttpResponse::Ok()
                .content_type("text/plain")
                .body("/static/placeholder.png");
        }

        // Try to cache the image and return the cached URL
        match cache_image(url).await {
            Ok(cached_url) => HttpResponse::Ok()
                .content_type("text/plain")
                .body(cached_url),
            Err(e) => {
                // If caching fails, log the error and return the placeholder image URL
                error!("Failed to cache image: {}", e);
                HttpResponse::Ok()
                    .content_type("text/plain")
                    .body("/static/placeholder.png")
            }
        }
    } else {
        HttpResponse::BadRequest().finish()
    }
}








async fn category(
    path: web::Path<String>,
    query: web::Query<PaginationQuery>,
    categories: web::Data<HashMap<String, Category>>
) -> impl Responder {
    let category_name = urlencoding::decode(&path.into_inner()).expect("Failed to decode category name").into_owned();
    
    if let Some(category) = categories.get(&category_name) {
        let page_size = query.page_size.unwrap_or(100);
        let page = query.page.unwrap_or(1);
        
        let total_channels = category.channels.len();
        let total_pages = (total_channels + page_size - 1) / page_size;
        
        let start_index = (page - 1) * page_size;
        let end_index = std::cmp::min(start_index + page_size, total_channels);
        
let channels_html = category.channels[start_index..end_index].iter().map(|channel| {
    format!(r#"
        <li>
            <img src="/static/placeholder.png" data-src="/lazy_load_image?url={}" alt="{}" class="thumbnail lazyload">
            <a href="{}" target="_blank">{}</a>
            <button onclick="playChannel('{}', 'mpv')">Play with mpv</button>
            <button onclick="playChannel('{}', 'vlc')">Play with VLC</button>
        </li>
    "#, encode(&channel.icon_url), channel.name, channel.url, channel.name, channel.url, channel.url)
}).collect::<String>();


        let pagination_html = generate_pagination_html(page, total_pages, page_size, &category_name);

    let html = format!(r#"
        <html>
        <head>
            <link rel="stylesheet" href="/static/styles.css">
            <title>{} Channels</title>
            <script src="/static/script.js"></script>
        </head>
        <body>
            <h1>{} Channels</h1>
            <p>Showing {}-{} of {} channels</p>
            <div>
                <a href="/category/{}?page_size=100">100</a> |
                <a href="/category/{}?page_size=1000">1000</a> |
                <a href="/category/{}?page_size=10000">10000</a> |
                <a href="/category/{}?page_size={}">All</a>
            </div>
            <div class="view-controls">
                <button onclick="setView('list')">List View</button>
                <button onclick="setView('grid')">Grid View</button>
            </div>
            {}
            <ul id="channelList" class="list-view">
                {}
            </ul>
            {}
        </body>
        </html>
    "#,
            category.name,
            category.name,
            start_index + 1,
            end_index,
            total_channels,
            encode(&category_name),
            encode(&category_name),
            encode(&category_name),
            encode(&category_name),
            total_channels,
            pagination_html,
            channels_html,
            pagination_html
        );

        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else {
        HttpResponse::NotFound().finish()
    }
}

fn generate_pagination_html(current_page: usize, total_pages: usize, page_size: usize, base_url: &str) -> String {
    let mut pagination_html = String::new();
    
    if total_pages > 1 {
        pagination_html.push_str("<div class='pagination'>");
        
        if current_page > 1 {
            pagination_html.push_str(&format!("<a href='{}?page_size={}&page={}'>Previous</a> ", 
                base_url, page_size, current_page - 1));
        }
        
        let start_page = std::cmp::max(1, current_page.saturating_sub(2));
        let end_page = std::cmp::min(total_pages, start_page + 4);
        
        for i in start_page..=end_page {
            if i == current_page {
                pagination_html.push_str(&format!("<span>{}</span> ", i));
            } else {
                pagination_html.push_str(&format!("<a href='{}?page_size={}&page={}'>{}</a> ", 
                    base_url, page_size, i, i));
            }
        }
        
        if current_page < total_pages {
            pagination_html.push_str(&format!("<a href='{}?page_size={}&page={}'>Next</a>", 
                base_url, page_size, current_page + 1));
        }
        
        pagination_html.push_str("</div>");
    }
    
    pagination_html
}



async fn play(
    path: web::Path<(String, String)>,
    config: web::Data<HashMap<String, String>>
) -> impl Responder {
    let (player, url) = path.into_inner();
    let url = urlencoding::decode(&url).expect("Failed to decode URL").into_owned();
    
    let player_path = match player.as_str() {
        "mpv" => config.get("mpv_path").cloned(),
        "vlc" => config.get("vlc_path").cloned(),
        _ => return HttpResponse::BadRequest().body("Invalid player specified"),
    };

    let player_path = match player_path {
        Some(path) => path,
        None => return HttpResponse::InternalServerError().body(format!("{} path not specified in config", player)),
    };

    let mut command = Command::new(&player_path);
    command.arg(&url);

    match command.spawn() {
        Ok(mut child) => {
            match child.wait() {
                Ok(status) => {
                    if status.success() {
                        HttpResponse::Ok().finish()
                    } else {
                        warn!("{} exited with non-zero status", player);
                        HttpResponse::InternalServerError().body(format!("{} exited with non-zero status", player))
                    }
                }
                Err(e) => {
                    error!("Failed to wait for {} process: {}", player, e);
                    HttpResponse::InternalServerError().body(format!("Failed to wait for {} process", player))
                }
            }
        }
        Err(e) => {
            error!("Failed to execute {}: {}", player, e);
            HttpResponse::InternalServerError().body(format!("Failed to execute {}", player))
        }
    }
}





fn clean_channel_name(name: &str) -> String {
    let name = name.trim_start_matches("tvg-id=\"\"").trim();
    let name = name.trim_start_matches("tvg-name=\"").trim();
    if let Some(index) = name.find('"') {
        name[..index].trim().to_string()
    } else {
        name.to_string()
    }
}

async fn search(
    query: web::Query<HashMap<String, String>>,
    pagination: web::Query<PaginationQuery>,
    categories: web::Data<HashMap<String, Category>>
) -> impl Responder {
    if let Some(q) = query.get("q") {
        let search_term = q.to_lowercase();
        let mut results = Vec::new();

        for category in categories.values() {
            for channel in &category.channels {
                if channel.name.to_lowercase().contains(&search_term) {
                    results.push((category.name.clone(), channel.clone()));
                }
            }
        }

        results.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        let page_size = pagination.page_size.unwrap_or(100);
        let page = pagination.page.unwrap_or(1);
        
        let total_results = results.len();
        let total_pages = (total_results + page_size - 1) / page_size;
        
        let start_index = (page - 1) * page_size;
        let end_index = std::cmp::min(start_index + page_size, total_results);

        let results_html = results[start_index..end_index].iter().map(|(category_name, channel)| {
            format!(r#"
                <li>
                    <img src="/static/placeholder.png" data-src="/lazy_load_image?url={}" alt="{}" class="thumbnail lazyload">
                    <a href="{}" target="_blank">{}</a> (Category: {})
                    <button onclick="playChannel('{}', 'mpv')">Play with mpv</button>
                    <button onclick="playChannel('{}', 'vlc')">Play with VLC</button>
                </li>
            "#, encode(&channel.icon_url), channel.name, channel.url, channel.name, category_name, channel.url, channel.url)
        }).collect::<String>();

        let pagination_html = generate_pagination_html(page, total_pages, page_size, &format!("/search?q={}", encode(q)));

        let html = format!(r#"
            <html>
            <head>
                <link rel="stylesheet" href="/static/styles.css">
                <title>Search Results</title>
                <script src="/static/script.js"></script>
            </head>
            <body>
                <h1>Search Results for "{}"</h1>
                <p>Showing {}-{} of {} channels</p>
                <div>
                    <a href="/search?q={}&page_size=100">100</a> |
                    <a href="/search?q={}&page_size=1000">1000</a> |
                    <a href="/search?q={}&page_size=10000">10000</a> |
                    <a href="/search?q={}&page_size={}">All</a>
                </div>
                <div class="view-controls">
                    <button onclick="setView('list')">List View</button>
                    <button onclick="setView('grid')">Grid View</button>
                </div>
                <ul id="searchResults" class="list-view">
                    {}
                </ul>
                {}
            </body>
            </html>
        "#, 
            q,
            start_index + 1,
            end_index,
            total_results,
            encode(q),
            encode(q),
            encode(q),
            encode(q),
            total_results,
            results_html,
            pagination_html
        );

        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else {
        HttpResponse::BadRequest().finish()
    }
}



#[actix_web::main]
async fn main() {
    // Initialize logger
    env_logger::init();

    info!("Starting M3U Playlist Browser");
    info!("Access the application at http://localhost:8080");

    let config = read_config();
    let playlist_path = config.get("playlist_path").cloned().unwrap_or_else(|| "playlist.m3u".to_string());
    
    // Check if playlist file exists
    if !std::path::Path::new(&playlist_path).exists() {
        error!("Playlist file not found: {}", playlist_path);
        exit(1);
    }

    let file = match File::open(&playlist_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open playlist file: {}", e);
            exit(1);
        }
    };

    let reader = BufReader::new(file);

    let mut categories: HashMap<String, Category> = HashMap::new();
    let mut current_category: Option<String> = None;
    let mut current_channel: Option<Channel> = None;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");

        if line.starts_with("#EXTINF") {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 1 {
				let mut category_name = if parts[0].contains("group-title=\"\"") || parts[0].contains("group=\"\"") {
					"No Category".to_string()
				} else {
					"Uncategorized".to_string()
				};
                let mut channel_name = clean_channel_name(parts[0].trim_start_matches("#EXTINF:-1").trim());
                let mut icon_url = "".to_string();

                // Check for the most common ways to identify categories or groups
                if let Some(group_title_index) = parts[0].find("group-title=\"") {
                    let group_title_start = group_title_index + "group-title=\"".len();
                    if let Some(group_title_end) = parts[0][group_title_start..].find('"') {
                        category_name = parts[0][group_title_start..group_title_start + group_title_end].to_string();
                        channel_name = clean_channel_name(parts[1].trim());
                    }
                } else if let Some(group_index) = parts[0].find("group=\"") {
                    let group_start = group_index + "group=\"".len();
                    if let Some(group_end) = parts[0][group_start..].find('"') {
                        category_name = parts[0][group_start..group_start + group_end].to_string();
                        channel_name = clean_channel_name(parts[1].trim());
                    }
                }

                // Check for the icon or thumbnail URL
                if let Some(tvg_logo_index) = parts[0].find("tvg-logo=\"") {
                    let tvg_logo_start = tvg_logo_index + "tvg-logo=\"".len();
                    if let Some(tvg_logo_end) = parts[0][tvg_logo_start..].find('"') {
                        icon_url = parts[0][tvg_logo_start..tvg_logo_start + tvg_logo_end].to_string();
                    }
                }

				if category_name.is_empty() {
					category_name = "No Category".to_string();
				}

				current_category = Some(category_name.clone());
				current_channel = Some(Channel {
					name: channel_name,
					url: "".to_string(),
					icon_url: if icon_url.is_empty() { "/static/placeholder.png".to_string() } else { icon_url },
				});

            }
        } else if line.starts_with("http") {
            if let Some(category_name) = current_category.clone() {
                if let Some(channel) = current_channel.take() {
                    let category = categories.entry(category_name.clone()).or_insert(Category {
                        name: category_name,
                        channels: Vec::new(),
                    });
                    category.channels.push(Channel {
                        name: channel.name,
                        url: line.clone(),
                        icon_url: channel.icon_url,
                    });
                }
            }
        }
    }

    let categories_data = web::Data::new(categories);
    
    // Clone config before moving it into web::Data
    let config_clone = config.clone();
    let config_data = web::Data::new(config);

    // Check if VLC and MPV paths are specified
    if !config_clone.contains_key("vlc_path") {
        warn!("VLC path not specified in config.txt");
    }
    if !config_clone.contains_key("mpv_path") {
        warn!("MPV path not specified in config.txt");
    }

    info!("Starting server on http://localhost:8080");
    match HttpServer::new(move || {
        App::new()
            .app_data(categories_data.clone())
            .app_data(config_data.clone())
			.service(fs::Files::new("/static", "./static").show_files_listing())
            .route("/", web::get().to(index))
            .route("/category/{name}", web::get().to(category))
            .route("/play/{player}/{url}", web::get().to(play))
            .route("/image_cache/{filename}", web::get().to(cached_image))
            .route("/lazy_load_image", web::get().to(lazy_load_image))
            .route("/search", web::get().to(search))
    })
    .bind("127.0.0.1:8080")
    {
        Ok(server) => {
            if let Err(e) = server.run().await {
                error!("Server error: {}", e);
                exit(1);
            }
        }
        Err(e) => {
            error!("Failed to bind to address: {}", e);
            exit(1);
        }
    }
}