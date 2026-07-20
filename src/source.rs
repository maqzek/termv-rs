use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::m3u;
use crate::types::Channel;

fn exe_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("Could not resolve current_exe");
    exe.parent()
        .expect("Could not resolve exe parent dir")
        .to_path_buf()
}

fn source_path() -> PathBuf {
    exe_dir().join("source.txt")
}

fn cache_path() -> PathBuf {
    exe_dir().join("playlist.json")
}

fn etag_path() -> PathBuf {
    exe_dir().join("playlist.etag")
}

fn mtime_path() -> PathBuf {
    exe_dir().join("playlist.mtime")
}

pub fn read_saved_source() -> Option<String> {
    fs::read_to_string(source_path()).ok()
}

pub fn write_saved_source(s: &str) {
    fs::write(source_path(), s).expect("Unable to write source.txt");
}

fn load_cache() -> Vec<Channel> {
    let json = fs::read_to_string(cache_path()).expect("Error reading cached playlist");
    serde_json::from_str(json.as_str()).expect("Error parsing cached playlist")
}

fn save_cache(channels: &[Channel]) {
    let json = serde_json::to_string(channels).expect("Error serializing playlist");
    fs::write(cache_path(), json).expect("Unable to write playlist.json");
}

fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn fetch_local(path: &str, force: bool) -> Vec<Channel> {
    let metadata = fs::metadata(path).expect("Could not read file metadata");
    let mtime = metadata
        .modified()
        .expect("Could not read file modified time");

    if !force {
        if let Ok(stored) = fs::read_to_string(mtime_path()) {
            if let Ok(stored_secs) = stored.parse::<u64>() {
                let mtime_secs = mtime
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if stored_secs == mtime_secs && cache_path().exists() {
                    let channels = load_cache();
                    println!(
                        "Playlist unchanged. Loaded {} channels from cache.",
                        channels.len()
                    );
                    return channels;
                }
            }
        }
    }

    println!("Parsing {}...", path);
    let content = fs::read_to_string(path).expect("Error reading m3u file");
    let channels = m3u::parse(&content);
    save_cache(&channels);
    let mtime_secs = mtime
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fs::write(mtime_path(), mtime_secs.to_string()).expect("Unable to write playlist.mtime");
    println!("Loaded {} channels.", channels.len());
    channels
}

fn fetch_url(url: &str, force: bool) -> Vec<Channel> {
    let stored_etag = fs::read_to_string(etag_path()).ok();

    let req = ureq::get(url).set("Accept-Encoding", "gzip");
    let req = if !force {
        if let Some(ref etag) = stored_etag {
            req.set("If-None-Match", etag)
        } else {
            req
        }
    } else {
        req
    };

    println!("Fetching {}...", url);
    let resp = req
        .call()
        .expect("Could not connect to the internet. Check if your net is working");

    if resp.status() == 304 {
        let channels = load_cache();
        println!(
            "Playlist unchanged (304 Not Modified). Loaded {} channels from cache.",
            channels.len()
        );
        return channels;
    }

    let etag = resp.header("etag").map(|s| s.to_string());
    let body = resp.into_string().unwrap();
    let channels = m3u::parse(&body);
    save_cache(&channels);
    if let Some(e) = etag {
        fs::write(etag_path(), e).expect("Unable to write playlist.etag");
    }
    println!("Loaded {} channels.", channels.len());
    channels
}

pub fn fetch(source: &str, force: bool) -> Vec<Channel> {
    if is_url(source) {
        fetch_url(source, force)
    } else {
        fetch_local(source, force)
    }
}
