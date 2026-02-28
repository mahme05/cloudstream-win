// src/pages/SearchPage.tsx
// Search page: pick a plugin, enter a query, see results.

import { useState, useEffect } from "react";
import { useStore } from "../store";
import MediaCard from "../components/cards/MediaCard";

export default function SearchPage() {
  const { plugins, activePluginId, setActivePlugin, search, searchResults, searchLoading, searchQuery } = useStore();
  const [inputValue, setInputValue] = useState(searchQuery);

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (!activePluginId || !inputValue.trim()) return;
    search(activePluginId, inputValue.trim());
  };

  return (
    <div className="page search-page">
      <h1 className="page-title">Search</h1>

      {/* Plugin selector */}
      <div className="plugin-pills" style={{ marginBottom: "1rem" }}>
        {plugins.map((plugin) => (
          <button
            key={plugin.id}
            className={`plugin-pill ${activePluginId === plugin.id ? "active" : ""}`}
            onClick={() => setActivePlugin(plugin.id)}
          >
            {plugin.name}
          </button>
        ))}
      </div>

      {/* Search form */}
      <form className="search-form" onSubmit={handleSearch}>
        <input
          className="search-input"
          type="text"
          placeholder="Search for movies, shows, anime..."
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          disabled={!activePluginId}
        />
        <button
          className="btn-primary"
          type="submit"
          disabled={!activePluginId || !inputValue.trim() || searchLoading}
        >
          {searchLoading ? "Searching..." : "Search"}
        </button>
      </form>

      {!activePluginId && plugins.length > 0 && (
        <p className="hint">Select a plugin to search</p>
      )}

      {/* Results */}
      {searchResults.length > 0 && (
        <>
          <p className="results-count">{searchResults.length} results for "{searchQuery}"</p>
          <div className="card-grid">
            {searchResults.map((result) => (
              <MediaCard
                key={result.id}
                id={result.id}
                pluginId={activePluginId!}
                title={result.title}
                posterUrl={result.poster_url}
                mediaType={result.media_type}
                year={result.year}
                rating={result.rating}
              />
            ))}
          </div>
        </>
      )}

      {searchResults.length === 0 && searchQuery && !searchLoading && (
        <div className="empty-state">
          <div className="empty-icon">🔍</div>
          <p>No results found for "{searchQuery}"</p>
        </div>
      )}
    </div>
  );
}
