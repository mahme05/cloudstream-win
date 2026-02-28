// src/pages/PluginsPage.tsx
// Page for managing installed plugins.
// Users can install new .wasm plugin files here.

import { useState } from "react";
import { useStore } from "../store";
import { api } from "../api/tauri";
import { open } from "@tauri-apps/plugin-dialog";
import type { PluginInfo } from "../types";

export default function PluginsPage() {
  const { plugins, loadPlugins } = useStore();
  const [installing, setInstalling] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [newPlugin, setNewPlugin] = useState<Partial<PluginInfo>>({
    id: "",
    name: "",
    version: "1.0.0",
    description: "",
    author: "",
    supported_types: ["movies", "shows"],
  });
  const [wasmPath, setWasmPath] = useState("");

  const handleBrowse = async () => {
    // Opens a file picker — returns path to selected .wasm file
    const selected = await open({
      filters: [{ name: "WASM Plugin", extensions: ["wasm"] }],
    });
    if (typeof selected === "string") {
      setWasmPath(selected);
      // Auto-fill plugin ID from filename
      const filename = selected.split(/[\\/]/).pop()?.replace(".wasm", "") || "";
      setNewPlugin((p) => ({ ...p, id: filename, name: filename }));
    }
  };

  const handleInstall = async () => {
    if (!wasmPath || !newPlugin.id || !newPlugin.name) return;
    setInstalling(true);
    try {
      await api.plugins.install(wasmPath, newPlugin as PluginInfo);
      await loadPlugins(); // Refresh list
      setShowForm(false);
      setWasmPath("");
      setNewPlugin({ id: "", name: "", version: "1.0.0", description: "", author: "", supported_types: ["movies", "shows"] });
    } catch (err) {
      console.error("Install failed:", err);
      alert(`Failed to install plugin: ${err}`);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="page plugins-page">
      <div className="page-header">
        <h1 className="page-title">Plugins</h1>
        <button className="btn-primary" onClick={() => setShowForm(!showForm)}>
          + Install Plugin
        </button>
      </div>

      {/* Install form */}
      {showForm && (
        <div className="install-form">
          <h3>Install a Plugin</h3>
          <p className="hint">
            Plugins are .wasm files. You can write your own or download community plugins.
          </p>
          
          <div className="form-row">
            <label>Plugin File (.wasm)</label>
            <div className="file-input-row">
              <input type="text" value={wasmPath} readOnly placeholder="No file selected" className="text-input" />
              <button className="btn-secondary" onClick={handleBrowse}>Browse...</button>
            </div>
          </div>

          <div className="form-row">
            <label>Plugin ID</label>
            <input
              type="text"
              value={newPlugin.id}
              onChange={(e) => setNewPlugin((p) => ({ ...p, id: e.target.value }))}
              placeholder="my-plugin"
              className="text-input"
            />
          </div>

          <div className="form-row">
            <label>Display Name</label>
            <input
              type="text"
              value={newPlugin.name}
              onChange={(e) => setNewPlugin((p) => ({ ...p, name: e.target.value }))}
              placeholder="My Plugin"
              className="text-input"
            />
          </div>

          <div className="form-row">
            <label>Author</label>
            <input
              type="text"
              value={newPlugin.author}
              onChange={(e) => setNewPlugin((p) => ({ ...p, author: e.target.value }))}
              placeholder="Your Name"
              className="text-input"
            />
          </div>

          <div className="form-actions">
            <button className="btn-secondary" onClick={() => setShowForm(false)}>Cancel</button>
            <button
              className="btn-primary"
              onClick={handleInstall}
              disabled={!wasmPath || !newPlugin.id || installing}
            >
              {installing ? "Installing..." : "Install"}
            </button>
          </div>
        </div>
      )}

      {/* Installed plugins */}
      {plugins.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">🔌</div>
          <h2>No plugins installed</h2>
          <p>Install a plugin to start streaming content.</p>
        </div>
      ) : (
        <div className="plugin-list">
          {plugins.map((plugin) => (
            <div key={plugin.id} className="plugin-row">
              <div className="plugin-icon">🔌</div>
              <div className="plugin-info">
                <div className="plugin-name">{plugin.name}</div>
                <div className="plugin-meta">
                  v{plugin.version} by {plugin.author}
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
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
