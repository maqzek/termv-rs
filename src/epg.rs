use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;

use chrono::Utc;
use quick_xml::events::Event;
use quick_xml::Reader;

struct Programme {
    title: String,
}

pub struct EpgData {
    name_to_id: HashMap<String, String>,
    current: HashMap<String, Programme>,
}

impl EpgData {
    pub fn current_programme_for_name(&self, name: &str) -> Option<String> {
        let id = self.name_to_id.get(name)?;
        let programme = self.current.get(id)?;
        Some(programme.title.clone())
    }
}

fn exe_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("Could not resolve current_exe");
    exe.parent()
        .expect("Could not resolve exe parent dir")
        .to_path_buf()
}

fn source_path() -> PathBuf {
    exe_dir().join("epg_source.txt")
}

fn raw_path() -> PathBuf {
    exe_dir().join("epg_raw.xml")
}

fn etag_path() -> PathBuf {
    exe_dir().join("epg.etag")
}

pub fn read_saved_source() -> Option<String> {
    fs::read_to_string(source_path()).ok()
}

pub fn write_saved_source(s: &str) {
    fs::write(source_path(), s).expect("Unable to write epg_source.txt");
}

fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn parse_xmltv_time(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_str(s.trim(), "%Y%m%d%H%M%S %z")
        .ok()
        .map(|dt| dt.timestamp())
}

fn parse_channels<R: std::io::BufRead>(reader: R) -> Option<HashMap<String, String>> {
    let mut reader = Reader::from_reader(reader);
    let mut buf = Vec::new();
    let mut name_to_id: HashMap<String, String> = HashMap::new();
    let mut current_channel_id: Option<String> = None;
    let mut in_display_name = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"channel" => {
                    for attr in e.attributes().filter_map(Result::ok) {
                        if attr.key.as_ref() == b"id" {
                            current_channel_id =
                                Some(String::from_utf8_lossy(attr.value.as_ref()).to_string());
                        }
                    }
                }
                b"display-name" => {
                    if current_channel_id.is_some() {
                        in_display_name = true;
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"display-name" => in_display_name = false,
                b"channel" => current_channel_id = None,
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_display_name {
                    if let Some(ref id) = current_channel_id {
                        if let Ok(text) = e.unescape() {
                            let name = text.trim().to_string();
                            if !name.is_empty() {
                                name_to_id.insert(name, id.clone());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("Warning: EPG channel parse error: {}", e);
                return None;
            }
            _ => {}
        }
        buf.clear();
    }

    Some(name_to_id)
}

fn parse_programmes<R: std::io::BufRead>(
    reader: R,
    needed_ids: &HashSet<&String>,
    now_ts: i64,
) -> HashMap<String, Programme> {
    let mut reader = Reader::from_reader(reader);
    let mut buf = Vec::new();
    let mut current: HashMap<String, Programme> = HashMap::new();

    let mut in_programme = false;
    let mut prog_channel: Option<String> = None;
    let mut prog_start: Option<i64> = None;
    let mut prog_stop: Option<i64> = None;
    let mut in_title = false;
    let mut title_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"programme" => {
                    in_programme = true;
                    prog_channel = None;
                    prog_start = None;
                    prog_stop = None;
                    title_text.clear();
                    for attr in e.attributes().filter_map(Result::ok) {
                        match attr.key.as_ref() {
                            b"channel" => {
                                prog_channel =
                                    Some(String::from_utf8_lossy(attr.value.as_ref()).to_string());
                            }
                            b"start" => {
                                let s = String::from_utf8_lossy(attr.value.as_ref());
                                prog_start = parse_xmltv_time(&s);
                            }
                            b"stop" => {
                                let s = String::from_utf8_lossy(attr.value.as_ref());
                                prog_stop = parse_xmltv_time(&s);
                            }
                            _ => {}
                        }
                    }
                }
                b"title" => {
                    if in_programme {
                        in_title = true;
                        title_text.clear();
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"title" => in_title = false,
                b"programme" => {
                    if in_programme {
                        if let (Some(ref ch), Some(start), Some(stop)) =
                            (&prog_channel, prog_start, prog_stop)
                        {
                            if needed_ids.contains(ch) && start <= now_ts && now_ts < stop {
                                let title = title_text.trim().to_string();
                                if !title.is_empty() {
                                    current.insert(ch.clone(), Programme { title });
                                }
                            }
                        }
                    }
                    in_programme = false;
                    prog_channel = None;
                    prog_start = None;
                    prog_stop = None;
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_title {
                    if let Ok(text) = e.unescape() {
                        title_text.push_str(&text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("Warning: EPG programme parse error: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    current
}

fn parse_xml_file(path: &PathBuf) -> Option<EpgData> {
    let now_ts = Utc::now().timestamp();

    let file1 = fs::File::open(path).ok()?;
    let reader1 = BufReader::new(file1);
    let name_to_id = parse_channels(reader1)?;

    let needed_ids: HashSet<&String> = name_to_id.values().collect();

    let file2 = fs::File::open(path).ok()?;
    let reader2 = BufReader::new(file2);
    let current = parse_programmes(reader2, &needed_ids, now_ts);

    Some(EpgData {
        name_to_id,
        current,
    })
}

fn fetch_local(path: &str) -> Option<EpgData> {
    println!("Parsing EPG from {}...", path);
    let data = parse_xml_file(&PathBuf::from(path))?;
    println!(
        "EPG loaded: {} channels, {} current programmes",
        data.name_to_id.len(),
        data.current.len()
    );
    Some(data)
}

fn fetch_url(url: &str, force: bool) -> Option<EpgData> {
    let stored_etag = fs::read_to_string(etag_path()).ok();
    let raw_exists = raw_path().exists();

    let req = ureq::get(url).set("Accept-Encoding", "gzip");
    let req = if !force && raw_exists {
        if let Some(ref etag) = stored_etag {
            req.set("If-None-Match", etag)
        } else {
            req
        }
    } else {
        req
    };

    println!("Fetching EPG from {}...", url);
    let resp = match req.call() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: EPG fetch failed: {}", e);
            return None;
        }
    };

    if resp.status() == 304 {
        println!("EPG unchanged (304 Not Modified). Using cached file.");
        return parse_xml_file(&raw_path());
    }

    let etag = resp.header("etag").map(|s| s.to_string());
    let body = resp.into_string().ok()?;
    fs::write(raw_path(), &body).ok()?;
    if let Some(e) = etag {
        fs::write(etag_path(), e).ok()?;
    }

    let data = parse_xml_file(&raw_path())?;
    println!(
        "EPG loaded: {} channels, {} current programmes",
        data.name_to_id.len(),
        data.current.len()
    );
    Some(data)
}

pub fn fetch(source: &str, force: bool) -> Option<EpgData> {
    if is_url(source) {
        fetch_url(source, force)
    } else {
        fetch_local(source)
    }
}
