// src/pages/DownloadsPage.tsx
import { useStore } from "../store";
import { api } from "../api/tauri";

export default function DownloadsPage() {
  const downloads = useStore((s) => s.downloads);

  const handleCancel = async (id: string) => {
    await api.downloads.cancel(id);
  };

  const statusIcon = (status: string) => {
    switch (status) {
      case "downloading": return "⬇";
      case "done": return "✓";
      case "failed": return "✗";
      case "cancelled": return "⊘";
      default: return "…";
    }
  };

  return (
    <div className="page">
      <h1 className="page-title">Downloads</h1>
      {downloads.length === 0 ? (
        <div className="empty-state">
          <div className="empty-icon">↓</div>
          <p>No downloads yet.</p>
        </div>
      ) : (
        <div className="downloads-list">
          {downloads.map((d) => (
            <div key={d.id} className={`download-row status-${d.status}`}>
              <div className="download-icon">{statusIcon(d.status)}</div>
              <div className="download-info">
                <div className="download-title">{d.title}</div>
                {d.episode_title && (
                  <div className="download-episode">{d.episode_title}</div>
                )}
                {d.status === "downloading" && (
                  <div className="progress-bar">
                    <div className="progress-fill" style={{ width: `${d.progress * 100}%` }} />
                    <span className="progress-text">{Math.round(d.progress * 100)}%</span>
                  </div>
                )}
              </div>
              {d.status === "downloading" && (
                <button className="btn-cancel" onClick={() => handleCancel(d.id)}>
                  Cancel
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
