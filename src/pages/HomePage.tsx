// src/pages/HomePage.tsx
// Home page: shows continue watching, bookmarks, and plugin selection.

import { useStore } from "../store";
import { useNavigate } from "react-router-dom";
import MediaCard from "../components/cards/MediaCard";

export default function HomePage() {
  const { plugins, activePluginId, setActivePlugin, bookmarks, history } = useStore();
  const navigate = useNavigate();

  const continueWatching = history
    .filter((h) => h.duration_seconds > 0 && h.progress_seconds / h.duration_seconds < 0.95)
    .slice(0, 10);

  return (
    <div className="page home-page">
      <h1 className="page-title">CloudStream</h1>

      {/* Plugin selector */}
      {plugins.length > 0 && (
        <section className="section">
          <h2 className="section-title">Plugins</h2>
          <div className="plugin-pills">
            {plugins.map((plugin) => (
              <button
                key={plugin.id}
                className={`plugin-pill ${activePluginId === plugin.id ? "active" : ""}`}
                onClick={() => {
                  setActivePlugin(plugin.id);
                  navigate("/search");
                }}
              >
                {plugin.name}
              </button>
            ))}
          </div>
        </section>
      )}

      {/* No plugins installed yet */}
      {plugins.length === 0 && (
        <div className="empty-state">
          <div className="empty-icon">🔌</div>
          <h2>No plugins installed</h2>
          <p>Go to the Plugins page to install your first plugin.</p>
          <button className="btn-primary" onClick={() => navigate("/plugins")}>
            Manage Plugins
          </button>
        </div>
      )}

      {/* Continue Watching */}
      {continueWatching.length > 0 && (
        <section className="section">
          <h2 className="section-title">Continue Watching</h2>
          <div className="card-row">
            {continueWatching.map((item) => (
              <div
                key={item.id}
                className="history-card"
                onClick={() => navigate(`/media/${item.plugin_id}/${item.media_id}`)}
              >
                <div className="history-card-title">{item.title}</div>
                {item.episode_title && (
                  <div className="history-card-episode">{item.episode_title}</div>
                )}
                {/* Progress bar */}
                <div className="progress-bar">
                  <div
                    className="progress-fill"
                    style={{
                      width: `${(item.progress_seconds / item.duration_seconds) * 100}%`,
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Bookmarks */}
      {bookmarks.length > 0 && (
        <section className="section">
          <h2 className="section-title">My Library</h2>
          <div className="card-grid">
            {bookmarks.slice(0, 12).map((bookmark) => (
              <MediaCard
                key={bookmark.id}
                id={bookmark.media_id}
                pluginId={bookmark.plugin_id}
                title={bookmark.title}
                posterUrl={bookmark.poster_url}
                mediaType={bookmark.media_type}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
