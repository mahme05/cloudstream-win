// src/store/index.ts
// Global state management using Zustand.
// 
// Zustand is like React's useState but shared across all components.
// Instead of prop-drilling, any component can read/write this store.
//
// Structure: separate "slices" for each domain (plugins, bookmarks, etc.)

import { create } from "zustand";
import type { PluginInfo, SearchResult, Bookmark, Download, WatchHistory } from "../types";
import { api } from "../api/tauri";

interface AppStore {
  // ─── PLUGINS ───
  plugins: PluginInfo[];
  loadPlugins: () => Promise<void>;

  // ─── SEARCH ───
  searchResults: SearchResult[];
  searchQuery: string;
  searchLoading: boolean;
  activePluginId: string | null;
  search: (pluginId: string, query: string) => Promise<void>;
  setActivePlugin: (id: string | null) => void;
  clearSearch: () => void;

  // ─── BOOKMARKS ───
  bookmarks: Bookmark[];
  loadBookmarks: () => Promise<void>;
  addBookmark: (payload: Omit<Bookmark, "id" | "created_at">) => Promise<void>;
  removeBookmark: (mediaId: string, pluginId: string) => Promise<void>;
  isBookmarked: (mediaId: string, pluginId: string) => boolean;

  // ─── DOWNLOADS ───
  downloads: Download[];
  loadDownloads: () => Promise<void>;
  updateDownloadProgress: (id: string, progress: number) => void;
  markDownloadComplete: (id: string) => void;
  markDownloadFailed: (id: string, error: string) => void;

  // ─── WATCH HISTORY ───
  history: WatchHistory[];
  loadHistory: () => Promise<void>;
}

export const useStore = create<AppStore>((set, get) => ({
  // ─── PLUGINS ───
  plugins: [],
  loadPlugins: async () => {
    // Plugins live in memory only — re-list from the Rust plugin manager
    const plugins = await api.plugins.list();
    set({ plugins });
  },

  // ─── SEARCH ───
  searchResults: [],
  searchQuery: "",
  searchLoading: false,
  activePluginId: null,

  search: async (pluginId: string, query: string) => {
    set({ searchLoading: true, searchQuery: query, activePluginId: pluginId });
    try {
      const results = await api.plugins.search(pluginId, query);
      set({ searchResults: results, searchLoading: false });
    } catch (error) {
      console.error("Search failed:", error);
      set({ searchLoading: false });
    }
  },

  setActivePlugin: (id) => set({ activePluginId: id }),
  clearSearch: () => set({ searchResults: [], searchQuery: "" }),

  // ─── BOOKMARKS ───
  bookmarks: [],
  loadBookmarks: async () => {
    const bookmarks = await api.bookmarks.list();
    set({ bookmarks });
  },

  addBookmark: async (payload) => {
    const bookmark = await api.bookmarks.add(payload);
    set((state) => ({ bookmarks: [bookmark, ...state.bookmarks] }));
  },

  removeBookmark: async (mediaId, pluginId) => {
    await api.bookmarks.remove(mediaId, pluginId);
    set((state) => ({
      bookmarks: state.bookmarks.filter(
        (b) => !(b.media_id === mediaId && b.plugin_id === pluginId)
      ),
    }));
  },

  isBookmarked: (mediaId, pluginId) => {
    return get().bookmarks.some(
      (b) => b.media_id === mediaId && b.plugin_id === pluginId
    );
  },

  // ─── DOWNLOADS ───
  downloads: [],
  loadDownloads: async () => {
    const downloads = await api.downloads.list();
    set({ downloads });
  },

  updateDownloadProgress: (id, progress) => {
    set((state) => ({
      downloads: state.downloads.map((d) =>
        d.id === id ? { ...d, progress, status: "downloading" as const } : d
      ),
    }));
  },

  markDownloadComplete: (id) => {
    set((state) => ({
      downloads: state.downloads.map((d) =>
        d.id === id ? { ...d, progress: 1, status: "done" as const } : d
      ),
    }));
  },

  markDownloadFailed: (id, _error) => {
    set((state) => ({
      downloads: state.downloads.map((d) =>
        d.id === id ? { ...d, status: "failed" as const } : d
      ),
    }));
  },

  // ─── WATCH HISTORY ───
  history: [],
  loadHistory: async () => {
    const history = await api.streaming.getHistory(50);
    set({ history });
  },
}));
