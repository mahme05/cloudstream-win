// src/api/tauri.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  PluginInfo, SearchResult, Episode, StreamSource,
  Bookmark, WatchHistory, Download, DownloadProgressEvent,
} from "../types";

export const api = {
  plugins: {
    list: () => invoke<PluginInfo[]>("list_plugins"),
    installFile: (jsPath: string) => invoke<PluginInfo>("install_plugin", { payload: { jsPath } }),
    installUrl: (url: string) => invoke<string>("fetch_url", { url }).then(source => invoke<PluginInfo>("install_plugin", { payload: { source } })),
    remove: (pluginId: string) => invoke<boolean>("remove_plugin", { pluginId }),
    search: (pluginId: string, query: string) =>
      invoke<SearchResult[]>("search_content", { pluginId, query }),
    getEpisodes: (pluginId: string, showId: string) =>
      invoke<Episode[]>("get_episodes", { pluginId, showId }),
    getStreams: (pluginId: string, mediaId: string) =>
      invoke<StreamSource[]>("get_streams", { pluginId, mediaId }),
  },

  bookmarks: {
    list: () => invoke<Bookmark[]>("get_bookmarks"),
    add: (payload: Omit<Bookmark, "id" | "created_at">) =>
      invoke<Bookmark>("add_bookmark", { payload }),
    remove: (mediaId: string, pluginId: string) =>
      invoke<void>("remove_bookmark", { mediaId, pluginId }),
  },

  streaming: {
    // Returns which player was used: "vlc" | "mpv" | "mpc" | "default"
    play: (url: string, title: string, headers?: Record<string, string>) =>
      invoke<string>("play_stream", { url, title, headers }),
    getHistory: (limit?: number) =>
      invoke<WatchHistory[]>("get_watch_history", { limit }),
    updateProgress: (payload: Omit<WatchHistory, "id" | "watched_at">) =>
      invoke<void>("update_watch_progress", { payload }),
  },

  downloads: {
    list: () => invoke<Download[]>("get_downloads"),
    start: (payload: Omit<Download, "id" | "status" | "progress" | "created_at">) =>
      invoke<string>("start_download", { payload }),
    cancel: (downloadId: string) => invoke<void>("cancel_download", { downloadId }),
  },
};

export function onDownloadProgress(cb: (e: DownloadProgressEvent) => void) {
  return listen<DownloadProgressEvent>("download-progress", (e) => cb(e.payload));
}
export function onDownloadComplete(cb: (id: string) => void) {
  return listen<{ id: string }>("download-complete", (e) => cb(e.payload.id));
}
export function onDownloadFailed(cb: (d: { id: string; error: string }) => void) {
  return listen<{ id: string; error: string }>("download-failed", (e) => cb(e.payload));
}
