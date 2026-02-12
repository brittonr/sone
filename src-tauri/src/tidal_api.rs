use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const TIDAL_AUTH_URL: &str = "https://auth.tidal.com/v1/oauth2";
const TIDAL_API_URL: &str = "https://api.tidal.com/v1";
// Device-code credentials (from python-tidal) – limited to LOSSLESS quality
const CLIENT_ID: &str = "REDACTED_CLIENT_ID";
const CLIENT_SECRET: &str = "REDACTED_CLIENT_SECRET";
// PKCE credentials (from python-tidal) – enables HI_RES_LOSSLESS (24-bit)
const CLIENT_ID_PKCE: &str = "REDACTED_CLIENT_ID_PKCE";
const CLIENT_SECRET_PKCE: &str = "REDACTED_CLIENT_SECRET_PKCE";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    #[serde(default)]
    pub user_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalTrack {
    pub id: u64,
    pub title: String,
    pub duration: u32,
    #[serde(default)]
    pub artist: Option<TidalArtist>,
    #[serde(default)]
    pub album: Option<TidalAlbum>,
    #[serde(default)]
    pub audio_quality: Option<String>,
    #[serde(default)]
    pub track_number: Option<u32>,
    #[serde(default)]
    pub date_added: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalAlbumDetail {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub artist: Option<TidalArtist>,
    #[serde(default)]
    pub number_of_tracks: Option<u32>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub release_date: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedTracks {
    pub items: Vec<TidalTrack>,
    pub total_number_of_items: u32,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TidalArtist {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TidalAlbum {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub cover: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalPlaylistCreator {
    pub id: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalPlaylistRaw {
    pub uuid: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub square_image: Option<String>,
    #[serde(default)]
    pub number_of_tracks: Option<u32>,
    #[serde(default)]
    pub creator: Option<TidalPlaylistCreator>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalPlaylist {
    pub uuid: String,
    pub title: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub number_of_tracks: Option<u32>,
    pub creator: Option<TidalPlaylistCreator>,
}

impl From<TidalPlaylistRaw> for TidalPlaylist {
    fn from(raw: TidalPlaylistRaw) -> Self {
        TidalPlaylist {
            uuid: raw.uuid,
            title: raw.title,
            description: raw.description,
            // Prefer squareImage, fallback to image
            image: raw.square_image.or(raw.image),
            number_of_tracks: raw.number_of_tracks,
            creator: raw.creator,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalLyrics {
    #[serde(default)]
    pub track_id: Option<u64>,
    #[serde(default)]
    pub lyrics_provider: Option<String>,
    #[serde(default)]
    pub provider_commontrack_id: Option<String>,
    #[serde(default)]
    pub provider_lyrics_id: Option<String>,
    #[serde(default)]
    pub lyrics: Option<String>,
    #[serde(default)]
    pub subtitles: Option<String>,
    #[serde(default)]
    pub is_right_to_left: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TidalContributor {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TidalCredit {
    #[serde(rename = "type")]
    pub credit_type: String,
    #[serde(default)]
    pub contributors: Vec<TidalContributor>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub url: String,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub bit_depth: Option<u32>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub audio_quality: Option<String>,
    /// Raw MPD/DASH manifest XML when the stream is DASH.
    /// `None` for BTS (single-URL) streams.
    #[serde(default)]
    pub manifest: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalSearchResults {
    pub artists: Vec<TidalArtist>,
    pub albums: Vec<TidalAlbumDetail>,
    pub tracks: Vec<TidalTrack>,
    pub playlists: Vec<TidalPlaylist>,
    #[serde(default)]
    pub top_hit_type: Option<String>,
}

// ==================== Home Page / Pages API ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TidalArtistDetail {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub picture: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HomePageSection {
    pub title: String,
    pub section_type: String,
    pub items: Value,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub api_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HomePageResponse {
    pub sections: Vec<HomePageSection>,
}

pub struct TidalClient {
    client: Client,
    pub tokens: Option<AuthTokens>,
    pub is_pkce: bool,
}

impl TidalClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            tokens: None,
            is_pkce: false,
        }
    }

    pub fn start_device_auth(&self) -> Result<DeviceCode, String> {
        let params = [
            ("client_id", CLIENT_ID),
            ("scope", "r_usr w_usr w_sub"),
        ];

        let response = self
            .client
            .post(format!("{}/device_authorization", TIDAL_AUTH_URL))
            .form(&params)
            .send()
            .map_err(|e| format!("Failed to start auth: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();
        
        if !status.is_success() {
            return Err(format!("Auth request failed ({}): {}", status, body));
        }

        serde_json::from_str::<DeviceCode>(&body)
            .map_err(|e| format!("Failed to parse response: {} - Body: {}", e, body))
    }

    pub fn poll_for_token(&mut self, device_code: &str) -> Result<AuthTokens, String> {
        let params = [
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("scope", "r_usr w_usr w_sub"),
        ];

        let response = self
            .client
            .post(format!("{}/token", TIDAL_AUTH_URL))
            .form(&params)
            .send()
            .map_err(|e| format!("Failed to poll token: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            // Check if it's still pending
            if body.contains("authorization_pending") {
                return Err("Pending".to_string());
            }
            return Err(format!("Token request failed ({}): {}", status, body));
        }

        let tokens = serde_json::from_str::<AuthTokens>(&body)
            .map_err(|e| format!("Failed to parse tokens: {} - Body: {}", e, body))?;

        self.tokens = Some(tokens.clone());
        Ok(tokens)
    }

    pub fn refresh_token(&mut self) -> Result<AuthTokens, String> {
        let current_tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let refresh_tok = current_tokens.refresh_token.clone();
        let old_user_id = current_tokens.user_id;

        let (cid, csec) = if self.is_pkce {
            (CLIENT_ID_PKCE, CLIENT_SECRET_PKCE)
        } else {
            (CLIENT_ID, CLIENT_SECRET)
        };

        let params = [
            ("client_id", cid),
            ("client_secret", csec),
            ("refresh_token", refresh_tok.as_str()),
            ("grant_type", "refresh_token"),
            ("scope", "r_usr w_usr w_sub"),
        ];

        let response = self
            .client
            .post(format!("{}/token", TIDAL_AUTH_URL))
            .form(&params)
            .send()
            .map_err(|e| format!("Failed to refresh token: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Token refresh failed ({}): {}", status, body));
        }

        // Tidal's refresh response may not include refresh_token, so use a
        // permissive struct and fall back to the existing refresh token.
        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            #[serde(default)]
            refresh_token: Option<String>,
            expires_in: u64,
            token_type: String,
            #[serde(default)]
            user_id: Option<u64>,
        }

        let parsed = serde_json::from_str::<RefreshResponse>(&body)
            .map_err(|e| format!("Failed to parse refreshed tokens: {} - Body: {}", e, body))?;

        let new_tokens = AuthTokens {
            access_token: parsed.access_token,
            refresh_token: parsed.refresh_token.unwrap_or(refresh_tok),
            expires_in: parsed.expires_in,
            token_type: parsed.token_type,
            user_id: parsed.user_id.or(old_user_id),
        };

        self.tokens = Some(new_tokens.clone());
        Ok(new_tokens)
    }

    pub fn exchange_pkce_code(
        &mut self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        client_unique_key: &str,
    ) -> Result<AuthTokens, String> {
        let response = self
            .client
            .post(format!("{}/token", TIDAL_AUTH_URL))
            .form(&[
                ("code", code),
                ("client_id", CLIENT_ID_PKCE),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri),
                ("scope", "r_usr+w_usr+w_sub"),
                ("code_verifier", code_verifier),
                ("client_unique_key", client_unique_key),
            ])
            .send()
            .map_err(|e| format!("Failed to exchange PKCE code: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("PKCE token exchange failed ({}): {}", status, body));
        }

        let tokens = serde_json::from_str::<AuthTokens>(&body)
            .map_err(|e| format!("Failed to parse PKCE tokens: {} - Body: {}", e, body))?;

        self.tokens = Some(tokens.clone());
        self.is_pkce = true;
        Ok(tokens)
    }

    pub fn get_user_profile(&self, user_id: u64) -> Result<(String, Option<String>), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to get user profile: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("User profile request failed: {}", response.status()));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UserProfile {
            #[serde(default)]
            first_name: Option<String>,
            #[serde(default)]
            last_name: Option<String>,
            #[serde(default)]
            username: Option<String>,
        }

        let data = response
            .json::<UserProfile>()
            .map_err(|e| format!("Failed to parse user profile: {}", e))?;

        let username = data.username.clone();
        let name = match (&data.first_name, &data.last_name) {
            (Some(f), Some(l)) if !f.is_empty() => format!("{} {}", f, l),
            (Some(f), _) if !f.is_empty() => f.clone(),
            _ => username.clone().unwrap_or_else(|| "Tidal User".to_string()),
        };

        Ok((name, username))
    }

    pub fn get_session_info(&self) -> Result<u64, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/sessions", TIDAL_API_URL))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .send()
            .map_err(|e| format!("Failed to get session: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Session request failed: {}", response.status()));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SessionResponse {
            user_id: u64,
        }

        let data = response
            .json::<SessionResponse>()
            .map_err(|e| format!("Failed to parse session: {}", e))?;

        Ok(data.user_id)
    }

    pub fn get_user_playlists(&self, user_id: u64) -> Result<Vec<TidalPlaylist>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/playlists", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US"), ("limit", "50")])
            .send()
            .map_err(|e| format!("Failed to fetch playlists: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct PlaylistResponse {
            items: Vec<TidalPlaylistRaw>,
        }

        let data = serde_json::from_str::<PlaylistResponse>(&body)
            .map_err(|e| format!("Failed to parse playlists: {} - Body: {}", e, body))?;
        
        // Convert raw playlists to our format
        let playlists: Vec<TidalPlaylist> = data.items.into_iter().map(|p| p.into()).collect();

        Ok(playlists)
    }

    pub fn create_playlist(&self, user_id: u64, title: &str, description: &str) -> Result<TidalPlaylist, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .post(format!("{}/users/{}/playlists", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .form(&[("title", title), ("description", description)])
            .send()
            .map_err(|e| format!("Failed to create playlist: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Create playlist API error ({}): {}", status, body));
        }

        let raw = serde_json::from_str::<TidalPlaylistRaw>(&body)
            .map_err(|e| format!("Failed to parse created playlist: {} - Body: {}", e, body))?;

        Ok(raw.into())
    }

    pub fn add_track_to_playlist(&self, playlist_id: &str, track_id: u64) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        // First, get the playlist ETag which is required for modifications
        let head_response = self
            .client
            .get(format!("{}/playlists/{}", TIDAL_API_URL, playlist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to get playlist ETag: {}", e))?;

        let etag = head_response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("*")
            .to_string();

        // Add the track
        let response = self
            .client
            .post(format!("{}/playlists/{}/items", TIDAL_API_URL, playlist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .header("If-None-Match", &etag)
            .query(&[("countryCode", "US")])
            .form(&[
                ("trackIds", &track_id.to_string()),
                ("onDupes", &"FAIL".to_string()),
            ])
            .send()
            .map_err(|e| format!("Failed to add track to playlist: {}", e))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(format!("Add track to playlist API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn remove_track_from_playlist(&self, playlist_id: &str, index: u32) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        // First, get the playlist ETag which is required for modifications
        let head_response = self
            .client
            .get(format!("{}/playlists/{}", TIDAL_API_URL, playlist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to get playlist ETag: {}", e))?;

        let etag = head_response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("*")
            .to_string();

        // Remove the track at the given index
        let response = self
            .client
            .delete(format!("{}/playlists/{}/items/{}", TIDAL_API_URL, playlist_id, index))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .header("If-None-Match", &etag)
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to remove track from playlist: {}", e))?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(format!("Remove track from playlist API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn get_favorite_playlists(&self, user_id: u64) -> Result<Vec<TidalPlaylist>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/favorites/playlists", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US"), ("limit", "50")])
            .send()
            .map_err(|e| format!("Failed to fetch favorite playlists: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Favorite playlists API error ({}): {}", status, body));
        }

        // The favorites endpoint wraps each playlist in { item: {...}, created: "..." }
        #[derive(Deserialize)]
        struct FavEntry {
            item: TidalPlaylistRaw,
        }
        #[derive(Deserialize)]
        struct FavResponse {
            items: Vec<FavEntry>,
        }

        let data = serde_json::from_str::<FavResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite playlists: {} - Body: {}", e, body))?;

        let playlists: Vec<TidalPlaylist> = data.items.into_iter().map(|e| e.item.into()).collect();
        Ok(playlists)
    }

    pub fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<TidalTrack>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        // Use /tracks endpoint (not /items) which includes dateAdded on each track
        let response = self
            .client
            .get(format!("{}/playlists/{}/tracks", TIDAL_API_URL, playlist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US"), ("limit", "100")])
            .send()
            .map_err(|e| format!("Failed to fetch tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct TracksResponse {
            items: Vec<TidalTrack>,
        }

        let data = serde_json::from_str::<TracksResponse>(&body)
            .map_err(|e| format!("Failed to parse tracks: {} - Body: {}", e, body))?;

        Ok(data.items)
    }

    pub fn get_album_detail(&self, album_id: u64) -> Result<TidalAlbumDetail, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/albums/{}", TIDAL_API_URL, album_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch album: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        serde_json::from_str::<TidalAlbumDetail>(&body)
            .map_err(|e| format!("Failed to parse album: {} - Body: {}", e, body))
    }

    pub fn get_album_tracks(&self, album_id: u64, offset: u32, limit: u32) -> Result<PaginatedTracks, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/albums/{}/tracks", TIDAL_API_URL, album_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch album tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AlbumTracksResponse {
            items: Vec<TidalTrack>,
            total_number_of_items: u32,
            #[serde(default)]
            offset: u32,
            #[serde(default)]
            limit: u32,
        }

        let data = serde_json::from_str::<AlbumTracksResponse>(&body)
            .map_err(|e| format!("Failed to parse album tracks: {} - Body: {}", e, body))?;

        Ok(PaginatedTracks {
            items: data.items,
            total_number_of_items: data.total_number_of_items,
            offset: data.offset,
            limit: data.limit,
        })
    }

    pub fn get_favorite_tracks(&self, user_id: u64, offset: u32, limit: u32) -> Result<PaginatedTracks, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/favorites/tracks", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct FavoriteTrackItem {
            item: TidalTrack,
            created: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FavoriteTracksResponse {
            items: Vec<FavoriteTrackItem>,
            total_number_of_items: u32,
            #[serde(default)]
            offset: u32,
            #[serde(default)]
            limit: u32,
        }

        let data = serde_json::from_str::<FavoriteTracksResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite tracks: {} - Body: {}", e, body))?;

        Ok(PaginatedTracks {
            items: data.items.into_iter().map(|f| {
                let mut t = f.item;
                t.date_added = Some(f.created);
                t
            }).collect(),
            total_number_of_items: data.total_number_of_items,
            offset: data.offset,
            limit: data.limit,
        })
    }

    pub fn is_track_favorited(&self, user_id: u64, track_id: u64) -> Result<bool, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .get(format!("{}/users/{}/favorites/tracks", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", "2000"),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct FavoriteTrackItem {
            #[serde(default)]
            id: Option<u64>,
            #[serde(default)]
            item: Option<TidalTrack>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FavoriteTracksResponse {
            #[serde(default)]
            items: Vec<FavoriteTrackItem>,
        }

        let data = serde_json::from_str::<FavoriteTracksResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite tracks: {} - Body: {}", e, body))?;

        Ok(data.items.iter().any(|entry| {
            entry.id == Some(track_id)
                || entry
                    .item
                    .as_ref()
                    .is_some_and(|track| track.id == track_id)
        }))
    }

    pub fn get_favorite_track_ids(&self, user_id: u64) -> Result<Vec<u64>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .get(format!("{}/users/{}/favorites/tracks", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", "2000"),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct FavItem {
            item: TidalTrack,
        }

        #[derive(Deserialize)]
        struct FavResponse {
            #[serde(default)]
            items: Vec<FavItem>,
        }

        let data = serde_json::from_str::<FavResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite track ids: {}", e))?;

        Ok(data.items.into_iter().map(|f| f.item.id).collect())
    }

    pub fn add_favorite_track(&self, user_id: u64, track_id: u64) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let track_id_str = track_id.to_string();

        let response = self
            .client
            .post(format!("{}/users/{}/favorites/tracks", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .form(&[("trackId", track_id_str.as_str())])
            .send()
            .map_err(|e| format!("Failed to favorite track: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn remove_favorite_track(&self, user_id: u64, track_id: u64) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .delete(format!(
                "{}/users/{}/favorites/tracks/{}",
                TIDAL_API_URL, user_id, track_id
            ))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to remove favorite track: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn is_album_favorited(&self, user_id: u64, album_id: u64) -> Result<bool, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .get(format!("{}/users/{}/favorites/albums", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", "2000"),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite albums: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct FavoriteAlbumItem {
            #[serde(default)]
            id: Option<u64>,
            #[serde(default)]
            item: Option<TidalAlbum>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FavoriteAlbumsResponse {
            #[serde(default)]
            items: Vec<FavoriteAlbumItem>,
        }

        let data = serde_json::from_str::<FavoriteAlbumsResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite albums: {} - Body: {}", e, body))?;

        Ok(data.items.iter().any(|entry| {
            entry.id == Some(album_id)
                || entry
                    .item
                    .as_ref()
                    .is_some_and(|album| album.id == album_id)
        }))
    }

    pub fn add_favorite_album(&self, user_id: u64, album_id: u64) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;
        let album_id_str = album_id.to_string();

        let response = self
            .client
            .post(format!("{}/users/{}/favorites/albums", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .form(&[("albumId", album_id_str.as_str())])
            .send()
            .map_err(|e| format!("Failed to favorite album: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn remove_favorite_album(&self, user_id: u64, album_id: u64) -> Result<(), String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .delete(format!(
                "{}/users/{}/favorites/albums/{}",
                TIDAL_API_URL, user_id, album_id
            ))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to remove favorite album: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        Ok(())
    }

    pub fn get_stream_url(&self, track_id: u64, quality: &str) -> Result<StreamInfo, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!(
                "{}/tracks/{}/playbackinfopostpaywall",
                TIDAL_API_URL, track_id
            ))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("audioquality", quality),
                ("playbackmode", "STREAM"),
                ("assetpresentation", "FULL"),
            ])
            .send()
            .map_err(|e| format!("Failed to get stream URL: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PlaybackInfo {
            manifest_mime_type: String,
            manifest: String,
            #[serde(default)]
            audio_quality: Option<String>,
            #[serde(default)]
            bit_depth: Option<u32>,
            #[serde(default)]
            sample_rate: Option<u32>,
        }

        let data = serde_json::from_str::<PlaybackInfo>(&body)
            .map_err(|e| format!("Failed to parse playback info: {} - Body: {}", e, body))?;

        use base64::Engine;
        let manifest_bytes = base64::engine::general_purpose::STANDARD.decode(&data.manifest)
            .map_err(|e| format!("Failed to decode manifest: {}", e))?;
        let manifest_str = String::from_utf8(manifest_bytes)
            .map_err(|e| format!("Invalid manifest encoding: {}", e))?;

        let mut codec: Option<String> = None;

        // Handle BTS format (JSON with urls array)
        let url = if data.manifest_mime_type.contains("vnd.tidal.bts") {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            #[allow(dead_code)]
            struct BtsManifest {
                urls: Vec<String>,
                codecs: Option<String>,
                mime_type: Option<String>,
                encryption_type: Option<String>,
            }

            let manifest_data = serde_json::from_str::<BtsManifest>(&manifest_str)
                .map_err(|e| format!("Failed to parse BTS manifest: {} - Manifest: {}", e, manifest_str))?;

            codec = manifest_data.codecs.map(|c| c.to_uppercase().split('.').next().unwrap_or("").to_string());

            manifest_data
                .urls
                .into_iter()
                .next()
                .ok_or("No URL in BTS manifest".to_string())?
        }
        // Handle DASH/MPD format — return raw manifest for GStreamer
        else if data.manifest_mime_type.contains("dash+xml") {
            // Extract codec from manifest
            if let Some(codecs_start) = manifest_str.find("codecs=\"") {
                let start = codecs_start + 8;
                if let Some(codecs_end) = manifest_str[start..].find("\"") {
                    let raw = &manifest_str[start..start + codecs_end];
                    codec = Some(if raw.contains("flac") { "FLAC".to_string() } else { raw.to_uppercase() });
                }
            }

            return Ok(StreamInfo {
                url: String::new(),
                codec,
                bit_depth: data.bit_depth,
                sample_rate: data.sample_rate,
                audio_quality: data.audio_quality,
                manifest: Some(manifest_str),
            });
        }
        // JSON fallback
        else {
            #[derive(Deserialize)]
            struct JsonManifest {
                urls: Option<Vec<String>>,
            }

            if let Ok(manifest_data) = serde_json::from_str::<JsonManifest>(&manifest_str) {
                if let Some(urls) = manifest_data.urls {
                    if let Some(u) = urls.into_iter().next() {
                        u
                    } else {
                        return Err("Empty URL list in manifest".to_string());
                    }
                } else {
                    return Err("No urls in JSON manifest".to_string());
                }
            } else {
                return Err(format!("Unknown manifest format '{}': {}", data.manifest_mime_type, &manifest_str[..manifest_str.len().min(300)]));
            }
        };

        Ok(StreamInfo {
            url,
            codec,
            bit_depth: data.bit_depth,
            sample_rate: data.sample_rate,
            audio_quality: data.audio_quality,
            manifest: None,
        })
    }

    pub fn get_track_lyrics(&self, track_id: u64) -> Result<TidalLyrics, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/tracks/{}/lyrics", TIDAL_API_URL, track_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch lyrics: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Lyrics not available ({})", status));
        }

        serde_json::from_str::<TidalLyrics>(&body)
            .map_err(|e| format!("Failed to parse lyrics: {} - Body: {}", e, body))
    }

    pub fn get_track_credits(&self, track_id: u64) -> Result<Vec<TidalCredit>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/tracks/{}/credits", TIDAL_API_URL, track_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch credits: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Credits not available ({})", status));
        }

        serde_json::from_str::<Vec<TidalCredit>>(&body)
            .map_err(|e| format!("Failed to parse credits: {} - Body: {}", e, body))
    }

    pub fn get_track_radio(&self, track_id: u64, limit: u32) -> Result<Vec<TidalTrack>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/tracks/{}/radio", TIDAL_API_URL, track_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch track radio: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Track radio not available ({})", status));
        }

        // Tidal v1 radio endpoint may return { items: [...] } or a flat array
        #[derive(Deserialize)]
        struct RadioResponse {
            items: Vec<TidalTrack>,
        }

        if let Ok(data) = serde_json::from_str::<RadioResponse>(&body) {
            return Ok(data.items);
        }

        // Fallback: try parsing as a flat array
        serde_json::from_str::<Vec<TidalTrack>>(&body)
            .map_err(|e| format!("Failed to parse track radio: {} - Body: {}", e, body))
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<TidalSearchResults, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/search", TIDAL_API_URL))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("query", query),
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
                ("types", "ARTISTS,ALBUMS,TRACKS,PLAYLISTS"),
            ])
            .send()
            .map_err(|e| format!("Failed to search: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Search failed ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchSection<T> {
            items: Vec<T>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TopHit {
            #[serde(rename = "type")]
            hit_type: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchResponse {
            #[serde(default)]
            artists: Option<SearchSection<TidalArtist>>,
            #[serde(default)]
            albums: Option<SearchSection<TidalAlbumDetail>>,
            #[serde(default)]
            tracks: Option<SearchSection<TidalTrack>>,
            #[serde(default)]
            playlists: Option<SearchSection<TidalPlaylistRaw>>,
            #[serde(default)]
            top_hit: Option<TopHit>,
        }

        let data = serde_json::from_str::<SearchResponse>(&body)
            .map_err(|e| format!("Failed to parse search results: {} - Body: {}", e, body))?;

        Ok(TidalSearchResults {
            artists: data.artists.map(|s| s.items).unwrap_or_default(),
            albums: data.albums.map(|s| s.items).unwrap_or_default(),
            tracks: data.tracks.map(|s| s.items).unwrap_or_default(),
            playlists: data
                .playlists
                .map(|s| s.items.into_iter().map(|p| p.into()).collect())
                .unwrap_or_default(),
            top_hit_type: data.top_hit.and_then(|h| h.hit_type),
        })
    }

    // ==================== Home Page (Pages API) ====================

    /// Fetch a single page endpoint. Handles both V1 and V2 response formats.
    fn fetch_page_endpoint(&self, endpoint: &str) -> Result<Vec<HomePageSection>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/{}", TIDAL_API_URL, endpoint))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("deviceType", "BROWSER"),
                ("locale", "en_US"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch {}: {}", endpoint, e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            eprintln!("Page endpoint {} failed ({}): {}", endpoint, status, &body[..body.len().min(200)]);
            return Ok(vec![]); // Don't fail the whole home page for one endpoint
        }

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse {} JSON: {}", endpoint, e))?;

        let result = Self::parse_page_response(&json)?;
        eprintln!("DEBUG [{}]: parsed {} sections: {:?}", endpoint,
            result.sections.len(),
            result.sections.iter().map(|s| format!("\"{}\" ({})", s.title, s.section_type)).collect::<Vec<_>>()
        );

        // If we got 0 sections, log the top-level keys for debugging
        if result.sections.is_empty() {
            if let Some(obj) = json.as_object() {
                let keys: Vec<&String> = obj.keys().collect();
                eprintln!("DEBUG [{}]: 0 sections parsed, top-level keys: {:?}", endpoint, keys);
            }
        }

        Ok(result.sections)
    }

    /// Build a dedup key from a section: uses title + first 3 item IDs.
    /// This ensures sections with the same title but different content are kept.
    fn section_dedup_key(s: &HomePageSection) -> String {
        let mut key = s.title.clone();
        if let Some(items) = s.items.as_array() {
            for item in items.iter().take(3) {
                let id = item.get("id").and_then(|i| i.as_u64()).map(|i| i.to_string())
                    .or_else(|| item.get("uuid").and_then(|u| u.as_str()).map(|s| s.to_string()))
                    .or_else(|| item.get("mixId").and_then(|m| m.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                key.push('|');
                key.push_str(&id);
            }
        }
        key
    }

    /// Helper: add sections to the collection, deduplicating smartly.
    /// Skips sections with empty titles.
    /// Uses title + item IDs for dedup key so same-title sections with different content are kept.
    fn add_unique_sections(
        all: &mut Vec<HomePageSection>,
        seen: &mut std::collections::HashSet<String>,
        new_sections: Vec<HomePageSection>,
    ) {
        for s in new_sections {
            // Skip sections with empty/blank titles
            if s.title.trim().is_empty() {
                continue;
            }
            let key = Self::section_dedup_key(&s);
            if seen.insert(key) {
                all.push(s);
            }
        }
    }

    /// Fetch the full home page by calling multiple Tidal page endpoints.
    /// The Tidal web app builds its home view from several endpoints, not just /pages/home.
    pub fn get_home_page(&self) -> Result<HomePageResponse, String> {
        let mut all_sections = Vec::new();
        let mut seen_titles = std::collections::HashSet::new();

        // Primary home endpoint - has core sections like New Albums, The Hits, etc.
        let home_sections = self.fetch_page_endpoint("pages/home")?;
        Self::add_unique_sections(&mut all_sections, &mut seen_titles, home_sections);

        // "For You" page - has personalized mixes, radio stations, recommendations
        if let Ok(sections) = self.fetch_page_endpoint("pages/for_you") {
            Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
        }

        // My Mixes - personal mixes (My Mix 1-8, My Daily Discovery, etc.)
        // Note: pages/my_collection_my_mixes often returns untitled sections that
        // duplicate pages/for_you content, so we only add titled unique ones.
        if let Ok(sections) = self.fetch_page_endpoint("pages/my_collection_my_mixes") {
            Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
        }

        // Explore page - popular playlists, trending, editorial picks
        for endpoint in &[
            "pages/explore",
        ] {
            if let Ok(sections) = self.fetch_page_endpoint(endpoint) {
                Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
            }
        }

        // Recently played / listening history
        if let Ok(sections) = self.fetch_page_endpoint("pages/my_collection_recently_played") {
            Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
        }

        // Video content endpoint - disabled for now since the app can't play video
        // if let Ok(sections) = self.fetch_page_endpoint("pages/videos") {
        //     Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
        // }

        // Rising / trending content - rename generic titles to avoid confusion
        if let Ok(mut sections) = self.fetch_page_endpoint("pages/rising") {
            let rename_map: std::collections::HashMap<&str, &str> = [
                ("Playlists", "Popular playlists on TIDAL"),
                ("New Tracks", "Rising new tracks"),
                ("New Albums", "Rising new albums"),
                ("Artists", "Rising artists"),
                // Video sections disabled since the app can't play video
                // ("Video Playlists", "Rising video playlists"),
                // ("New Videos", "Rising new videos"),
            ].iter().cloned().collect();

            // Filter out video sections since we can't play video
            sections.retain(|s| s.section_type != "VIDEO_LIST"
                && s.title != "Video Playlists"
                && s.title != "New Videos");

            for sec in &mut sections {
                if let Some(new_title) = rename_map.get(sec.title.as_str()) {
                    sec.title = new_title.to_string();
                }
            }
            Self::add_unique_sections(&mut all_sections, &mut seen_titles, sections);
        }

        // ---- Build synthetic sections from user data ----
        if let Some(user_id) = self.tokens.as_ref().and_then(|t| t.user_id) {
            // "Your listening history" from recently favorited tracks
            match self.get_listening_history(user_id) {
                Ok(tracks) if tracks.as_array().map(|a| !a.is_empty()).unwrap_or(false) => {
                    all_sections.push(HomePageSection {
                        title: "Your listening history".to_string(),
                        section_type: "TRACK_LIST".to_string(),
                        items: tracks,
                        has_more: false,
                        api_path: None,
                    });
                }
                Ok(_) => {}
                Err(e) => eprintln!("DEBUG: Failed to get listening history: {}", e),
            }

            // "Albums you might like" from user's favorite albums
            match self.get_favorite_albums_raw(user_id, 20) {
                Ok(albums) if albums.as_array().map(|a| !a.is_empty()).unwrap_or(false) => {
                    all_sections.push(HomePageSection {
                        title: "Albums you'll enjoy".to_string(),
                        section_type: "ALBUM_LIST".to_string(),
                        items: albums,
                        has_more: false,
                        api_path: None,
                    });
                }
                Ok(_) => {}
                Err(e) => eprintln!("DEBUG: Failed to get favorite albums: {}", e),
            }

            // "Playlists you'll love" from user's playlists
            match self.get_user_playlists_raw(user_id, 20) {
                Ok(playlists) if playlists.as_array().map(|a| !a.is_empty()).unwrap_or(false) => {
                    all_sections.push(HomePageSection {
                        title: "Playlists you'll love".to_string(),
                        section_type: "PLAYLIST_LIST".to_string(),
                        items: playlists,
                        has_more: false,
                        api_path: None,
                    });
                }
                Ok(_) => {}
                Err(e) => eprintln!("DEBUG: Failed to get user playlists for section: {}", e),
            }
        }

        // Reorder sections for a better home page flow:
        // 1. Core editorial (from /home) stays at top
        // 2. Personalized content next
        // 3. Synthetic/user sections
        // 4. Video and rising content last
        let priority_order = |title: &str| -> u8 {
            match title {
                // Core editorial sections first
                "The Hits" | "New Tracks" | "New Albums" | "From our editors" | "Spotlighted Uploads" => 0,
                // Personalized content
                t if t.contains("Custom mixes") || t.contains("Radio stations") || t.contains("releases for you") => 1,
                // User data sections
                "Your favorite artists" | "Your listening history" | "Albums you'll enjoy" | "Playlists you'll love" => 2,
                // Video content
                t if t.contains("Video") || t.contains("video") || t.contains("Classics") => 3,
                // Rising/trending
                t if t.contains("Rising") || t.contains("Popular") => 4,
                _ => 2, // default: mix in with personalized
            }
        };

        all_sections.sort_by_key(|s| priority_order(&s.title));

        eprintln!("DEBUG [home_page]: total {} unique sections", all_sections.len());
        Ok(HomePageResponse { sections: all_sections })
    }

    /// Parse a pages API response, supporting V1, V2, and tab/category formats.
    fn parse_page_response(json: &Value) -> Result<HomePageResponse, String> {
        let mut sections = Vec::new();

        // ---- V1 format: { rows: [ { modules: [ { type, title, pagedList, ... } ] } ] }
        if let Some(rows) = json.get("rows").and_then(|r| r.as_array()) {
            for row in rows {
                if let Some(modules) = row.get("modules").and_then(|m| m.as_array()) {
                    for module in modules {
                        if let Some(sec) = Self::parse_v1_module(module) {
                            sections.push(sec);
                        }
                    }
                }
            }
        }

        // ---- V2 format: { items: [ { type, title, items: [...], viewAll, ... } ] }
        if sections.is_empty() {
            if let Some(top_items) = json.get("items").and_then(|i| i.as_array()) {
                // Check if items look like V2 sections (objects with type/title/items)
                // vs just being raw content items
                let looks_like_sections = top_items.first()
                    .map(|f| f.get("items").is_some() || f.get("type").and_then(|t| t.as_str())
                        .map(|t| t.contains("LIST") || t == "SHORTCUT_LIST" || t == "PAGE_LINKS_CLOUD")
                        .unwrap_or(false))
                    .unwrap_or(false);

                if looks_like_sections {
                    for item in top_items {
                        if let Some(sec) = Self::parse_v2_section(item) {
                            sections.push(sec);
                        }
                    }
                }
            }
        }

        // ---- Tab format: { tabs: [ { title, items: [...] } ] }
        // The explore page often uses a tabs-based structure
        if sections.is_empty() {
            if let Some(tabs) = json.get("tabs").and_then(|t| t.as_array()) {
                for tab in tabs {
                    // Each tab may contain rows (V1) or items (V2) inside it
                    if let Some(rows) = tab.get("rows").and_then(|r| r.as_array()) {
                        for row in rows {
                            if let Some(modules) = row.get("modules").and_then(|m| m.as_array()) {
                                for module in modules {
                                    if let Some(sec) = Self::parse_v1_module(module) {
                                        sections.push(sec);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(items) = tab.get("items").and_then(|i| i.as_array()) {
                        for item in items {
                            if let Some(sec) = Self::parse_v2_section(item) {
                                sections.push(sec);
                            }
                        }
                    }
                }
            }
        }

        // ---- Categories format: { categories: [...] } or { sections: [...] }
        if sections.is_empty() {
            let containers = [
                json.get("categories").and_then(|c| c.as_array()),
                json.get("sections").and_then(|s| s.as_array()),
            ];
            for container in containers.into_iter().flatten() {
                for item in container {
                    // Try V1 module parsing first, then V2
                    if let Some(sec) = Self::parse_v1_module(item) {
                        sections.push(sec);
                    } else if let Some(sec) = Self::parse_v2_section(item) {
                        sections.push(sec);
                    }
                }
            }
        }

        // ---- Fallback: if the response itself looks like a single section
        //      e.g. { title, items: [...] } from a "view all" endpoint
        if sections.is_empty() {
            if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
                let page_title = json.get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Results")
                    .to_string();
                sections.push(HomePageSection {
                    title: page_title,
                    section_type: "MIXED_LIST".to_string(),
                    items: Value::Array(items.clone()),
                    has_more: false,
                    api_path: None,
                });
            }
        }

        Ok(HomePageResponse { sections })
    }

    /// Parse a V1 module (from rows/modules format).
    fn parse_v1_module(module: &Value) -> Option<HomePageSection> {
        let section_type = module.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        // Only skip truly non-content promotional types
        if section_type == "FEATURED_PROMOTIONS"
            || section_type == "MULTIPLE_TOP_PROMOTIONS"
            || section_type == "TEXT_BLOCK"
            || section_type == "SOCIAL"
            || section_type == "ARTICLE_LIST"
        {
            return None;
        }

        // Get title - check multiple possible fields
        let title = module.get("title")
            .and_then(|t| t.as_str())
            .or_else(|| module.get("header").and_then(|h| h.as_str()))
            .unwrap_or("")
            .to_string();

        // PAGE_LINKS are navigation sections, not content - skip
        if section_type == "PAGE_LINKS_CLOUD" || section_type == "PAGE_LINKS" {
            return None;
        }

        // Allow sections even with empty titles if they have items
        // (some sections have descriptions but no title)

        // Extract items from pagedList, highlights, listItems, or other containers
        let items = if let Some(paged_list) = module.get("pagedList") {
            paged_list.get("items").cloned().unwrap_or(Value::Array(vec![]))
        } else if let Some(highlights) = module.get("highlights") {
            if let Some(arr) = highlights.as_array() {
                let unwrapped: Vec<Value> = arr.iter()
                    .filter_map(|h| h.get("item").cloned())
                    .collect();
                Value::Array(unwrapped)
            } else {
                Value::Array(vec![])
            }
        } else if let Some(list_items) = module.get("listItems").and_then(|l| l.as_array()) {
            // Some modules use "listItems" instead of "pagedList"
            Value::Array(list_items.clone())
        } else if module.get("mix").is_some() {
            // MIX_HEADER type - single mix as an item
            Value::Array(vec![module.get("mix").cloned().unwrap_or(Value::Null)])
        } else {
            // Last resort: look for any array field that looks like items
            let mut found = Value::Array(vec![]);
            if let Some(obj) = module.as_object() {
                for (key, val) in obj {
                    if key == "type" || key == "title" || key == "header" || key == "showMore"
                        || key == "viewAll" || key == "description" || key == "id" || key == "selfLink" {
                        continue;
                    }
                    if let Some(arr) = val.as_array() {
                        if !arr.is_empty() && arr[0].is_object() {
                            found = val.clone();
                            break;
                        }
                    }
                }
            }
            found
        };

        // Skip truly empty sections
        if items.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            return None;
        }

        // Extract "showMore" or "viewAll" api path - check multiple locations
        let api_path = module
            .get("showMore")
            .and_then(|sm| sm.get("apiPath"))
            .and_then(|p| p.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                module
                    .get("pagedList")
                    .and_then(|pl| pl.get("dataApiPath"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                module
                    .get("viewAll")
                    .and_then(|va| {
                        if let Some(s) = va.as_str() { Some(s.to_string()) }
                        else { va.get("apiPath").and_then(|p| p.as_str()).map(|s| s.to_string()) }
                    })
            });

        let has_more = api_path.is_some();

        Some(HomePageSection {
            title,
            section_type,
            items,
            has_more,
            api_path,
        })
    }

    /// Parse a V2 section (from the flat items format).
    /// V2 sections look like:
    /// { "type": "HORIZONTAL_LIST", "moduleId": "...", "title": "...",
    ///   "items": [ { "type": "ALBUM", "data": { ... } }, ... ],
    ///   "viewAll": "pages/..." }
    fn parse_v2_section(section: &Value) -> Option<HomePageSection> {
        let section_type = section.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        // Get title from title field or titleTextInfo
        let title = section.get("title")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                section.get("titleTextInfo")
                    .and_then(|ti| ti.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        if title.is_empty() {
            return None;
        }

        // V2 items can be in "items" array, where each has { type, data }
        let raw_items = section.get("items").and_then(|i| i.as_array());

        let items = if let Some(raw) = raw_items {
            // Unwrap the "data" field from each item if present,
            // but keep the item type info by merging it
            let unwrapped: Vec<Value> = raw.iter()
                .filter_map(|item| {
                    if let Some(data) = item.get("data") {
                        // Merge item-level type into data for identification
                        let mut merged = data.clone();
                        if let Some(obj) = merged.as_object_mut() {
                            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                                obj.entry("_itemType".to_string())
                                    .or_insert(Value::String(item_type.to_string()));
                            }
                        }
                        Some(merged)
                    } else {
                        // No "data" wrapper — item is already flat
                        Some(item.clone())
                    }
                })
                .collect();
            Value::Array(unwrapped)
        } else {
            Value::Array(vec![])
        };

        // V2 viewAll is either a string or an object
        let api_path = section.get("viewAll")
            .and_then(|va| {
                if let Some(s) = va.as_str() {
                    Some(s.to_string())
                } else {
                    va.get("apiPath").and_then(|p| p.as_str()).map(|s| s.to_string())
                }
            })
            .or_else(|| {
                section.get("showMore")
                    .and_then(|sm| sm.get("apiPath"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            });

        let has_more = api_path.is_some();

        // Map V2 section types to something our frontend understands
        let mapped_type = match section_type.as_str() {
            "SHORTCUT_LIST" => "SHORTCUT_LIST",
            "HORIZONTAL_LIST" | "HORIZONTAL_LIST_WITH_CONTEXT" => {
                // Try to detect the content type from items
                if let Some(arr) = items.as_array() {
                    if let Some(first) = arr.first() {
                        let item_type = first.get("_itemType")
                            .or_else(|| first.get("type"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        match item_type {
                            "MIX" => "MIX_LIST",
                            "ALBUM" => "ALBUM_LIST",
                            "PLAYLIST" => "PLAYLIST_LIST",
                            "ARTIST" => "ARTIST_LIST",
                            "TRACK" => "TRACK_LIST",
                            _ => {
                                // Detect by data shape
                                if first.get("mixType").is_some() || first.get("mixImages").is_some() {
                                    "MIX_LIST"
                                } else if first.get("uuid").is_some() {
                                    "PLAYLIST_LIST"
                                } else if first.get("cover").is_some() || first.get("numberOfTracks").is_some() {
                                    "ALBUM_LIST"
                                } else if first.get("picture").is_some() && first.get("cover").is_none() {
                                    "ARTIST_LIST"
                                } else {
                                    "MIXED_TYPES_LIST"
                                }
                            }
                        }
                    } else {
                        "MIXED_TYPES_LIST"
                    }
                } else {
                    "MIXED_TYPES_LIST"
                }
            }
            "TRACK_LIST" => "TRACK_LIST",
            other => other,
        };

        Some(HomePageSection {
            title,
            section_type: mapped_type.to_string(),
            items,
            has_more,
            api_path,
        })
    }

    pub fn get_favorite_artists(&self, user_id: u64, limit: u32) -> Result<Vec<TidalArtistDetail>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/favorites/artists", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite artists: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        struct FavoriteArtistItem {
            item: TidalArtistDetail,
        }

        #[derive(Deserialize)]
        struct FavoriteArtistsResponse {
            items: Vec<FavoriteArtistItem>,
        }

        let data = serde_json::from_str::<FavoriteArtistsResponse>(&body)
            .map_err(|e| format!("Failed to parse favorite artists: {} - Body: {}", e, &body[..body.len().min(500)]))?;

        Ok(data.items.into_iter().map(|f| f.item).collect())
    }

    /// Fetch user's favorite albums as raw JSON for home page sections.
    fn get_favorite_albums_raw(&self, user_id: u64, limit: u32) -> Result<Value, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/favorites/albums", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch favorite albums: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Favorite albums API error ({}): {}", status, body));
        }

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse favorite albums: {}", e))?;

        // Response format: { items: [ { item: { id, title, cover, artist, ... }, created: "..." } ] }
        if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
            let albums: Vec<Value> = items.iter()
                .filter_map(|entry| entry.get("item").cloned())
                .collect();
            eprintln!("DEBUG [favorite_albums]: got {} albums", albums.len());
            Ok(Value::Array(albums))
        } else {
            Ok(Value::Array(vec![]))
        }
    }

    /// Fetch user's playlists as raw JSON for home page sections.
    fn get_user_playlists_raw(&self, user_id: u64, limit: u32) -> Result<Value, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/users/{}/playlists", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch user playlists: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("User playlists API error ({}): {}", status, body));
        }

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse user playlists: {}", e))?;

        // Response format: { items: [ { uuid, title, image, ... } ] }
        if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
            eprintln!("DEBUG [user_playlists]: got {} playlists", items.len());
            Ok(Value::Array(items.clone()))
        } else {
            Ok(Value::Array(vec![]))
        }
    }

    /// Fetch user's recently played tracks for the "listening history" section.
    fn get_listening_history(&self, user_id: u64) -> Result<Value, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        // Try the user's listening history via favorites/tracks (recent order)
        let response = self
            .client
            .get(format!("{}/users/{}/favorites/tracks", TIDAL_API_URL, user_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", "12"),
                ("offset", "0"),
                ("order", "DATE"),
                ("orderDirection", "DESC"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch listening history: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Listening history API error ({}): {}", status, body));
        }

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse listening history: {}", e))?;

        // Response format: { items: [ { item: { id, title, artist, album, ... }, created: "..." } ] }
        if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
            let tracks: Vec<Value> = items.iter()
                .filter_map(|entry| entry.get("item").cloned())
                .collect();
            eprintln!("DEBUG [listening_history]: got {} tracks", tracks.len());
            Ok(Value::Array(tracks))
        } else {
            Ok(Value::Array(vec![]))
        }
    }

    // ==================== Artist Detail ====================

    /// Fetch full artist detail (name, picture, etc.)
    pub fn get_artist_detail(&self, artist_id: u64) -> Result<TidalArtistDetail, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/artists/{}", TIDAL_API_URL, artist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch artist detail: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Artist detail API error ({}): {}", status, body));
        }

        serde_json::from_str::<TidalArtistDetail>(&body)
            .map_err(|e| format!("Failed to parse artist detail: {} - Body: {}", e, &body[..body.len().min(500)]))
    }

    // ==================== Mix / Radio Items ====================

    /// Fetch the tracks in a mix (custom mixes, radio stations, etc.)
    /// Tidal mixes use a string mixId like "00e4f8f7a5bd..."
    pub fn get_mix_items(&self, mix_id: &str) -> Result<Vec<TidalTrack>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/mixes/{}/items", TIDAL_API_URL, mix_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch mix items: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Mix items API error ({}): {}", status, body));
        }

        // Response: { items: [ { item: { id, title, ... }, type: "track" } ] }
        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse mix items: {}", e))?;

        if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
            let tracks: Vec<TidalTrack> = items.iter()
                .filter_map(|entry| {
                    entry.get("item")
                        .and_then(|item| serde_json::from_value::<TidalTrack>(item.clone()).ok())
                })
                .collect();
            Ok(tracks)
        } else {
            Ok(vec![])
        }
    }

    // ==================== Artist Page ====================

    /// Fetch an artist's top tracks
    pub fn get_artist_top_tracks(&self, artist_id: u64, limit: u32) -> Result<Vec<TidalTrack>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/artists/{}/toptracks", TIDAL_API_URL, artist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch artist top tracks: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Artist top tracks API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ArtistTracksResponse {
            items: Vec<TidalTrack>,
        }

        let data = serde_json::from_str::<ArtistTracksResponse>(&body)
            .map_err(|e| format!("Failed to parse artist top tracks: {} - Body: {}", e, &body[..body.len().min(500)]))?;

        Ok(data.items)
    }

    /// Fetch an artist's albums
    pub fn get_artist_albums(&self, artist_id: u64, limit: u32) -> Result<Vec<TidalAlbumDetail>, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/artists/{}/albums", TIDAL_API_URL, artist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[
                ("countryCode", "US"),
                ("limit", &limit.to_string()),
                ("offset", "0"),
            ])
            .send()
            .map_err(|e| format!("Failed to fetch artist albums: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Artist albums API error ({}): {}", status, body));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ArtistAlbumsResponse {
            items: Vec<TidalAlbumDetail>,
        }

        let data = serde_json::from_str::<ArtistAlbumsResponse>(&body)
            .map_err(|e| format!("Failed to parse artist albums: {} - Body: {}", e, &body[..body.len().min(500)]))?;

        Ok(data.items)
    }

    /// Fetch artist bio text
    pub fn get_artist_bio(&self, artist_id: u64) -> Result<String, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let response = self
            .client
            .get(format!("{}/artists/{}/bio", TIDAL_API_URL, artist_id))
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US")])
            .send()
            .map_err(|e| format!("Failed to fetch artist bio: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Ok(String::new()); // Bio not always available
        }

        let json: Value = serde_json::from_str(&body).unwrap_or_default();
        Ok(json.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string())
    }

    pub fn get_page(&self, api_path: &str) -> Result<HomePageResponse, String> {
        let tokens = self.tokens.as_ref().ok_or("Not authenticated")?;

        let url = if api_path.starts_with("http") {
            api_path.to_string()
        } else {
            format!("{}/{}", TIDAL_API_URL, api_path)
        };

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .query(&[("countryCode", "US"), ("deviceType", "BROWSER")])
            .send()
            .map_err(|e| format!("Failed to fetch page: {}", e))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Page API error ({}): {}", status, body));
        }

        let json: Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse page JSON: {}", e))?;

        Self::parse_page_response(&json)
    }
}
