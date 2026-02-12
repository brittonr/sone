mod audio;
mod tidal_api;

use audio::AudioPlayer;
use base64::Engine;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use std::time::{SystemTime, UNIX_EPOCH};
use tidal_api::{AuthTokens, DeviceCode, HomePageResponse, PaginatedTracks, StreamInfo, TidalAlbumDetail, TidalArtistDetail, TidalClient, TidalCredit, TidalLyrics, TidalPlaylist, TidalSearchResults, TidalTrack};


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Settings {
    auth_tokens: Option<AuthTokens>,
    volume: f32,
    last_track_id: Option<u64>,
    #[serde(default)]
    is_pkce: bool,
}

const CACHE_TTL_SECS: u64 = 12 * 60 * 60; // 12 hours

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CacheMeta {
    #[serde(default)]
    home_page_ts: u64,
    #[serde(default)]
    favorite_artists_ts: u64,
}

pub struct AppState {
    audio_player: AudioPlayer,
    tidal_client: Mutex<TidalClient>,
    settings_path: PathBuf,
    cache_dir: PathBuf,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl AppState {
    fn new() -> Self {
        // Get config dir
        let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.push("tide-vibe");
        fs::create_dir_all(&config_dir).ok();

        let settings_path = config_dir.join("settings.json");
        let cache_dir = config_dir.join("cache");
        fs::create_dir_all(&cache_dir).ok();

        Self {
            audio_player: AudioPlayer::new(),
            tidal_client: Mutex::new(TidalClient::new()),
            settings_path,
            cache_dir,
        }
    }

    fn load_settings(&self) -> Option<Settings> {
        if let Ok(content) = fs::read_to_string(&self.settings_path) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(&self.settings_path, json).map_err(|e| e.to_string())
    }

    // ---- Cache helpers ----

    fn load_cache_meta(&self) -> CacheMeta {
        let path = self.cache_dir.join("cache_meta.json");
        if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            CacheMeta::default()
        }
    }

