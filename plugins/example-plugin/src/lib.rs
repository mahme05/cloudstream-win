// plugins/example-plugin/src/lib.rs
// An example plugin written in Rust that gets compiled to .wasm
//
// This shows what a plugin author would write.
// Compile with: cargo build --target wasm32-unknown-unknown --release
//
// The host (our Tauri app) calls these functions with JSON strings
// and reads JSON strings back.

use std::collections::HashMap;

// We define the same types here that the host expects
// (in a real plugin SDK, you'd import a shared crate)
#[derive(serde::Serialize)]
struct SearchResult {
    id: String,
    title: String,
    poster_url: Option<String>,
    media_type: String,
    year: Option<i32>,
    rating: Option<f32>,
    description: Option<String>,
}

#[derive(serde::Serialize)]
struct Episode {
    id: String,
    title: String,
    season: Option<i32>,
    episode_number: i32,
    thumbnail_url: Option<String>,
    description: Option<String>,
}

#[derive(serde::Serialize)]
struct StreamSource {
    url: String,
    quality: String,
    format: String,
    subtitles: Vec<()>,
    headers: HashMap<String, String>,
}

// ─────────────────────────────────────────────
// EXPORTED FUNCTIONS
// The host calls these by name.
// They receive/return JSON via WASM memory pointers.
//
// In a production implementation you'd use a proper ABI helper
// like wit-bindgen. This is simplified for clarity.
// ─────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn search(ptr: *const u8, len: usize) -> u64 {
    let query = unsafe {
        let bytes = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(bytes).to_string()
    };

    // In a real plugin, you'd use the host's http_fetch function
    // to make web requests. Here we return fake data.
    let results = vec![
        SearchResult {
            id: "tt0111161".to_string(),
            title: format!("Search result for: {}", query),
            poster_url: Some("https://via.placeholder.com/150x225".to_string()),
            media_type: "movie".to_string(),
            year: Some(1994),
            rating: Some(9.3),
            description: Some("A great movie".to_string()),
        }
    ];

    let json = serde_json::to_string(&results).unwrap();
    write_to_wasm_memory(json)
}

#[no_mangle]
pub extern "C" fn get_streams(ptr: *const u8, len: usize) -> u64 {
    let _media_id = unsafe {
        let bytes = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(bytes).to_string()
    };

    let streams = vec![
        StreamSource {
            url: "https://example.com/stream.m3u8".to_string(),
            quality: "1080p".to_string(),
            format: "hls".to_string(),
            subtitles: vec![],
            headers: HashMap::new(),
        },
        StreamSource {
            url: "https://example.com/stream_720.m3u8".to_string(),
            quality: "720p".to_string(),
            format: "hls".to_string(),
            subtitles: vec![],
            headers: HashMap::new(),
        },
    ];

    let json = serde_json::to_string(&streams).unwrap();
    write_to_wasm_memory(json)
}

#[no_mangle]
pub extern "C" fn get_episodes(ptr: *const u8, len: usize) -> u64 {
    let _show_id = unsafe {
        let bytes = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(bytes).to_string()
    };

    let episodes: Vec<Episode> = (1..=12).map(|i| Episode {
        id: format!("s01e{:02}", i),
        title: format!("Episode {}", i),
        season: Some(1),
        episode_number: i,
        thumbnail_url: None,
        description: Some(format!("Episode {} of Season 1", i)),
    }).collect();

    let json = serde_json::to_string(&episodes).unwrap();
    write_to_wasm_memory(json)
}

// Helper: writes a string into WASM linear memory and returns (ptr << 32 | len)
// This is the convention our host uses to read strings back
fn write_to_wasm_memory(s: String) -> u64 {
    let bytes = s.into_bytes();
    let len = bytes.len();
    let ptr = bytes.as_ptr() as u64;
    std::mem::forget(bytes); // Don't drop — host will read this memory
    (ptr << 32) | (len as u64)
}
