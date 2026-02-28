// @plugin-info {"id":"consumet-movies","name":"Movies & TV (TMDB)","version":"1.0.0","description":"Movies and TV shows via TMDB metadata + VidSrc streaming","author":"CloudStream Win","icon_url":null,"supported_types":["movie","show"],"is_builtin":false}

// Movies & TV Shows plugin
// - Uses TMDB (The Movie Database) API for metadata and search
// - Uses VidSrc for stream URLs (free, no API key needed)
//
// To get a free TMDB API key: https://www.themoviedb.org/settings/api
// Replace YOUR_TMDB_API_KEY below with your key.

const TMDB_API_KEY = "YOUR_TMDB_API_KEY";
const TMDB_BASE = "https://api.themoviedb.org/3";
const TMDB_IMG = "https://image.tmdb.org/t/p/w500";
const VIDSRC_BASE = "https://vidsrc.to/embed";

async function search(query) {
    const res = await fetch(
        `${TMDB_BASE}/search/multi?api_key=${TMDB_API_KEY}&query=${encodeURIComponent(query)}&include_adult=false`
    );
    if (!res.ok) return JSON.stringify([]);

    const data = await res.json();
    const results = (data.results || [])
        .filter(item => item.media_type === "movie" || item.media_type === "tv")
        .slice(0, 20)
        .map(item => {
            const isMovie = item.media_type === "movie";
            return {
                id: `${item.media_type}:${item.id}`,
                title: isMovie ? item.title : item.name,
                poster_url: item.poster_path ? `${TMDB_IMG}${item.poster_path}` : null,
                media_type: isMovie ? "movie" : "show",
                year: isMovie
                    ? (item.release_date ? parseInt(item.release_date) : null)
                    : (item.first_air_date ? parseInt(item.first_air_date) : null),
                rating: item.vote_average ? Math.round(item.vote_average * 10) / 10 : null,
                description: item.overview || null,
            };
        });

    return JSON.stringify(results);
}

async function getEpisodes(showId) {
    // showId format: "tv:12345"
    const [type, id] = showId.split(":");
    if (type !== "tv") return JSON.stringify([]);

    // Get show details to find seasons
    const res = await fetch(`${TMDB_BASE}/tv/${id}?api_key=${TMDB_API_KEY}`);
    if (!res.ok) return JSON.stringify([]);

    const show = await res.json();
    const episodes = [];

    // Load episodes from each season
    for (const season of (show.seasons || [])) {
        if (season.season_number === 0) continue; // skip specials

        const sRes = await fetch(
            `${TMDB_BASE}/tv/${id}/season/${season.season_number}?api_key=${TMDB_API_KEY}`
        );
        if (!sRes.ok) continue;

        const sData = await sRes.json();
        for (const ep of (sData.episodes || [])) {
            episodes.push({
                id: `tv:${id}:${season.season_number}:${ep.episode_number}`,
                title: ep.name || `Episode ${ep.episode_number}`,
                season: season.season_number,
                episode_number: ep.episode_number,
                thumbnail_url: ep.still_path ? `${TMDB_IMG}${ep.still_path}` : null,
                description: ep.overview || null,
            });
        }
    }

    return JSON.stringify(episodes);
}

async function getStreams(mediaId) {
    // mediaId format:
    //   movie: "movie:12345"
    //   episode: "tv:12345:1:3"  (show:season:episode)
    const parts = mediaId.split(":");

    let embedUrl;
    if (parts[0] === "movie") {
        embedUrl = `${VIDSRC_BASE}/movie/${parts[1]}`;
    } else if (parts[0] === "tv") {
        embedUrl = `${VIDSRC_BASE}/tv/${parts[1]}/${parts[2]}/${parts[3]}`;
    } else {
        return JSON.stringify([]);
    }

    // VidSrc embed URL is not a direct stream — return it as an embed
    // The player will open it in a webview
    return JSON.stringify([{
        url: embedUrl,
        quality: "auto",
        format: "embed",
        subtitles: [],
        headers: {},
    }]);
}