    fn save_cache_meta(&self, meta: &CacheMeta) -> Result<(), String> {
        let path = self.cache_dir.join("cache_meta.json");
        let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    fn read_cache_file(&self, name: &str) -> Option<String> {
        let path = self.cache_dir.join(name);
        fs::read_to_string(&path).ok()
    }

    fn write_cache_file(&self, name: &str, content: &str) -> Result<(), String> {
        let path = self.cache_dir.join(name);
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    fn is_cache_fresh(&self, timestamp: u64) -> bool {
        let now = now_secs();
        now.saturating_sub(timestamp) < CACHE_TTL_SECS
    }
}

// ==================== Tidal Authentication ====================

#[tauri::command]
fn start_tidal_auth(state: State<AppState>) -> Result<DeviceCode, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.start_device_auth()
}

#[tauri::command(rename_all = "camelCase")]
fn poll_tidal_auth(state: State<AppState>, device_code: String) -> Result<AuthTokens, String> {
    let mut client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let tokens = client.poll_for_token(&device_code)?;

    // Save tokens to settings
    let settings = Settings {
        auth_tokens: Some(tokens.clone()),
        volume: 1.0,
        last_track_id: None,
        is_pkce: false,
    };
    state.save_settings(&settings)?;

    Ok(tokens)
}

#[tauri::command]
fn load_saved_auth(state: State<AppState>) -> Result<Option<AuthTokens>, String> {
    println!("DEBUG: Loading saved auth from {:?}", state.settings_path);
    if let Some(settings) = state.load_settings() {
        println!("DEBUG: Settings loaded, auth_tokens present: {}, is_pkce: {}", settings.auth_tokens.is_some(), settings.is_pkce);
        if let Some(tokens) = settings.auth_tokens {
            // Restore tokens to client
            let mut client = state.tidal_client.lock().map_err(|e| e.to_string())?;
            client.tokens = Some(tokens.clone());
            client.is_pkce = settings.is_pkce;
            println!("DEBUG: Tokens restored to client, user_id: {:?}, is_pkce: {}", tokens.user_id, settings.is_pkce);
            return Ok(Some(tokens));
        }
    } else {
        println!("DEBUG: No settings file found");
    }
    Ok(None)
}

#[tauri::command]
fn refresh_tidal_auth(state: State<AppState>) -> Result<AuthTokens, String> {
    let mut client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let new_tokens = client.refresh_token()?;

    // Save refreshed tokens to settings
    let mut settings = state.load_settings().unwrap_or(Settings {
        auth_tokens: None,
        volume: 1.0,
        last_track_id: None,
        is_pkce: client.is_pkce,
    });
    settings.auth_tokens = Some(new_tokens.clone());
    state.save_settings(&settings)?;

    Ok(new_tokens)
}

const PKCE_REDIRECT_URI: &str = "https://tidal.com/android/login/auth";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PkceAuthParams {
    authorize_url: String,
    code_verifier: String,
    client_unique_key: String,
}

#[tauri::command]
fn start_pkce_auth() -> Result<PkceAuthParams, String> {
    // Generate PKCE values
    let mut rng = rand::rng();
    let random_bytes: [u8; 32] = rng.random();
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(hasher.finalize());

    let client_unique_key = format!("{:016x}", rng.random::<u64>());

    let authorize_url = format!(
        "https://login.tidal.com/authorize?response_type=code&redirect_uri={}&client_id=REDACTED_CLIENT_ID_PKCE&lang=EN&appMode=android&client_unique_key={}&code_challenge={}&code_challenge_method=S256&restrict_signup=true",
        "https%3A%2F%2Ftidal.com%2Fandroid%2Flogin%2Fauth",
        client_unique_key,
        code_challenge,
    );

    Ok(PkceAuthParams {
        authorize_url,
        code_verifier,
        client_unique_key,
    })
}

#[tauri::command(rename_all = "camelCase")]
fn complete_pkce_auth(
    state: State<AppState>,
    code: String,
    code_verifier: String,
    client_unique_key: String,
) -> Result<AuthTokens, String> {
    let mut client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let tokens = client.exchange_pkce_code(&code, &code_verifier, PKCE_REDIRECT_URI, &client_unique_key)?;

    // Save tokens and mark as PKCE session
    let mut settings = state.load_settings().unwrap_or(Settings {
        auth_tokens: None,
        volume: 1.0,
        last_track_id: None,
        is_pkce: false,
    });
    settings.auth_tokens = Some(tokens.clone());
    settings.is_pkce = true;
    state.save_settings(&settings)?;

    Ok(tokens)
}

#[tauri::command]
fn logout(state: State<AppState>) -> Result<(), String> {
    // Clear tokens
    let mut client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.tokens = None;
    client.is_pkce = false;

    // Delete settings file
    fs::remove_file(&state.settings_path).ok();

    // Clear all cached data
    if let Ok(entries) = fs::read_dir(&state.cache_dir) {
        for entry in entries.flatten() {
            fs::remove_file(entry.path()).ok();
        }
    }

    Ok(())
}

#[tauri::command]
fn get_session_user_id(state: State<AppState>) -> Result<u64, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_session_info()
}

#[tauri::command(rename_all = "camelCase")]
fn get_user_profile(state: State<AppState>, user_id: u64) -> Result<(String, Option<String>), String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_user_profile(user_id)
}

// ==================== Tidal API Calls ====================

#[tauri::command(rename_all = "camelCase")]
fn get_user_playlists(state: State<AppState>, user_id: u64) -> Result<Vec<TidalPlaylist>, String> {
    println!("DEBUG: Getting playlists for user_id: {}", user_id);
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let result = client.get_user_playlists(user_id);
    match &result {
        Ok(playlists) => println!("DEBUG: Got {} playlists", playlists.len()),
        Err(e) => {
            println!("DEBUG: Failed to get playlists: {}", e);
        }
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
fn get_playlist_tracks(
    state: State<AppState>,
    playlist_id: String,
) -> Result<Vec<TidalTrack>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_playlist_tracks(&playlist_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_favorite_tracks(
    state: State<AppState>,
    user_id: u64,
    offset: u32,
    limit: u32,
) -> Result<PaginatedTracks, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_favorite_tracks(user_id, offset, limit)
}

#[tauri::command(rename_all = "camelCase")]
fn get_favorite_track_ids(state: State<AppState>, user_id: u64) -> Result<Vec<u64>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_favorite_track_ids(user_id)
}

#[tauri::command(rename_all = "camelCase")]
fn is_track_favorited(state: State<AppState>, user_id: u64, track_id: u64) -> Result<bool, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.is_track_favorited(user_id, track_id)
}

#[tauri::command(rename_all = "camelCase")]
fn add_favorite_track(state: State<AppState>, user_id: u64, track_id: u64) -> Result<(), String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.add_favorite_track(user_id, track_id)
}

