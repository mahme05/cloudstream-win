<div align="center">

# 🎬 CloudStream Win

**A free, open-source streaming app for Windows powered by a JavaScript plugin system.**

Install plugins to add any content source — anime, movies, TV shows, and more.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)](../../releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange)](https://tauri.app)
[![GitHub release](https://img.shields.io/github/v/release/YOUR_USERNAME/cloudstream-win?color=red)](../../releases/latest)

[**⬇ Download**](../../releases/latest) · [**🔌 Browse Plugins**](#-available-plugins) · [**✍️ Write a Plugin**](#-writing-a-plugin) · [**🐛 Report a Bug**](../../issues)

</div>

---

## 📖 Table of Contents

- [What is this?](#-what-is-this)
- [Features](#-features)
- [Installation](#-installation)
- [How to use plugins](#-how-to-use-plugins)
- [Available plugins](#-available-plugins)
- [Writing a plugin](#-writing-a-plugin)
- [Plugin API reference](#-plugin-api-reference)
- [Building from source](#-building-from-source)
- [Contributing](#-contributing)
- [FAQ](#-faq)
- [License](#-license)

---

## 🤔 What is this?

CloudStream Win is a Windows desktop streaming app inspired by [CloudStream](https://github.com/recloudstream/cloudstream) for Android, rebuilt for Windows using [Tauri](https://tauri.app) (Rust + React).

Instead of having content built-in, **you install plugins** — small JavaScript files that tell the app where to find streams. This means:

- The app itself is completely legal and safe to distribute
- Anyone who knows JavaScript can write a plugin for any site
- Plugins are sandboxed — they can only make network requests, nothing else on your system

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🔌 **JS Plugin System** | Install `.js` files to add any content source |
| 🔍 **Search** | Search across any installed plugin |
| 📺 **Episode Browser** | Full season/episode lists for shows and anime |
| ♥ **Bookmarks** | Save titles to your personal library |
| ⬇ **Downloads** | Download episodes for offline viewing with live progress |
| ⏱ **Watch History** | Tracks progress and lets you resume where you left off |
| 🛡 **Sandboxed Plugins** | Plugins cannot access your filesystem or run arbitrary code |
| 🌙 **Dark Theme** | Clean dark UI designed for media browsing |

---

## 📦 Installation

### Requirements
- Windows 10 or Windows 11 (64-bit)
- [Microsoft Edge WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) — already installed on most modern Windows systems

### Steps

1. Go to the [**Releases**](../../releases/latest) page
2. Download `cloudstream-win_x.x.x_x64_en-US.msi`
3. Run the installer and follow the prompts
4. Launch **CloudStream Win** from the Start Menu
5. Go to the **Plugins** tab and install your first plugin

> **Windows SmartScreen warning:** You may see a warning because the app is not code-signed.
> Click **"More info" → "Run anyway"** to proceed.
> This is normal for open-source apps without a paid signing certificate.
> You can verify the installer hash against the one listed in the release notes.

---

## 🔌 How to use plugins

Plugins are `.js` files that add content sources. Without at least one plugin installed, the app has no content to show.

### Installing a plugin

**Method 1 — From a file:**
1. Download a `.js` plugin file (see [Available plugins](#-available-plugins) below)
2. Open the app → **Plugins** → **Manual Install**
3. Click **Browse for .js file** and select your downloaded file

**Method 2 — From a URL:**
1. Find a raw URL to a `.js` plugin (e.g. a raw GitHub link)
2. Open the app → **Plugins** → **Manual Install**
3. Paste the URL into the URL field and click **Install**

**Method 3 — Browse list:**
1. Open the app → **Plugins** → **Browse**
2. Click **Install** next to any plugin in the list

### Searching for content

1. Go to the **Search** tab
2. Select a plugin from the buttons at the top
3. Type a search query and press **Search**
4. Click a result to view details, episodes, and stream sources

---

## 🔌 Available Plugins

Download these from the [`plugins/`](./plugins) folder in this repository.

| File | Content | Source | Requires |
|------|---------|--------|---------|
| [`gogoanime.js`](./plugins/gogoanime.js) | Anime | GogoAnime | [Consumet API](#consumet-api-setup) |
| [`zoro.js`](./plugins/zoro.js) | Anime HD (SUB + DUB) | Hianime | [Consumet API](#consumet-api-setup) |
| [`movies-tmdb.js`](./plugins/movies-tmdb.js) | Movies & TV Shows | TMDB + VidSrc | Free [TMDB API key](https://www.themoviedb.org/settings/api) |

> **Want your plugin listed here?** Open a pull request — see [Contributing](#-contributing).

---

### Consumet API setup

The anime plugins use [Consumet](https://github.com/consumet/consumet.ts) — a free open-source API that provides streams from 50+ providers including GogoAnime, Hianime, Crunchyroll, and more.

You need to run a Consumet instance yourself, or use a public one.

#### Option A — Self-host with Docker (recommended)

```bash
# Install Docker Desktop first: https://www.docker.com/products/docker-desktop
docker run -d -p 3000:3000 consumet/consumet-api
```

Your instance runs at `http://localhost:3000`. Edit the plugin file and set:

```js
const BASE_URL = "http://localhost:3000";
```

#### Option B — Use a public instance

```js
const BASE_URL = "https://api.consumet.org";
```

> ⚠️ Public instances may be slow or unavailable. Self-hosting is more reliable.

---

## ✍️ Writing a Plugin

Plugins are plain `.js` files. If you know JavaScript, you can write one in under an hour.

Every plugin needs:
- A **header comment** with plugin metadata
- A **`search()`** function
- A **`getEpisodes()`** function
- A **`getStreams()`** function

Start by copying [`plugins/TEMPLATE.js`](./plugins/TEMPLATE.js).

---

### Complete working example

```js
// @plugin-info {"id":"my-anime-site","name":"My Anime Site","version":"1.0.0","description":"Anime from mysite.com","author":"YourName","icon_url":null,"supported_types":["anime"],"is_builtin":false}

const BASE = "https://mysite.com/api";

// Called when the user searches for something.
// Return JSON.stringify() of a SearchResult array.
async function search(query) {
    const res = await fetch(`${BASE}/search?q=${encodeURIComponent(query)}`);
    const data = await res.json();

    return JSON.stringify(data.results.map(item => ({
        id: item.id,               // passed to getEpisodes() and getStreams()
        title: item.title,
        poster_url: item.image,    // cover art URL, or null
        media_type: "anime",       // "anime" | "movie" | "show"
        year: item.year,           // release year, or null
        rating: item.score,        // score out of 10, or null
        description: item.synopsis // short description, or null
    })));
}

// Called when the user opens a show or anime page.
// Return JSON.stringify() of an Episode array.
// For movies, return JSON.stringify([]) — movies have no episodes.
async function getEpisodes(showId) {
    const res = await fetch(`${BASE}/info/${showId}`);
    const data = await res.json();

    return JSON.stringify(data.episodes.map(ep => ({
        id: ep.id,                          // passed to getStreams()
        title: ep.title || `Episode ${ep.number}`,
        season: ep.season || null,
        episode_number: ep.number,
        thumbnail_url: ep.image || null,
        description: ep.description || null
    })));
}

// Called when the user clicks Play on a movie or episode.
// Return JSON.stringify() of a StreamSource array.
// Return multiple sources at different qualities if available.
async function getStreams(mediaId) {
    const res = await fetch(`${BASE}/watch/${mediaId}`);
    const data = await res.json();

    return JSON.stringify(data.sources.map(src => ({
        url: src.url,                                         // direct video URL
        quality: src.quality || "auto",                       // "1080p" | "720p" | "480p" | "auto"
        format: src.url.includes(".m3u8") ? "hls" : "mp4",   // "hls" | "mp4" | "dash"
        subtitles: (data.subtitles || []).map(sub => ({
            url: sub.url,
            language: sub.lang,
            label: sub.lang
        })),
        headers: { "Referer": "https://mysite.com" }          // add if the site needs it
    })));
}
```

---

### Useful patterns

**Scraping HTML** (when the site has no API):
```js
const res = await fetch("https://mysite.com/search?q=naruto");
const html = await res.text();

// Extract data with regex
const ids = [...html.matchAll(/href="\/anime\/([\w-]+)"/g)].map(m => m[1]);
const titles = [...html.matchAll(/<h3 class="name">(.*?)<\/h3>/g)].map(m => m[1]);
```

**POST request**:
```js
const res = await fetch("https://mysite.com/api/search", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query: "naruto", page: 1 })
});
const data = await res.json();
```

**Setting custom headers** (for sites that block scrapers):
```js
const res = await fetch("https://mysite.com/video", {
    headers: {
        "Referer": "https://mysite.com",
        "X-Requested-With": "XMLHttpRequest"
    }
});
```

**Debugging your plugin:**
Run the app in dev mode (`npm run tauri dev`) and press `F12` to open DevTools.
`console.log()` output from your plugin will appear in the Console tab.

---

## 📋 Plugin API Reference

### Header comment

Every plugin **must** start with this comment (everything on one line):

```js
// @plugin-info {"id":"...","name":"...","version":"...","description":"...","author":"...","icon_url":null,"supported_types":["anime"],"is_builtin":false}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique ID. Lowercase, no spaces. e.g. `"gogoanime"` |
| `name` | string | Display name shown in the app |
| `version` | string | Semver string e.g. `"1.0.0"` |
| `description` | string | One-line description of what the plugin provides |
| `author` | string | Your name or GitHub username |
| `icon_url` | string\|null | URL to a 64×64 PNG icon, or `null` |
| `supported_types` | string[] | Any of: `"anime"`, `"movie"`, `"show"` |
| `is_builtin` | boolean | Always `false` for community plugins |

---

### `search(query)` return type

```ts
{
    id: string           // required — unique ID for this result
    title: string        // required — display title
    media_type: string   // required — "anime" | "movie" | "show"
    poster_url?: string  // optional — cover image URL
    year?: number        // optional — release year
    rating?: number      // optional — score out of 10
    description?: string // optional — synopsis
}[]
```

---

### `getEpisodes(showId)` return type

```ts
{
    id: string              // required — episode ID, passed to getStreams()
    title: string           // required — episode title
    episode_number: number  // required — episode number within season
    season?: number         // optional — season number
    thumbnail_url?: string  // optional — episode thumbnail
    description?: string    // optional — episode synopsis
}[]
```

> Return `[]` for movies.

---

### `getStreams(mediaId)` return type

```ts
{
    url: string       // required — direct video URL (.m3u8, .mp4) or embed URL
    quality: string   // required — "1080p" | "720p" | "480p" | "auto"
    format: string    // required — "hls" | "mp4" | "dash" | "embed"
    subtitles: {      // required — array of subtitle tracks (can be [])
        url: string
        language: string
        label: string
    }[]
    headers: {        // required — HTTP headers for the player (can be {})
        [key: string]: string
    }
}[]
```

---

## 🏗 Building from Source

### Prerequisites

| Tool | Where to get it | Notes |
|------|----------------|-------|
| **Rust** | https://rustup.rs | Run installer, restart terminal after |
| **Node.js** | https://nodejs.org | Use the LTS version |
| **MSVC Build Tools** | https://visualstudio.microsoft.com/visual-cpp-build-tools/ | Select "Desktop development with C++" — required by Rust on Windows |
| **WebView2** | https://developer.microsoft.com/microsoft-edge/webview2/ | Usually pre-installed on Windows 11 |

### Run in development

```bash
git clone https://github.com/YOUR_USERNAME/cloudstream-win.git
cd cloudstream-win
npm install
npm run tauri dev
```

The app window opens with hot reload — save any `.tsx` file and it updates instantly. Rust changes require restarting the command.

### Build a release installer

```bash
npm run tauri build
```

Output: `src-tauri/target/release/bundle/msi/cloudstream-win_x.x.x_x64_en-US.msi`

---

### Project structure

```
cloudstream-win/
│
├── src/                          ← React frontend (TypeScript)
│   ├── api/tauri.ts              ← All invoke() calls in one typed place
│   ├── store/index.ts            ← Global state with Zustand
│   ├── types/index.ts            ← TypeScript types mirroring Rust structs
│   ├── components/Layout.tsx     ← Sidebar + routing shell
│   └── pages/                    ← One file per screen
│       ├── HomePage.tsx
│       ├── SearchPage.tsx
│       ├── MediaDetailPage.tsx
│       ├── BookmarksPage.tsx
│       ├── DownloadsPage.tsx
│       ├── HistoryPage.tsx
│       └── PluginsPage.tsx
│
├── src-tauri/src/                ← Rust backend
│   ├── lib.rs                    ← Entry point, wires everything together
│   ├── commands/                 ← IPC handlers (callable from React)
│   │   ├── plugins.rs            ← install / remove / search / streams
│   │   ├── bookmarks.rs          ← bookmark CRUD
│   │   ├── downloads.rs          ← download manager with progress events
│   │   └── streaming.rs          ← play stream + watch history
│   ├── plugin_runtime/mod.rs     ← QuickJS engine + fetch() sandbox
│   ├── db/mod.rs                 ← SQLite via sqlx
│   └── player/mod.rs             ← mpv player wrapper (stub)
│
└── plugins/                      ← Community plugin .js files
    ├── TEMPLATE.js               ← Copy this to start writing a plugin
    ├── gogoanime.js
    ├── zoro.js
    └── movies-tmdb.js
```

---

## 🤝 Contributing

All contributions are welcome.

### Add a plugin (easiest)

1. Write and test a `.js` plugin file
2. Fork this repo
3. Add your file to `plugins/`
4. Add it to the plugin table in this README
5. Open a pull request

### Fix a bug or add a feature

1. Fork and clone the repo
2. Create a branch: `git checkout -b fix/my-fix`
3. Make changes and test with `npm run tauri dev`
4. Open a pull request with a description of what you changed and why

### Guidelines

- Plugin files must be tested and working before submitting
- One plugin per pull request
- Never include API keys or credentials in committed files
- Plugin IDs must be unique — check existing plugins before choosing one

---

## ❓ FAQ

**Is this legal?**
The app itself contains no content and is completely legal. Whether accessing a specific site through a plugin is legal depends on your country and that site's terms of service. Use responsibly.

**Why does Windows warn about the installer?**
Code signing certificates cost money and this is a free open-source project. The warning is a standard Windows SmartScreen message. You can verify the installer's SHA256 hash against the one published in the release notes.

**Can I use CloudStream's Android plugins?**
No. CloudStream plugins are written in Kotlin for Android's JVM. This app uses JavaScript plugins — a completely different format. However, most sites that have CloudStream plugins can be ported to JS for this app fairly easily.

**The anime plugins aren't working.**
You need a running Consumet API instance. See [Consumet API setup](#consumet-api-setup).

**How do I update a plugin?**
Go to **Plugins → Installed**, remove the old version, then reinstall the updated file.

**Where is my data stored?**
`%APPDATA%\com.cloudstream.win\` — this contains your SQLite database with bookmarks, history, and download records.

**Can I request a feature?**
Yes — open an [issue](../../issues) with the label `enhancement`.

---

## 📄 License

[MIT](LICENSE) — free to use, modify, and distribute.

This project is not affiliated with the original CloudStream for Android.

---

<div align="center">

Built with [Tauri](https://tauri.app) · [React](https://react.dev) · [Rust](https://rust-lang.org) · [QuickJS](https://bellard.org/quickjs)

**[⬆ Back to top](#-cloudstream-win)**

</div>
