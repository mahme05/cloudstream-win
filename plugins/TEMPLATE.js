// @plugin-info {"id":"my-plugin-template","name":"Plugin Template","version":"1.0.0","description":"A starter template for writing your own plugin","author":"Your Name","icon_url":null,"supported_types":["anime","movie","show"],"is_builtin":false}

// ═══════════════════════════════════════════════════════
// CloudStream Win — Plugin Template
// ═══════════════════════════════════════════════════════
//
// HOW TO WRITE A PLUGIN:
//
// 1. Copy this file and rename it (e.g. my-site.js)
// 2. Edit the @plugin-info comment at the top
// 3. Implement the 3 functions: search(), getEpisodes(), getStreams()
// 4. Each function MUST return JSON.stringify(array)
// 5. Install it in the app via Plugins → Install → Browse
//
// AVAILABLE GLOBALS:
//   fetch(url, options?)  — make HTTP requests (same as browser fetch)
//   console.log(...)      — logging (goes to app console)
//   JSON, Math, Date      — standard JS built-ins
//
// FETCH EXAMPLE:
//   const res = await fetch("https://example.com/api/search?q=naruto");
//   const data = await res.json();   // parse JSON response
//   const html = await res.text();   // get raw HTML for scraping
//
// POST REQUEST:
//   const res = await fetch("https://example.com/api", {
//     method: "POST",
//     headers: { "Content-Type": "application/json" },
//     body: JSON.stringify({ query: "naruto" })
//   });
//
// ═══════════════════════════════════════════════════════

/**
 * Search for content.
 * @param {string} query - User's search term
 * @returns {string} JSON.stringify of SearchResult[]
 */
async function search(query) {
    // TODO: Make a request to your site's search endpoint
    // const res = await fetch(`https://example.com/search?q=${encodeURIComponent(query)}`);
    // const data = await res.json();

    // Return an array of search results
    const results = [
        // Each result must have these fields:
        {
            id: "unique-id-123",          // Used in getEpisodes() and getStreams()
            title: "Example Title",
            poster_url: "https://example.com/poster.jpg",  // or null
            media_type: "anime",           // "anime" | "movie" | "show"
            year: 2024,                    // or null
            rating: 8.5,                   // or null
            description: "A great show",   // or null
        }
    ];

    // IMPORTANT: Always return JSON.stringify, not a plain object
    return JSON.stringify(results);
}

/**
 * Get episodes for a show/anime.
 * Not needed for movies — return [] for those.
 * @param {string} showId - The id from a SearchResult
 * @returns {string} JSON.stringify of Episode[]
 */
async function getEpisodes(showId) {
    // TODO: Fetch episode list from your site
    // const res = await fetch(`https://example.com/show/${showId}/episodes`);

    const episodes = [
        {
            id: "ep-id-s1e1",          // Used in getStreams()
            title: "Episode 1",
            season: 1,                  // or null
            episode_number: 1,
            thumbnail_url: null,
            description: "The beginning",
        }
    ];

    return JSON.stringify(episodes);
}

/**
 * Get stream URLs for a movie or episode.
 * @param {string} mediaId - For movies: the SearchResult id.
 *                           For episodes: the Episode id.
 * @returns {string} JSON.stringify of StreamSource[]
 */
async function getStreams(mediaId) {
    // TODO: Fetch the actual video stream URL from your site
    // const res = await fetch(`https://example.com/watch/${mediaId}`);
    // Parse the response to find the .m3u8 or .mp4 URL

    const streams = [
        {
            url: "https://example.com/stream.m3u8",  // Direct video URL
            quality: "1080p",                          // "1080p" | "720p" | "480p" | "auto"
            format: "hls",                             // "hls" | "mp4" | "dash" | "embed"
            subtitles: [
                // Optional subtitle tracks
                {
                    url: "https://example.com/subs/en.vtt",
                    language: "en",
                    label: "English",
                }
            ],
            headers: {
                // Optional HTTP headers (some sites need Referer)
                "Referer": "https://example.com",
            },
        }
    ];

    return JSON.stringify(streams);
}

// ═══════════════════════════════════════════════════════
// TIPS FOR HTML SCRAPING
// (when the site has no API)
// ═══════════════════════════════════════════════════════
//
// const res = await fetch("https://example.com/search?q=naruto");
// const html = await res.text();
//
// Basic regex scraping:
// const titles = [...html.matchAll(/<h3 class="title">(.*?)<\/h3>/g)]
//                    .map(m => m[1]);
//
// For more complex scraping, copy the network requests from browser DevTools
// and replicate them with fetch().
// ═══════════════════════════════════════════════════════