#[tauri::command(rename_all = "camelCase")]
fn remove_favorite_track(state: State<AppState>, user_id: u64, track_id: u64) -> Result<(), String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.remove_favorite_track(user_id, track_id)
}

#[tauri::command(rename_all = "camelCase")]
fn is_album_favorited(state: State<AppState>, user_id: u64, album_id: u64) -> Result<bool, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.is_album_favorited(user_id, album_id)
}

#[tauri::command(rename_all = "camelCase")]
fn add_favorite_album(state: State<AppState>, user_id: u64, album_id: u64) -> Result<(), String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.add_favorite_album(user_id, album_id)
}

#[tauri::command(rename_all = "camelCase")]
fn remove_favorite_album(state: State<AppState>, user_id: u64, album_id: u64) -> Result<(), String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.remove_favorite_album(user_id, album_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_album_detail(state: State<AppState>, album_id: u64) -> Result<TidalAlbumDetail, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_album_detail(album_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_album_tracks(
    state: State<AppState>,
    album_id: u64,
    offset: u32,
    limit: u32,
) -> Result<PaginatedTracks, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_album_tracks(album_id, offset, limit)
}

#[tauri::command(rename_all = "camelCase")]
fn get_stream_url(state: State<AppState>, track_id: u64, quality: String) -> Result<StreamInfo, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_stream_url(track_id, &quality)
}

// ==================== Search ====================

#[tauri::command(rename_all = "camelCase")]
fn search_tidal(state: State<AppState>, query: String, limit: u32) -> Result<TidalSearchResults, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.search(&query, limit)
}

// ==================== Home Page & Pages API ====================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HomePageCached {
    home: HomePageResponse,
    is_stale: bool,
}

#[tauri::command]
fn get_home_page(state: State<AppState>) -> Result<HomePageCached, String> {
    let meta = state.load_cache_meta();

    // Try to serve from cache first
    if meta.home_page_ts > 0 {
        if let Some(cached) = state.read_cache_file("home_page.json") {
            if let Ok(home) = serde_json::from_str::<HomePageResponse>(&cached) {
                let is_stale = !state.is_cache_fresh(meta.home_page_ts);
                return Ok(HomePageCached { home, is_stale });
            }
        }
    }

    // No valid cache — fetch fresh
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let home = client.get_home_page()?;

    // Cache the result
    if let Ok(json) = serde_json::to_string(&home) {
        state.write_cache_file("home_page.json", &json).ok();
        let mut meta = state.load_cache_meta();
        meta.home_page_ts = now_secs();
        state.save_cache_meta(&meta).ok();
    }

    Ok(HomePageCached { home, is_stale: false })
}

#[tauri::command]
fn refresh_home_page(state: State<AppState>) -> Result<HomePageResponse, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let home = client.get_home_page()?;

    // Update cache
    if let Ok(json) = serde_json::to_string(&home) {
        state.write_cache_file("home_page.json", &json).ok();
        let mut meta = state.load_cache_meta();
        meta.home_page_ts = now_secs();
        state.save_cache_meta(&meta).ok();
    }

    Ok(home)
}

#[tauri::command(rename_all = "camelCase")]
fn get_favorite_artists(state: State<AppState>, user_id: u64, limit: u32) -> Result<Vec<TidalArtistDetail>, String> {
    let meta = state.load_cache_meta();

    // Try cache
    if state.is_cache_fresh(meta.favorite_artists_ts) {
        if let Some(cached) = state.read_cache_file("favorite_artists.json") {
            if let Ok(artists) = serde_json::from_str::<Vec<TidalArtistDetail>>(&cached) {
                return Ok(artists);
            }
        }
    }

    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let artists = client.get_favorite_artists(user_id, limit)?;

    // Cache
    if let Ok(json) = serde_json::to_string(&artists) {
        state.write_cache_file("favorite_artists.json", &json).ok();
        let mut meta = state.load_cache_meta();
        meta.favorite_artists_ts = now_secs();
        state.save_cache_meta(&meta).ok();
    }

    Ok(artists)
}

#[tauri::command(rename_all = "camelCase")]
fn get_page_section(state: State<AppState>, api_path: String) -> Result<HomePageResponse, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_page(&api_path)
}

