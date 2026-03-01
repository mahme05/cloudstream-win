// commands/repos.rs
// Repository browser — fetches CloudStream-compatible repo.json files
// and displays their plugin listings. This lets users browse the same
// repos as CloudStream. Actual playback uses our own JS plugin system.

use serde::{Deserialize, Serialize};

// ── CloudStream repo.json format ──────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoManifest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "pluginLists")]
    pub plugin_lists: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoPlugin {
    pub name: String,
    #[serde(rename = "internalName")]
    pub internal_name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    #[serde(rename = "tvTypes")]
    pub tv_types: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub version: Option<i32>,
    pub language: Option<String>,
    pub status: Option<i32>,
    pub url: Option<String>,
    #[serde(rename = "repositoryUrl")]
    pub repository_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoInfo {
    pub url: String,
    pub name: String,
    pub description: Option<String>,
    pub plugins: Vec<RepoPlugin>,
}

/// Fetch a repo manifest + all its plugin lists
/// React call: await invoke("fetch_repo", { url: "https://..." })
#[tauri::command]
pub async fn fetch_repo(url: String) -> Result<RepoInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // Fetch the repo manifest (repo.json / repo / Netflix.json etc.)
    let manifest_text = client.get(&url)
        .send().await.map_err(|e| format!("Failed to fetch repo: {}", e))?
        .text().await.map_err(|e| e.to_string())?;

    // Try to parse as CloudStream repo manifest first.
    // Some repos (e.g. Netflix.json) are a flat plugin list, not a manifest.
    let json: serde_json::Value = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("Invalid JSON from repo: {}", e))?;

    // Case 1: flat plugin array  [{"name":...}, ...]
    if json.is_array() {
        let plugins: Vec<RepoPlugin> = serde_json::from_value(json)
            .unwrap_or_default();
        return Ok(RepoInfo {
            url: url.clone(),
            name: url.split('/').last().unwrap_or("Repo").to_string(),
            description: None,
            plugins,
        });
    }

    // Case 2: manifest with pluginLists
    let manifest: RepoManifest = serde_json::from_value(json.clone())
        .map_err(|e| format!("Invalid repo format: {}", e))?;

    // Fetch all plugin lists and combine them
    let mut all_plugins: Vec<RepoPlugin> = Vec::new();
    for plugin_list_url in &manifest.plugin_lists {
        match client.get(plugin_list_url).send().await {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    if let Ok(plugins) = serde_json::from_str::<Vec<RepoPlugin>>(&text) {
                        all_plugins.extend(plugins);
                    }
                }
            }
            Err(e) => log::warn!("Failed to fetch plugin list {}: {}", plugin_list_url, e),
        }
    }

    Ok(RepoInfo {
        url: url.clone(),
        name: manifest.name,
        description: manifest.description,
        plugins: all_plugins,
    })
}

/// Fetch multiple repos at once
#[tauri::command]
pub async fn fetch_repos(urls: Vec<String>) -> Result<Vec<RepoInfo>, String> {
    let mut results = Vec::new();
    for url in urls {
        match fetch_repo(url).await {
            Ok(info) => results.push(info),
            Err(e) => log::warn!("Repo fetch failed: {}", e),
        }
    }
    Ok(results)
}
