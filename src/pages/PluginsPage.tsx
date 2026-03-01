// src/pages/PluginsPage.tsx
// Plugin manager with CloudStream-compatible repository browser.
// Users can add repo URLs (same format as CloudStream) and browse/install plugins.

import { useState, useEffect } from "react";
import { useStore } from "../store";
import { api } from "../api/tauri";
import { open } from "@tauri-apps/plugin-dialog";

// ── Types ──────────────────────────────────────────────────

interface RepoPlugin {
  name: string;
  internalName?: string;
  description?: string;
  iconUrl?: string;
  tvTypes?: string[];
  authors?: string[];
  version?: number;
  language?: string;
  status?: number;
  url?: string;
  repositoryUrl?: string;
}

interface RepoInfo {
  url: string;
  name: string;
  description?: string;
  plugins: RepoPlugin[];
}

// ── Default repos pre-loaded (same ones as CloudStream) ───

const DEFAULT_REPOS = [
  {
    label: "CloudStream Official",
    url: "https://raw.githubusercontent.com/recloudstream/extensions/builds/repo.json",
  },
  {
    label: "NetMirror (Netflix/Prime/Hotstar)",
    url: "https://raw.githubusercontent.com/Sushan64/NetMirror-Extension/refs/heads/builds/Netflix.json",
  },
  {
    label: "3rabi عربي (Arabic)",
    url: "https://raw.githubusercontent.com/Abodabodd/re-3arabi/refs/heads/main/repo",
  },
  {
    label: "MegaRepo",
    url: "https://raw.githubusercontent.com/self-similarity/MegaRepo/builds/repo.json",
  },
];

const TVTYPE_COLORS: Record<string, string> = {
  Movie: "#e50914",
  TvSeries: "#3498db",
  Anime: "#9b59b6",
  Live: "#e67e22",
  Others: "#7f8c8d",
  Drama: "#1abc9c",
  AsianDrama: "#16a085",
  Music: "#f39c12",
  Documentary: "#27ae60",
};