#[tauri::command(rename_all = "camelCase")]
fn get_mix_items(state: State<AppState>, mix_id: String) -> Result<Vec<TidalTrack>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_mix_items(&mix_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_artist_detail(state: State<AppState>, artist_id: u64) -> Result<TidalArtistDetail, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_artist_detail(artist_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_artist_top_tracks(state: State<AppState>, artist_id: u64, limit: u32) -> Result<Vec<TidalTrack>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_artist_top_tracks(artist_id, limit)
}

#[tauri::command(rename_all = "camelCase")]
fn get_artist_albums(state: State<AppState>, artist_id: u64, limit: u32) -> Result<Vec<TidalAlbumDetail>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_artist_albums(artist_id, limit)
}

#[tauri::command(rename_all = "camelCase")]
fn get_artist_bio(state: State<AppState>, artist_id: u64) -> Result<String, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_artist_bio(artist_id)
}

/// Debug command: returns the raw JSON structure of multiple page endpoints
/// so we can see what format Tidal is using and what sections are available.
#[tauri::command]
fn debug_home_page_raw(state: State<AppState>) -> Result<String, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    let tokens = client.tokens.as_ref().ok_or("Not authenticated")?;

    let http = reqwest::blocking::Client::new();
    let mut summary = String::new();

    let endpoints = [
        "pages/home",
        "pages/for_you",
        "pages/my_collection_recently_played",
        "pages/my_collection_my_mixes",
        "pages/explore",
        "pages/suggested_new_tracks_for_you",
        "pages/suggested_new_albums_for_you",
        "pages/show/essential_album",
    ];

    for endpoint in &endpoints {
        summary.push_str(&format!("=== {} ===\n", endpoint));

        let response = http
            .get(format!("https://api.tidal.com/v1/{}", endpoint))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US"), ("deviceType", "BROWSER"), ("locale", "en_US")])
            .send();

        match response {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    summary.push_str(&format!("  ERROR: status {}\n\n", status));
                    continue;
                }
                let body = resp.text().unwrap_or_default();
                let json: serde_json::Value = match serde_json::from_str(&body) {
                    Ok(j) => j,
                    Err(e) => { summary.push_str(&format!("  PARSE ERROR: {}\n\n", e)); continue; }
                };

                summary.push_str(&format!("  Top-level keys: {:?}\n",
                    json.as_object().map(|o| o.keys().collect::<Vec<_>>()).unwrap_or_default()));

                // V1
                if let Some(rows) = json.get("rows").and_then(|r| r.as_array()) {
                    summary.push_str(&format!("  FORMAT: V1 (rows), {} rows\n", rows.len()));
                    for (i, row) in rows.iter().enumerate() {
                        if let Some(modules) = row.get("modules").and_then(|m| m.as_array()) {
                            for module in modules {
                                let mtype = module.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                                let title = module.get("title").and_then(|t| t.as_str()).unwrap_or("(no title)");
                                let item_count = module.get("pagedList")
                                    .and_then(|pl| pl.get("items"))
                                    .and_then(|i| i.as_array())
                                    .map(|a| a.len())
                                    .or_else(|| module.get("highlights").and_then(|h| h.as_array()).map(|a| a.len()))
                                    .unwrap_or(0);
                                let has_more = module.get("showMore").is_some();
                                summary.push_str(&format!("    Row {}: type={:<30} title=\"{}\" items={} more={}\n",
                                    i, mtype, title, item_count, has_more));
                            }
                        }
                    }
                }

                // V2
                if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
                    summary.push_str(&format!("  FORMAT: V2 (items), {} sections\n", items.len()));
                    for (i, item) in items.iter().enumerate() {
                        let stype = item.get("type").and_then(|t| t.as_str()).unwrap_or("?");
                        let title = item.get("title")
                            .and_then(|t| t.as_str())
                            .or_else(|| item.get("titleTextInfo").and_then(|ti| ti.get("text")).and_then(|t| t.as_str()))
                            .unwrap_or("(no title)");
                        let item_count = item.get("items").and_then(|i| i.as_array()).map(|a| a.len()).unwrap_or(0);
                        let has_view_all = item.get("viewAll").is_some() || item.get("showMore").is_some();
                        let first_type = item.get("items").and_then(|i| i.as_array())
                            .and_then(|a| a.first()).and_then(|f| f.get("type")).and_then(|t| t.as_str()).unwrap_or("?");
                        summary.push_str(&format!("    Sec {}: type={:<35} title=\"{}\" items={} first={} more={}\n",
                            i, stype, title, item_count, first_type, has_view_all));
                    }
                }
            }
            Err(e) => {
                summary.push_str(&format!("  FETCH ERROR: {}\n", e));
            }
        }
        summary.push('\n');
    }

    Ok(summary)
}

