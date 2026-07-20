use crate::types::{Channel, Stream};

pub fn parse(content: &str) -> Vec<Channel> {
    let mut channels = Vec::new();
    let mut counter: usize = 0;
    let mut pending: Option<(Option<String>, Option<String>, Option<Vec<String>>)> = None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if let Some(info) = line.strip_prefix("#EXTINF") {
                let (attrs, name) = split_extinf(info);
                let id = attr_value(attrs, "tvg-id");
                let group = attr_value(attrs, "group-title");
                let categories = group.map(|g| vec![g]);
                pending = Some((id, name, categories));
            }
            continue;
        }

        let (id_opt, name_opt, categories_opt) = pending.take().unwrap_or((None, None, None));
        let id = id_opt.unwrap_or_else(|| {
            counter += 1;
            format!("_synthetic_{}", counter)
        });
        let channel = Channel {
            id: Some(id),
            name: name_opt,
            country: None,
            categories: categories_opt,
            is_nsfw: None,
            stream: Stream {
                id: None,
                url: Some(line.to_string()),
            },
        };
        channels.push(channel);
    }

    channels
}

fn split_extinf(info: &str) -> (&str, Option<String>) {
    let info = info.trim_start_matches(':');
    if let Some(comma) = info.rfind(',') {
        let attrs = &info[..comma];
        let name = info[comma + 1..].trim().to_string();
        if name.is_empty() {
            (attrs, None)
        } else {
            (attrs, Some(name))
        }
    } else {
        (info, None)
    }
}

fn attr_value(attrs: &str, key: &str) -> Option<String> {
    let mut search = 0;
    while let Some(rel) = attrs[search..].find(key) {
        let abs = search + rel;
        let before_ok = abs == 0 || attrs.as_bytes()[abs - 1] == b' ';
        let after = abs + key.len();
        let after_ok = attrs.as_bytes().get(after) == Some(&b'=');
        if before_ok && after_ok {
            let rest = &attrs[after + 1..];
            let value = extract_value(rest);
            if value.is_empty() {
                return None;
            }
            return Some(value);
        }
        search = abs + key.len();
    }
    None
}

fn extract_value(rest: &str) -> String {
    let bytes = rest.as_bytes();
    match bytes.first() {
        Some(&b'"') => {
            let inner = &rest[1..];
            let end = inner.find('"').unwrap_or(inner.len());
            inner[..end].to_string()
        }
        Some(&b'\'') => {
            let inner = &rest[1..];
            let end = inner.find('\'').unwrap_or(inner.len());
            inner[..end].to_string()
        }
        _ => {
            let end = rest.find(' ').unwrap_or(rest.len());
            rest[..end].to_string()
        }
    }
}
