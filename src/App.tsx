// src/App.tsx
// Root component. Sets up routing and initializes the app.
// 
// React Router handles navigation between pages.
// The layout wraps all pages with a sidebar.

import { useEffect } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { useStore } from "./store";
import { onDownloadProgress, onDownloadComplete, onDownloadFailed } from "./api/tauri";
import Layout from "./components/Layout";
import HomePage from "./pages/HomePage";
import SearchPage from "./pages/SearchPage";
import MediaDetailPage from "./pages/MediaDetailPage";
import BookmarksPage from "./pages/BookmarksPage";
import DownloadsPage from "./pages/DownloadsPage";
import PluginsPage from "./pages/PluginsPage";
import HistoryPage from "./pages/HistoryPage";
import "./index.css";

export default function App() {
  const { loadPlugins, loadBookmarks, loadDownloads, loadHistory,
          updateDownloadProgress, markDownloadComplete, markDownloadFailed } = useStore();

  // Initialize app data when it first loads
  useEffect(() => {
    loadPlugins();
    loadBookmarks();
    loadDownloads();
    loadHistory();
  }, []);

  // Subscribe to download events from the Rust backend
  // These are "server-sent" events (Rust pushes them to us)
  useEffect(() => {
    // listen() returns an unsubscribe function — call it on cleanup
    const unlistenProgress = onDownloadProgress((event) => {
      updateDownloadProgress(event.id, event.progress);
    });

    const unlistenComplete = onDownloadComplete((id) => {
      markDownloadComplete(id);
    });

    const unlistenFailed = onDownloadFailed(({ id, error }) => {
      markDownloadFailed(id, error);
    });

    // Cleanup listeners when component unmounts
    return () => {
      unlistenProgress.then((f) => f());
      unlistenComplete.then((f) => f());
      unlistenFailed.then((f) => f());
    };
  }, []);

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<HomePage />} />
          <Route path="search" element={<SearchPage />} />
          <Route path="media/:pluginId/:mediaId" element={<MediaDetailPage />} />
          <Route path="bookmarks" element={<BookmarksPage />} />
          <Route path="downloads" element={<DownloadsPage />} />
          <Route path="plugins" element={<PluginsPage />} />
          <Route path="history" element={<HistoryPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
