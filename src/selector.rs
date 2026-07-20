use crate::types::Channel;

#[derive(Debug)]
pub enum UserSelectionResult {
    None,
}

#[cfg(target_os = "windows")]
pub fn get_user_selection(
    query: String,
    channels: Vec<Channel>,
) -> Result<Channel, UserSelectionResult> {
    use std::collections::HashMap;
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    let mut by_id: HashMap<String, Channel> = HashMap::new();
    let mut buffer = String::new();
    for channel in &channels {
        let id = channel.id.clone().unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        by_id.insert(id.clone(), channel.clone());
        buffer.push_str(&id);
        buffer.push('\t');
        buffer.push_str(&channel.to_string());
        buffer.push('\n');
    }

    let mut fzf = Command::new("fzf")
        .args([
            "--reverse",
            "--delimiter=\t",
            "--with-nth=2..",
            "--query",
            query.as_str(),
            "--header",
            "Select channel (press Escape to exit)",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("fzf command failed to start");

    let mut stdin = fzf.stdin.take().expect("Failed to take stdin");
    stdin
        .write_all(buffer.as_bytes())
        .expect("Failed to write to stdin");

    let output = fzf
        .wait_with_output()
        .expect("Failed to read stdout of fzf");

    let selection = String::from_utf8_lossy(&output.stdout).to_string();

    if selection.is_empty() {
        return Err(UserSelectionResult::None);
    }

    let first_line = selection.lines().next().unwrap_or("");
    let id = first_line.split('\t').next().unwrap_or("");
    match by_id.remove(id) {
        Some(c) => Ok(c),
        None => Err(UserSelectionResult::None),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn get_user_selection(
    query: String,
    channels: Vec<Channel>,
) -> Result<Channel, UserSelectionResult> {
    extern crate skim;

    use std::collections::HashMap;
    use skim::prelude::*;

    let mut by_id: HashMap<String, Channel> = HashMap::new();
    for channel in &channels {
        if let Some(id) = &channel.id {
            by_id.insert(id.clone(), channel.clone());
        }
    }

    let options = SkimOptionsBuilder::default()
        .query(Some(query))
        .height("100%".to_string())
        .layout("reverse".to_string())
        .header(Some("Select channel (press Escape to exit)".to_string()))
        .no_multi(true)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for channel in channels {
        let _ = tx_item.send(Arc::new(channel));
    }
    let output = Skim::run_with(&options, Some(rx_item)).unwrap();
    if output.final_event == Event::EvActAbort {
        panic!("Killed me");
    }
    let first_item = output.selected_items.first().unwrap();
    let id = first_item.output().to_string();
    match by_id.remove(&id) {
        Some(c) => Ok(c),
        None => Err(UserSelectionResult::None),
    }
}
