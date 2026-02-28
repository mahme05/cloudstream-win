// src/pages/HistoryPage.tsx
import { useStore } from "../store";
import { useNavigate } from "react-router-dom";

export default function HistoryPage() {
  const history = useStore((s) => s.history);
  const navigate = useNavigate();

  return (
    <div className="page">
      <h1 className="page-title">Watch History</h1>
      {history.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">⏱</div>
          <p>Nothing watched yet.</p>
        </div>
      ) : (
        <div className="history-list">
          {history.map((item) => (
            <div
              key={item.id}
              className="history-row"
              onClick={() => navigate(`/media/${item.plugin_id}/${item.media_id}`)}
            >
              <div className="history-info">
                <div className="history-title">{item.title}</div>
                {item.episode_title && (
                  <div className="history-episode">{item.episode_title}</div>
                )}
                <div className="history-date">
                  {new Date(item.watched_at).toLocaleDateString()}
                </div>
              </div>
              {item.duration_seconds > 0 && (
                <div className="history-progress">
                  <div className="progress-bar">
                    <div
                      className="progress-fill"
                      style={{ width: `${(item.progress_seconds / item.duration_seconds) * 100}%` }}
                    />
                  </div>
                  <span className="progress-label">
                    {Math.round((item.progress_seconds / item.duration_seconds) * 100)}%
                  </span>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
