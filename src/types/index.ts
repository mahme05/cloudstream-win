// src/types/index.ts
// TypeScript types that mirror our Rust structs.
// Keep these in sync with the Rust types in plugin_runtime/mod.rs and db/mod.rs!

export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  icon_url?: string;
  supported_types: string[];
  is_builtin: boolean;
}

export interface SearchResult {
  id: string;
  title: string;
  poster_url?: string;
  media_type: "movie" | "show" | "anime";
  year?: number;
  rating?: number;
  description?: string;
}

export interface Episode {
  id: string;
  title: string;
  season?: number;
  episode_number: number;
  thumbnail_url?: string;
  description?: string;
}

export interface StreamSource {
  url: string;
  quality: string;      // "1080p", "720p", etc.
  format: string;       // "hls", "mp4", "dash"
  subtitles: SubtitleTrack[];
  headers: Record<string, string>;
}

export interface SubtitleTrack {
  url: string;
  language: string;
  label: string;
}

export interface Bookmark {
  id: string;
  media_id: string;
  plugin_id: string;
  title: string;
  poster_url?: string;
  media_type: string;
  created_at: string;
}

export interface WatchHistory {
  id: string;
  media_id: string;
  plugin_id: string;
  episode_id?: string;
  title: string;
  episode_title?: string;
  progress_seconds: number;
  duration_seconds: number;
  watched_at: string;
}

export interface Download {
  id: string;
  media_id: string;
  plugin_id: string;
  title: string;
  episode_title?: string;
  url: string;
  save_path: string;
  status: "pending" | "downloading" | "done" | "failed" | "cancelled";
  progress: number;  // 0.0 - 1.0
  created_at: string;
}

export interface DownloadProgressEvent {
  id: string;
  progress: number;
  downloaded: number;
  total: number;
}
