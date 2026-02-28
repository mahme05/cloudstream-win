// src/api/tauri.ts
// A typed wrapper around Tauri's invoke() calls.
// 
// Instead of calling invoke("search_content", {...}) everywhere with raw strings,
// we define typed functions here. If you change a command name in Rust,
// you only need to update it in one place here.
//
// invoke() is like fetch() but it calls your Rust backend instead of a server.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  PluginInfo,
  SearchResult,
  Episode,
  StreamSource,
  Bookmark,
  WatchHistory,
  Download,
  DownloadProgressEvent,
} from "../types";

// ─── PLUGIN API ───

export const api = {
  plugins: {
    list: () =>
      invoke<PluginInfo[]>("list_plugins"),

    install: (wasmPath: string, info: PluginInfo) =>
      invoke<void>("install_plugin", { wasmPath, info }),

    search: (pluginId: string, query: string) =>
      invoke<SearchResult[]>("search_content", { pluginId, query }),

    getEpisodes: (pluginId: string, showId: string) =>
      invoke<Episode[]>("get_episodes", { pluginId, showId }),

    getStreams: (pluginId: string, mediaId: string) =>
      invoke<StreamSource[]>("get_streams", { pluginId, mediaId }),
  },

  // ─── BOOKMARKS API ───
  bookmarks: {
    list: () =>
      invoke<Bookmark[]>("get_bookmarks"),

    add: (payload: {
      media_id: string;
      plugin_id: string;
      title: string;
      poster_url?: string;
      media_type: string;
    }) => invoke<Bookmark>("add_bookmark", { payload }),

    remove: (mediaId: string, pluginId: string) =>
      invoke<void>("remove_bookmark", { mediaId, pluginId }),
  },

  // ─── STREAMING API ───
  streaming: {
    play: (url: string, title: string, headers?: Record<string, string>) =>
      invoke<void>("play_stream", { url, title, headers }),

    getHistory: (limit?: number) =>
      invoke<WatchHistory[]>("get_watch_history", { limit }),

    updateProgress: (payload: {
      media_id: string;
      plugin_id: string;
      episode_id?: string;
      title: string;
      episode_title?: string;
      progress_seconds: number;
      duration_seconds: number;
    }) => invoke<void>("update_watch_progress", { payload }),
  },

  // ─── DOWNLOADS API ───
  downloads: {
    list: () =>
      invoke<Download[]>("get_downloads"),

    start: (payload: {
      media_id: string;
      plugin_id: string;
      title: string;
      episode_title?: string;
      url: string;
      save_path: string;
    }) => invoke<string>("start_download", { payload }),

    cancel: (downloadId: string) =>
      invoke<void>("cancel_download", { downloadId }),
  },
};

// ─── EVENT LISTENERS ───
// These are for events the Rust backend emits (push, not request/response)

export function onDownloadProgress(
  callback: (event: DownloadProgressEvent) => void
) {
  return listen<DownloadProgressEvent>("download-progress", (event) => {
    callback(event.payload);
  });
}

export function onDownloadComplete(callback: (id: string) => void) {
  return listen<{ id: string }>("download-complete", (event) => {
    callback(event.payload.id);
  });
}

export function onDownloadFailed(
  callback: (data: { id: string; error: string }) => void
) {
  return listen<{ id: string; error: string }>("download-failed", (event) => {
    callback(event.payload);
  });
}
