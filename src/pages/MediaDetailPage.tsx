// src/pages/MediaDetailPage.tsx
// Detail page for a movie or show.
// Shows: poster, description, episodes list, stream quality selector, bookmark button.

import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useStore } from "../store";
import { api } from "../api/tauri";
import type { Episode, StreamSource } from "../types";

export default function MediaDetailPage() {
  // Get URL params: /media/:pluginId/:mediaId
  const { pluginId, mediaId } = useParams<{ pluginId: string; mediaId: string }>();
  const navigate = useNavigate();

  const { isBookmarked, addBookmark, removeBookmark, searchResults } = useStore();

  // Find the media info from existing search results (cached)
  const mediaInfo = searchResults.find((r) => r.id === mediaId);

  const [episodes, setEpisodes] = useState<Episode[]>([]);
  const [streams, setStreams] = useState<StreamSource[]>([]);
  const [selectedEpisode, setSelectedEpisode] = useState<Episode | null>(null);
  const [loadingEpisodes, setLoadingEpisodes] = useState(false);
  const [loadingStreams, setLoadingStreams] = useState(false);
  const [showStreamPicker, setShowStreamPicker] = useState(false);

  const bookmarked = isBookmarked(mediaId!, pluginId!);
  const isShow = mediaInfo?.media_type !== "movie";

  // Load episodes if it's a show
  useEffect(() => {
    if (!pluginId || !mediaId) return;
    if (isShow) {
      setLoadingEpisodes(true);
      api.plugins.getEpisodes(pluginId, mediaId)
        .then(setEpisodes)
        .catch(console.error)
        .finally(() => setLoadingEpisodes(false));
    }
  }, [pluginId, mediaId, isShow]);

  const handlePlay = async (episodeId?: string) => {
    if (!pluginId || !mediaId) return;
    const id = episodeId || mediaId;

    setLoadingStreams(true);
    try {
      const sources = await api.plugins.getStreams(pluginId, id);
      setStreams(sources);
      setShowStreamPicker(true);
    } catch (err) {
      console.error("Failed to get streams:", err);
    } finally {
      setLoadingStreams(false);
    }
  };

  const handleStreamSelect = async (stream: StreamSource) => {
    setShowStreamPicker(false);
    await api.streaming.play(stream.url, mediaInfo?.title || "Video", stream.headers);
  };

  const toggleBookmark = async () => {
    if (!pluginId || !mediaId || !mediaInfo) return;
    if (bookmarked) {
      await removeBookmark(mediaId, pluginId);
    } else {
      await addBookmark({
        media_id: mediaId,
        plugin_id: pluginId,
        title: mediaInfo.title,
        poster_url: mediaInfo.poster_url,
        media_type: mediaInfo.media_type,
      });
    }
  };

  if (!mediaInfo) {
    return (
      <div className="page">
        <button className="back-btn" onClick={() => navigate(-1)}>← Back</button>
        <div className="empty-state">
          <p>Media not found. Go back and try again.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="page media-detail-page">
      <button className="back-btn" onClick={() => navigate(-1)}>← Back</button>

      {/* Hero section */}
      <div className="media-hero">
        {mediaInfo.poster_url && (
          <img className="media-poster" src={mediaInfo.poster_url} alt={mediaInfo.title} />
        )}
        <div className="media-info">
          <h1 className="media-title">{mediaInfo.title}</h1>
          <div className="media-meta">
            {mediaInfo.year && <span>{mediaInfo.year}</span>}
            {mediaInfo.rating && <span>⭐ {mediaInfo.rating.toFixed(1)}</span>}
            <span className="media-type-badge">{mediaInfo.media_type}</span>
          </div>
          {mediaInfo.description && (
            <p className="media-description">{mediaInfo.description}</p>
          )}
          <div className="media-actions">
            {/* Play button (for movies, or if no episodes loaded yet) */}
            {!isShow && (
              <button
                className="btn-primary btn-play"
                onClick={() => handlePlay()}
                disabled={loadingStreams}
              >
                {loadingStreams ? "Loading..." : "▶ Play"}
              </button>
            )}
            <button
              className={`btn-bookmark ${bookmarked ? "active" : ""}`}
              onClick={toggleBookmark}
            >
              {bookmarked ? "♥ Bookmarked" : "♡ Bookmark"}
            </button>
          </div>
        </div>
      </div>

      {/* Stream quality picker modal */}
      {showStreamPicker && streams.length > 0 && (
        <div className="modal-overlay" onClick={() => setShowStreamPicker(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h3>Select Quality</h3>
            {streams.map((stream, i) => (
              <button
                key={i}
                className="stream-option"
                onClick={() => handleStreamSelect(stream)}
              >
                <span className="stream-quality">{stream.quality}</span>
                <span className="stream-format">{stream.format.toUpperCase()}</span>
                {stream.subtitles.length > 0 && (
                  <span className="stream-subs">CC</span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Episodes list (for shows/anime) */}
      {isShow && (
        <section className="section">
          <h2 className="section-title">Episodes</h2>
          {loadingEpisodes ? (
            <div className="loading">Loading episodes...</div>
          ) : (
            <div className="episodes-list">
              {episodes.map((ep) => (
                <div key={ep.id} className="episode-row">
                  <div className="episode-info">
                    <span className="episode-number">
                      {ep.season ? `S${ep.season}E${ep.episode_number}` : `EP ${ep.episode_number}`}
                    </span>
                    <span className="episode-title">{ep.title}</span>
                    {ep.description && (
                      <p className="episode-desc">{ep.description}</p>
                    )}
                  </div>
                  <button
                    className="btn-play-episode"
                    onClick={() => handlePlay(ep.id)}
                    disabled={loadingStreams}
                  >
                    ▶
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>
      )}
    </div>
  );
}
