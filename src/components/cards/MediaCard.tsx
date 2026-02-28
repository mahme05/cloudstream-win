// src/components/cards/MediaCard.tsx
// Reusable card component for search results and bookmarks.

import { useNavigate } from "react-router-dom";

interface Props {
  id: string;
  pluginId: string;
  title: string;
  posterUrl?: string;
  mediaType: string;
  year?: number;
  rating?: number;
}

export default function MediaCard({ id, pluginId, title, posterUrl, mediaType, year, rating }: Props) {
  const navigate = useNavigate();

  return (
    <div
      className="media-card"
      onClick={() => navigate(`/media/${pluginId}/${id}`)}
    >
      <div className="media-card-poster">
        {posterUrl ? (
          <img src={posterUrl} alt={title} loading="lazy" />
        ) : (
          <div className="media-card-placeholder">
            <span>{mediaType === "movie" ? "🎬" : "📺"}</span>
          </div>
        )}
        <div className="media-card-type">{mediaType}</div>
      </div>
      <div className="media-card-info">
        <div className="media-card-title">{title}</div>
        <div className="media-card-meta">
          {year && <span>{year}</span>}
          {rating && <span>⭐ {rating.toFixed(1)}</span>}
        </div>
      </div>
    </div>
  );
}