export default function PluginsPage() {
  const { plugins, loadPlugins } = useStore();
  const [tab, setTab] = useState<"installed" | "repos" | "manual">("installed");

  // Repo browser state
  const [savedRepos, setSavedRepos] = useState<string[]>(() => {
    try { return JSON.parse(localStorage.getItem("saved_repos") || "[]"); } catch { return []; }
  });
  const [repoData, setRepoData] = useState<RepoInfo[]>([]);
  const [loadingRepos, setLoadingRepos] = useState(false);
  const [repoError, setRepoError] = useState("");
  const [newRepoUrl, setNewRepoUrl] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState("All");
  const [selectedRepo, setSelectedRepo] = useState<string | null>(null);

  // Manual install state
  const [installing, setInstalling] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [cs3UrlInput, setCs3UrlInput] = useState("");

  const flash = (msg: string, isError = false) => {
    if (isError) { setError(msg); setSuccess(""); }
    else { setSuccess(msg); setError(""); }
    setTimeout(() => { setError(""); setSuccess(""); }, 5000);
  };

  // Load repos when tab opens
  useEffect(() => {
    if (tab === "repos" && repoData.length === 0) {
      loadAllRepos([...DEFAULT_REPOS.map(r => r.url), ...savedRepos]);
    }
  }, [tab]);

  // Map of URL -> friendly label (from DEFAULT_REPOS or saved repo labels)
  const repoLabels: Record<string, string> = Object.fromEntries(
    DEFAULT_REPOS.map(r => [r.url, r.label])
  );

  const applyFriendlyNames = (results: RepoInfo[]) =>
    results.map(r => ({
      ...r,
      name: repoLabels[r.url] || r.name,
    }));

  const loadAllRepos = async (urls: string[]) => {
    setLoadingRepos(true);
    setRepoError("");
    try {
      const results = await invoke<RepoInfo[]>("fetch_repos", { urls });
      setRepoData(applyFriendlyNames(results));
    } catch (e: any) {
      setRepoError("Failed to load some repos: " + String(e));
    } finally {
      setLoadingRepos(false);
    }
  };

  const addRepo = async () => {
    if (!newRepoUrl.trim()) return;
    const url = newRepoUrl.trim();

    // Prevent duplicate repos
    const allUrls = [...DEFAULT_REPOS.map(r => r.url), ...savedRepos];
    if (allUrls.includes(url)) {
      setRepoError("This repo is already added.");
      setTimeout(() => setRepoError(""), 3000);
      return;
    }

    setNewRepoUrl("");
    setLoadingRepos(true);
    try {
      const result = await invoke<RepoInfo>("fetch_repo", { url });
      const named = { ...result, name: repoLabels[url] || result.name };
      setRepoData(prev => [...prev, named]);
      const updated = [...savedRepos, url];
      setSavedRepos(updated);
      localStorage.setItem("saved_repos", JSON.stringify(updated));
    } catch (e: any) {
      setRepoError("Failed to add repo: " + String(e));
    } finally {
      setLoadingRepos(false);
    }
  };

  const removeRepo = (url: string) => {
    const updated = savedRepos.filter(r => r !== url);
    setSavedRepos(updated);
    localStorage.setItem("saved_repos", JSON.stringify(updated));
    setRepoData(prev => prev.filter(r => r.url !== url));
  };

  // Collect all plugins across all repos for the flat view
  const allPlugins = repoData.flatMap(repo =>
    repo.plugins.map(p => ({ ...p, _repoName: repo.name, _repoUrl: repo.url }))
  );

  const allTypes = ["All", ...Array.from(new Set(
    allPlugins.flatMap(p => p.tvTypes || [])
  )).sort()];

  const filtered = allPlugins.filter(p => {
    const matchesSearch = !searchQuery ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.description || "").toLowerCase().includes(searchQuery.toLowerCase());
    const matchesType = typeFilter === "All" ||
      (p.tvTypes || []).includes(typeFilter);
    const matchesRepo = !selectedRepo || p._repoUrl === selectedRepo;
    return matchesSearch && matchesType && matchesRepo;
  });

  // Manual install
  const installFromFile = async () => {
    const selected = await open({
      filters: [{ name: "JS Plugin", extensions: ["js"] }],
      title: "Select a CloudStream Win plugin (.js)",
    });
    if (!selected || typeof selected !== "string") return;
    setInstalling("file");
    try {
      const info = await api.plugins.installFile(selected);
      await loadPlugins();
      flash(`✓ "${info.name}" installed!`);
      setTab("installed");
    } catch (e: any) { flash(String(e), true); }
    finally { setInstalling(null); }
  };

  const installCs3FromFile = async () => {
    const selected = await open({
      filters: [{ name: "CloudStream Plugin", extensions: ["cs3"] }],
      title: "Select a CloudStream plugin (.cs3)",
    });
    if (!selected || typeof selected !== "string") return;
    setInstalling("cs3-file");
    try {
      const info = await invoke<any>("install_native_plugin", { payload: { pluginPath: selected } });
      await loadPlugins();
      flash(`✓ "${info.name}" installed!`);
      setTab("installed");
    } catch (e: any) { flash(String(e), true); }
    finally { setInstalling(null); }
  };

  const installCs3FromUrl = async (url: string) => {
    setInstalling(url);
    try {
      const info = await invoke<any>("install_native_plugin", { payload: { pluginUrl: url } });
      await loadPlugins();
      flash(`✓ "${info.name}" installed!`);
      setTab("installed");
    } catch (e: any) { flash(String(e), true); }
    finally { setInstalling(null); }
  };

  const installFromUrl = async (url: string) => {
    setInstalling(url);
    try {
      const info = await api.plugins.installUrl(url);
      await loadPlugins();
      flash(`✓ "${info.name}" installed!`);
      setTab("installed");
    } catch (e: any) { flash(String(e), true); }
    finally { setInstalling(null); }
  };

  const removePlugin = async (pluginId: string, name: string) => {
    if (!confirm(`Remove "${name}"?`)) return;
    await api.plugins.remove(pluginId);
    await loadPlugins();
    flash(`Removed "${name}"`);
  };

  return (
    <div className="page plugins-page">
      <h1 className="page-title">Plugins</h1>

      {error && <div className="alert alert-error">{error}</div>}
      {success && <div className="alert alert-success">{success}</div>}

      {/* Tabs */}
      <div className="tabs">
        <button className={`tab ${tab === "installed" ? "active" : ""}`} onClick={() => setTab("installed")}>
          Installed {plugins.length > 0 && `(${plugins.length})`}
        </button>
        <button className={`tab ${tab === "repos" ? "active" : ""}`} onClick={() => setTab("repos")}>
          🌐 Browse Repos
        </button>
        <button className={`tab ${tab === "manual" ? "active" : ""}`} onClick={() => setTab("manual")}>
          Manual Install
        </button>
      </div>

      {/* ── INSTALLED ───────────────────────────────────── */}
      {tab === "installed" && (
        <div className="tab-content">
          {plugins.length === 0 ? (
            <div className="empty-state">
              <div className="empty-icon">🔌</div>
              <h2>No plugins installed</h2>
              <p>Browse repos to find plugins, or install a .js file manually.</p>
              <button className="btn-primary" onClick={() => setTab("repos")}>Browse Repos</button>
            </div>
          ) : (
            <div className="plugin-list">
              {plugins.map((plugin) => (
                <div key={plugin.id} className="plugin-row">
                  <div className="plugin-icon">🔌</div>
                  <div className="plugin-info">
                    <div className="plugin-name">{plugin.name}</div>
                    <div className="plugin-meta">v{plugin.version} · {plugin.author}</div>
                    {plugin.description && <div className="plugin-desc">{plugin.description}</div>}
                    <div className="plugin-types">
                      {plugin.supported_types.map(t => (
                        <span key={t} className="type-tag" style={{ borderColor: TVTYPE_COLORS[t], color: TVTYPE_COLORS[t] }}>{t}</span>
                      ))}
                    </div>
                  </div>
                  <button className="btn-remove" onClick={() => removePlugin(plugin.id, plugin.name)}>✕</button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── REPO BROWSER ─────────────────────────────────── */}
      {tab === "repos" && (
        <div className="tab-content">
          {/* Add repo bar */}
          <div className="repo-add-bar">
            <input
              className="text-input"
              placeholder="Paste a repo URL (e.g. cloudstreamrepo://raw.githubusercontent.com/...)"
              value={newRepoUrl}
              onChange={e => setNewRepoUrl(
                e.target.value.replace("cloudstreamrepo://", "https://")
              )}
              onKeyDown={e => e.key === "Enter" && addRepo()}
            />
            <button className="btn-primary btn-sm" onClick={addRepo} disabled={loadingRepos}>
              Add Repo
            </button>
            <button className="btn-secondary btn-sm" onClick={() => loadAllRepos([...DEFAULT_REPOS.map(r => r.url), ...savedRepos])} disabled={loadingRepos}>
              ↻ Refresh
            </button>
          </div>

          {/* Repo filter pills */}
          <div className="plugin-pills" style={{ marginBottom: 12 }}>
            <button className={`plugin-pill ${!selectedRepo ? "active" : ""}`} onClick={() => setSelectedRepo(null)}>
              All Repos ({repoData.length})
            </button>
            {repoData.map(repo => (
              <button
                key={repo.url}
                className={`plugin-pill ${selectedRepo === repo.url ? "active" : ""}`}
                onClick={() => setSelectedRepo(selectedRepo === repo.url ? null : repo.url)}
              >
                {repo.name}
                {savedRepos.includes(repo.url) && (
                  <span
                    style={{ marginLeft: 6, opacity: 0.6, cursor: "pointer" }}
                    onClick={e => { e.stopPropagation(); removeRepo(repo.url); }}
                    title="Remove repo"
                  >✕</span>
                )}
              </button>
            ))}
          </div>

          {/* Search + type filter */}
          <div className="repo-filters">
            <input
              className="text-input"
              placeholder="Search plugins..."
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              style={{ flex: 1 }}
            />
            <select
              className="text-input"
              value={typeFilter}
              onChange={e => setTypeFilter(e.target.value)}
              style={{ width: 140 }}
            >
              {allTypes.map(t => <option key={t}>{t}</option>)}
            </select>
          </div>

          {loadingRepos && (
            <div className="loading">Loading repositories...</div>
          )}

          {repoError && <div className="alert alert-error">{repoError}</div>}

          {!loadingRepos && (
            <>
              <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12 }}>
                {filtered.length} plugins across {repoData.length} repositories
              </div>

              <div className="plugin-list">
                {filtered.map((plugin, i) => (
                  <div key={`${plugin._repoUrl}-${plugin.internalName || plugin.name}-${i}`} className="plugin-row repo-plugin-row">
                    {plugin.iconUrl ? (
                      <img
                        src={plugin.iconUrl.replace("%size%", "64")}
                        className="plugin-icon-img"
                        alt=""
                        onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
                      />
                    ) : (
                      <div className="plugin-icon">🔌</div>
                    )}
                    <div className="plugin-info">
                      <div className="plugin-name">
                        {plugin.name}
                        {plugin.language && (
                          <span className="lang-badge">{plugin.language.toUpperCase()}</span>
                        )}
                      </div>
                      <div className="plugin-meta">
                        {plugin.authors?.join(", ")} · v{plugin.version}
                        <span style={{ marginLeft: 8, color: "var(--text-muted)", fontSize: 11 }}>
                          {plugin._repoName}
                        </span>
                      </div>
                      {plugin.description && (
                        <div className="plugin-desc">{plugin.description}</div>
                      )}
                      <div className="plugin-types" style={{ marginTop: 4 }}>
                        {(plugin.tvTypes || []).map(t => (
                          <span key={t} className="type-tag"
                            style={{ borderColor: TVTYPE_COLORS[t] || "#555", color: TVTYPE_COLORS[t] || "#aaa" }}>
                            {t}
                          </span>
                        ))}
                      </div>
                    </div>
                    <div className="repo-plugin-actions">
                      {(() => {
                        const pluginId = (plugin.internalName || plugin.name)
                          .toLowerCase().replace(/[^a-z0-9]/g, "_");
                        const isInstalled = plugins.some(p => p.id === pluginId);
                        return (
                          <button
                            className={isInstalled ? "btn-secondary btn-sm" : "btn-primary btn-sm"}
                            disabled={!!installing || isInstalled}
                            onClick={() => !isInstalled && plugin.url && installCs3FromUrl(plugin.url)}
                            title={isInstalled ? "Already installed" : plugin.url ? "Install this plugin" : "No download URL"}
                          >
                            {isInstalled ? "✓ Installed" : installing === plugin.url ? "Installing..." : "Install"}
                          </button>
                        );
                      })()}
                    </div>
                  </div>
                ))}

                {filtered.length === 0 && !loadingRepos && (
                  <div className="empty-state">
                    <div className="empty-icon">🔍</div>
                    <h2>No plugins found</h2>
                    <p>Try a different search or filter.</p>
                  </div>
                )}
              </div>

              {/* Info banner */}
              <div className="repo-info-banner">
                <strong>ℹ️ About these plugins</strong>
                <p>
                  These are real CloudStream plugins (.cs3). Click <strong>Install</strong> on any plugin
                  to download and load it via the JVM bridge. The plugin runs using the original
                  Kotlin code — full compatibility with all CloudStream extensions.
                </p>
              </div>
            </>
          )}

          {/* Default repos list */}
          <div style={{ marginTop: 24 }}>
            <h3 style={{ fontSize: 14, marginBottom: 10, color: "var(--text-secondary)" }}>
              Pre-loaded repositories
            </h3>
            {DEFAULT_REPOS.map(r => (
              <div key={r.url} className="repo-row">
                <div>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>{r.label}</div>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", fontFamily: "monospace" }}>{r.url}</div>
                </div>
                <span className="installed-badge">Loaded</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── MANUAL INSTALL ───────────────────────────────── */}
      {tab === "manual" && (
        <div className="tab-content">
          <div className="install-section">
            <h3>Install JS plugin from file</h3>
            <p className="hint">Select a .js plugin file from your computer.</p>
            <button className="btn-primary" onClick={installFromFile} disabled={installing === "file"}>
              {installing === "file" ? "Installing..." : "📂 Browse for .js file"}
            </button>
          </div>

          <div className="divider" />

          <div className="install-section">
            <h3>Install JS plugin from URL</h3>
            <p className="hint">
              Paste a direct link to a .js plugin file.<br />
              Note: CloudStream <code>.cs3</code> files are for Android only and cannot be installed here.
            </p>
            <div className="url-input-row">
              <input
                className="text-input"
                type="text"
                value={urlInput}
                onChange={e => setUrlInput(e.target.value)}
                placeholder="https://raw.githubusercontent.com/.../plugin.js"
              />
              <button
                className="btn-primary"
                onClick={() => installFromUrl(urlInput)}
                disabled={!urlInput.trim() || !!installing}
              >
                {installing === urlInput ? "Installing..." : "Install"}
              </button>
            </div>
          </div>

          <div className="divider" />

          <div className="install-section">
            <h3>Install CloudStream plugin from file (.cs3)</h3>
            <p className="hint">Select a .cs3 plugin file downloaded from a CloudStream repo.</p>
            <button className="btn-primary" onClick={installCs3FromFile} disabled={installing === "cs3-file"}>
              {installing === "cs3-file" ? "Installing..." : "📂 Browse for .cs3 file"}
            </button>
          </div>

          <div className="divider" />

          <div className="install-section">
            <h3>Install CloudStream plugin from URL (.cs3)</h3>
            <p className="hint">Paste a direct link to a .cs3 plugin file from any CloudStream repo.</p>
            <div className="url-input-row">
              <input
                className="text-input"
                type="text"
                value={cs3UrlInput}
                onChange={e => setCs3UrlInput(e.target.value)}
                placeholder="https://raw.githubusercontent.com/.../GogoAnime.cs3"
              />
              <button
                className="btn-primary"
                onClick={() => installCs3FromUrl(cs3UrlInput)}
                disabled={!cs3UrlInput.trim() || !!installing}
              >
                {installing === cs3UrlInput ? "Installing..." : "Install"}
              </button>
            </div>
          </div>

          <div className="divider" />

          <div className="install-section">
            <h3>Write your own plugin</h3>
            <p className="hint">
              JS plugins are plain JavaScript — any developer can write one.
              See <code>plugins/TEMPLATE.js</code> in the repo to get started.
            </p>
            <div className="code-block">{`// @plugin-info {"id":"my-plugin","name":"My Plugin","version":"1.0.0",
//  "description":"...","author":"You","supported_types":["anime"],"is_builtin":false}

async function search(query) { ... return JSON.stringify(results); }
async function getEpisodes(showId) { ... return JSON.stringify(episodes); }
async function getStreams(mediaId) { ... return JSON.stringify(streams); }`}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Helper — invoke without importing everywhere
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  return tauriInvoke<T>(cmd, args);
}
