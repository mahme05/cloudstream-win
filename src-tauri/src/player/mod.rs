// player/mod.rs
// libmpv integration for video playback.
// 
// mpv is the best open-source media player — it handles HLS, DASH, m3u8,
// and basically any format/codec. This wraps the mpv C library in Rust.
//
// NOTE: mpv-rs requires libmpv to be installed on the system.
// On Windows, you'll need to ship libmpv.dll alongside your app.
// Download from: https://sourceforge.net/projects/mpv-player-windows/files/libmpv/

// For now this is a stub — we'll implement it in build step 5.
// The streaming command above calls this once it's implemented.

use anyhow::Result;
use std::collections::HashMap;

pub struct Player {
    // mpv: libmpv::Mpv   <- uncomment when you add mpv-rs to Cargo.toml
}

impl Player {
    pub fn new() -> Result<Self> {
        // let mpv = libmpv::Mpv::new()?;
        // mpv.set_property("volume", 70)?;
        // mpv.set_property("fullscreen", false)?;
        Ok(Self {})
    }
}

/// Play a stream URL in mpv
/// Headers are passed as mpv http-header-fields option
pub async fn play(url: &str, title: &str, headers: Option<HashMap<String, String>>) -> Result<()> {
    log::info!("Would play: {} ({})", title, url);
    // TODO: Implement with mpv-rs
    // let mpv = Player::new()?;
    // if let Some(hdrs) = headers {
    //     let hdr_string = hdrs.iter()
    //         .map(|(k, v)| format!("{}: {}", k, v))
    //         .collect::<Vec<_>>()
    //         .join(",");
    //     mpv.mpv.set_property("http-header-fields", hdr_string)?;
    // }
    // mpv.mpv.command("loadfile", &[url, "replace"])?;
    Ok(())
}
