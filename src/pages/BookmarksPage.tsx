// src/pages/BookmarksPage.tsx
import { useStore } from "../store";
import MediaCard from "../components/cards/MediaCard";

export default function BookmarksPage() {
  const bookmarks = useStore((s) => s.bookmarks);

  return (
    <div className="page">
      <h1 className="page-title">Bookmarks</h1>
      {bookmarks.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">♡</div>
          <p>No bookmarks yet. Find something to watch!</p>
        </div>
      ) : (
        <div className="card-grid">
          {bookmarks.map((b) => (
            <MediaCard
              key={b.id}
              id={b.media_id}
              pluginId={b.plugin_id}
              title={b.title}
              posterUrl={b.poster_url}
              mediaType={b.media_type}
            />
          ))}
        </div>
      )}
    </div>
  );
}
