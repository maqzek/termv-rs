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
    let needle = format!("{}=\"", key);
    let start = attrs.find(&needle)? + needle.len();
    let rest = &attrs[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}
