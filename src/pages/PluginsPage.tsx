// src/pages/PluginsPage.tsx
import { useState } from "react";
import { useStore } from "../store";
import { api } from "../api/tauri";
import { open } from "@tauri-apps/plugin-dialog";

export default function PluginsPage() {
  const { plugins, loadPlugins } = useStore();
  const [installing, setInstalling] = useState(false);
  const [activeTab, setActiveTab] = useState<"file" | "url">("file");
  const [urlInput, setUrlInput] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleBrowseAndInstall = async () => {
    setError(null);
    const selected = await open({
      filters: [{ name: "JavaScript Plugin", extensions: ["js"] }],
      multiple: false,
    });
    if (!selected || typeof selected !== "string") return;

    setInstalling(true);
    try {
      await api.plugins.install(selected);
      await loadPlugins();
    } catch (err) {
      setError(`Failed to install: ${err}`);
    } finally {
      setInstalling(false);
    }
  };

  const handleInstallFromUrl = async () => {
    if (!urlInput.trim()) return;
    setError(null);
    setInstalling(true);
    try {
      const res = await fetch(urlInput.trim());
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const source = await res.text();
      await api.plugins.installFromSource(source);
      await loadPlugins();
      setUrlInput("");
    } catch (err) {
      setError(`Failed to install from URL: ${err}`);
    } finally {
      setInstalling(false);
    }
  };

  const handleRemove = async (pluginId: string) => {
    if (!confirm(`Remove plugin "${pluginId}"?`)) return;
    try {
      await api.plugins.remove(pluginId);
      await loadPlugins();
    } catch (err) {
      setError(`Failed to remove plugin: ${err}`);
    }
  };

  return (
    <div className="page plugins-page">
      <div className="page-header">
        <h1 className="page-title">Plugins</h1>
      </div>

      {/* Install Section */}
      <div className="install-form">
        <h3>Install a Plugin</h3>
        <p className="hint">
          Plugins are .js files that add content sources. Without plugins the app has no content.
        </p>

        {/* Tab switcher */}
        <div className="tab-row">
          <button
            className={`tab-btn ${activeTab === "file" ? "active" : ""}`}
            onClick={() => setActiveTab("file")}
          >
            From File
          </button>
          <button
            className={`tab-btn ${activeTab === "url" ? "active" : ""}`}
            onClick={() => setActiveTab("url")}
          >
            From URL
          </button>
        </div>

        {activeTab === "file" && (
          <div className="tab-content">
            <p className="hint">Select a .js plugin file from your computer.</p>
            <button
              className="btn-primary"
              onClick={handleBrowseAndInstall}
              disabled={installing}
            >
              {installing ? "Installing..." : "Browse & Install .js File"}
            </button>
          </div>
        )}

        {activeTab === "url" && (
          <div className="tab-content">
            <p className="hint">Paste a raw URL to a .js plugin file (e.g. a raw GitHub link).</p>
            <div className="file-input-row">
              <input
                className="text-input"
                type="text"
                placeholder="https://raw.githubusercontent.com/.../plugin.js"
                value={urlInput}
                onChange={(e) => setUrlInput(e.target.value)}
                disabled={installing}
              />
              <button
                className="btn-primary"
                onClick={handleInstallFromUrl}
                disabled={installing || !urlInput.trim()}
              >
                {installing ? "Installing..." : "Install"}
              </button>
            </div>
          </div>
        )}

        {error && <p className="error-text">{error}</p>}
      </div>

      {/* Installed plugins */}
      {plugins.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">🔌</div>
          <h2>No plugins installed</h2>
          <p>Install a plugin above to start streaming content.</p>
        </div>
      ) : (
        <div className="plugin-list">
          <h3 style={{ marginBottom: 12 }}>Installed ({plugins.length})</h3>
          {plugins.map((plugin) => (
            <div key={plugin.id} className="plugin-row">
              <div className="plugin-icon">🔌</div>
              <div className="plugin-info">
                <div className="plugin-name">{plugin.name}</div>
                <div className="plugin-meta">
                  v{plugin.version} · by {plugin.author || "Unknown"}
                </div>
                {plugin.description && (
                  <div className="plugin-desc">{plugin.description}</div>
                )}
                <div className="plugin-types">
                  {plugin.supported_types.map((t) => (
                    <span key={t} className="type-tag">{t}</span>
                  ))}
                </div>
              </div>
              <button
                className="btn-cancel"
                onClick={() => handleRemove(plugin.id)}
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
