use tauri::State;

use crate::AppState;
use crate::SoneError;
use crate::cache::{CacheResult, CacheTier};

#[tauri::command]
pub async fn get_image_bytes(state: State<'_, AppState>, url: String) -> Result<Vec<u8>, SoneError> {
    log::debug!("[get_image_bytes]: url={}", url);

    match state.disk_cache.get(&url, CacheTier::Image).await {
        CacheResult::Fresh(bytes) | CacheResult::Stale(bytes) => {
            log::debug!("[get_image_bytes]: cache hit ({} bytes)", bytes.len());
            Ok(bytes)
        }
        CacheResult::Miss => {
            let res = reqwest::get(&url).await?;
            let bytes = res.bytes().await?.to_vec();

            state.disk_cache
                .put(&url, &bytes, CacheTier::Image, &["image"])
                .await
                .ok();
            log::debug!("[get_image_bytes]: fetched and cached {} bytes", bytes.len());

            Ok(bytes)
        }
    }
}

#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AppState>) -> Result<crate::cache::CacheStats, SoneError> {
    Ok(state.disk_cache.stats().await)
}

#[tauri::command]
pub async fn clear_disk_cache(state: State<'_, AppState>) -> Result<(), SoneError> {
    log::info!("[clear_disk_cache]: user-initiated cache clear");
    state.disk_cache.clear().await;
    Ok(())
}