// ==================== Track Metadata (Lyrics, Credits, Radio) ====================

#[tauri::command(rename_all = "camelCase")]
fn get_track_lyrics(state: State<AppState>, track_id: u64) -> Result<TidalLyrics, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_track_lyrics(track_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_track_credits(state: State<AppState>, track_id: u64) -> Result<Vec<TidalCredit>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_track_credits(track_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_track_radio(state: State<AppState>, track_id: u64, limit: u32) -> Result<Vec<TidalTrack>, String> {
    let client = state.tidal_client.lock().map_err(|e| e.to_string())?;
    client.get_track_radio(track_id, limit)
}

// ==================== Audio Playback ====================

#[tauri::command(rename_all = "camelCase")]
fn play_tidal_track(state: State<AppState>, track_id: u64) -> Result<StreamInfo, String> {
    // Try quality tiers from highest to lowest.
    let stream_info = {
        let client = state.tidal_client.lock().map_err(|e| e.to_string())?;

        client.get_stream_url(track_id, "HI_RES_LOSSLESS")
            .or_else(|_| client.get_stream_url(track_id, "HI_RES"))
            .or_else(|_| client.get_stream_url(track_id, "LOSSLESS"))
            .or_else(|_| client.get_stream_url(track_id, "HIGH"))?
    };

    println!(
        "DEBUG: Playing track {} — quality={:?}, bitDepth={:?}, sampleRate={:?}, codec={:?}, dash={}",
        track_id, stream_info.audio_quality, stream_info.bit_depth, stream_info.sample_rate,
        stream_info.codec, stream_info.manifest.is_some()
    );

    let uri = if let Some(ref mpd) = stream_info.manifest {
        // DASH: pass MPD manifest as a data URI for GStreamer's dashdemux.
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(mpd.as_bytes());
        format!("data:application/dash+xml;base64,{}", b64)
    } else {
        // BTS: direct URL.
        stream_info.url.clone()
    };

    state.audio_player.play_url(&uri)?;

    // Save last played track
    if let Some(mut settings) = state.load_settings() {
        settings.last_track_id = Some(track_id);
        state.save_settings(&settings).ok();
    }

    Ok(stream_info)
}

#[tauri::command]
fn pause_track(state: State<AppState>) -> Result<(), String> {
    state.audio_player.pause()
}

#[tauri::command]
fn resume_track(state: State<AppState>) -> Result<(), String> {
    state.audio_player.resume()
}

#[tauri::command]
fn stop_track(state: State<AppState>) -> Result<(), String> {
    state.audio_player.stop()
}

#[tauri::command]
fn set_volume(state: State<AppState>, level: f32) -> Result<(), String> {
    state.audio_player.set_volume(level)?;

    // Save volume to settings
    if let Some(mut settings) = state.load_settings() {
        settings.volume = level;
        state.save_settings(&settings).ok();
    }

    Ok(())
}

#[tauri::command]
fn get_playback_position(state: State<AppState>) -> Result<f32, String> {
    state.audio_player.get_position()
}

#[tauri::command(rename_all = "camelCase")]
fn seek_track(state: State<AppState>, position_secs: f32) -> Result<(), String> {
    state.audio_player.seek(position_secs)
}

#[tauri::command]
fn is_track_finished(state: State<AppState>) -> Result<bool, String> {
    state.audio_player.is_finished()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_tidal_auth,
            poll_tidal_auth,
            load_saved_auth,
            refresh_tidal_auth,
            start_pkce_auth,
            complete_pkce_auth,
            logout,
            get_session_user_id,
            get_user_profile,
            get_user_playlists,
            get_playlist_tracks,
            get_favorite_tracks,
            get_favorite_track_ids,
            is_track_favorited,
            add_favorite_track,
            remove_favorite_track,
            is_album_favorited,
            add_favorite_album,
            remove_favorite_album,
            get_album_detail,
            get_album_tracks,
            get_stream_url,
            search_tidal,
            get_track_lyrics,
            get_track_credits,
            get_track_radio,
            get_home_page,
            refresh_home_page,
            get_favorite_artists,
            get_page_section,
            get_mix_items,
            get_artist_detail,
            get_artist_top_tracks,
            get_artist_albums,
            get_artist_bio,
            debug_home_page_raw,
            play_tidal_track,
            pause_track,
            resume_track,
            stop_track,
            set_volume,
            get_playback_position,
            seek_track,
            is_track_finished
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
