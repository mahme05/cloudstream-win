// @plugin-info {"id":"consumet-zoro","name":"Hianime (Zoro)","version":"1.0.0","description":"HD anime streaming via Hianime, powered by Consumet API","author":"CloudStream Win","icon_url":null,"supported_types":["anime"],"is_builtin":false}

// Hianime (formerly Zoro) plugin using Consumet API
// Has both SUB and DUB, usually 1080p quality.
// Requires a Consumet API instance — see gogoanime.js for setup instructions.

const BASE_URL = "https://api.consumet.org";
const PROVIDER = "zoro";

async function search(query) {
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/${encodeURIComponent(query)}`);
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const results = (data.results || []).map(item => ({
        id: item.id,
        title: item.title,
        poster_url: item.image || null,
        media_type: "anime",
        year: null,
        rating: null,
        description: item.type ? `[${item.type}]` : null,
    }));

    return JSON.stringify(results);
}

async function getEpisodes(showId) {
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/info?id=${encodeURIComponent(showId)}`);
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
    const res = await fetch(`${BASE_URL}/anime/${PROVIDER}/watch?episodeId=${encodeURIComponent(mediaId)}`);
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const streams = (data.sources || []).map(src => ({
        url: src.url,
        quality: src.quality || "auto",
        format: src.url.includes(".m3u8") ? "hls" : "mp4",
        subtitles: (data.subtitles || [])
            .filter(sub => sub.lang !== "Thumbnails")
            .map(sub => ({
                url: sub.url,
                language: sub.lang,
                label: sub.lang,
            })),
        headers: { "Referer": "https://hianime.to" },
    }));

    return JSON.stringify(streams);
}
