// @plugin-info {"id":"consumet-gogoanime","name":"GogoAnime","version":"1.0.0","description":"Anime streaming via GogoAnime (powered by Consumet API)","author":"CloudStream Win","icon_url":null,"supported_types":["anime"],"is_builtin":false}

// GogoAnime plugin using the public Consumet API
// Consumet is an open-source API that scrapes streaming sites legally for personal use.
// API docs: https://docs.consumet.org
//
// To use this plugin you need a Consumet API instance.
// You can self-host one: https://github.com/consumet/api.consumet.org
// Or use the public demo (may be slow): https://api.consumet.org
//
// HOW TO CHANGE THE API URL:
// Edit the BASE_URL constant below to point to your own instance.

const BASE_URL = "https://api.consumet.org";
const PROVIDER = "gogoanime";

async function search(query) {
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/${encodeURIComponent(query)}`);
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const results = (data.results || []).map(item => ({
        id: item.id,
        title: item.title,
        poster_url: item.image || null,
        media_type: "anime",
        year: item.releaseDate ? parseInt(item.releaseDate) : null,
        rating: null,
        description: item.subOrDub ? `[${item.subOrDub.toUpperCase()}]` : null,
    }));

    return JSON.stringify(results);
}

async function getEpisodes(showId) {
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/info/${encodeURIComponent(showId)}`);
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const episodes = (data.episodes || []).map(ep => ({
        id: ep.id,
        title: ep.title || `Episode ${ep.number}`,
        season: null,
        episode_number: ep.number,
        thumbnail_url: null,
        description: null,
    }));

    return JSON.stringify(episodes);
}

async function getStreams(mediaId) {
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/watch/${encodeURIComponent(mediaId)}`);
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const streams = (data.sources || []).map(src => ({
        url: src.url,
        quality: src.quality || "auto",
        format: src.url.includes(".m3u8") ? "hls" : "mp4",
        subtitles: (data.subtitles || []).map(sub => ({
            url: sub.url,
            language: sub.lang || "Unknown",
            label: sub.lang || "Unknown",
        })),
        headers: data.headers || {},
    }));

    return JSON.stringify(streams);
}
