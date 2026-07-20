use crate::types::Channel;

#[cfg(target_os = "windows")]
pub fn get_user_selection(channels: Vec<Channel>, mpv_flags: String) {
    use std::io::Write;
    use std::process::Command;
    use std::process::Stdio;

    let mut buffer = String::new();
    for channel in &channels {
        let url = channel.stream.url.clone().unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        buffer.push_str(&url);
        buffer.push('\t');
        buffer.push_str(&channel.to_string());
        buffer.push('\n');
    }

    let bind = format!(
        "enter:execute(echo \"Fetching channel, please wait...\" && mpv {} {{1}})",
        mpv_flags
    );

    let mut fzf = Command::new("fzf")
        .args([
            "--reverse",
            "--delimiter=\t",
            "--with-nth=2..",
            "--bind",
            bind.as_str(),
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

    fzf.wait_with_output().expect("Failed to wait for fzf");
}

#[cfg(not(target_os = "windows"))]
pub fn get_user_selection(channels: Vec<Channel>, mpv_flags: String) {
    extern crate skim;

    use skim::prelude::*;

    let bind = format!(
        "enter:execute(echo 'Fetching channel, please wait...' && mpv {} {{}})",
        mpv_flags
    );

    let options = SkimOptionsBuilder::default()
        .height("100%".to_string())
        .layout("reverse".to_string())
        .header(Some("Select channel (press Escape to exit)".to_string()))
        .no_multi(true)
        .bind(vec![bind])
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for channel in channels {
        let _ = tx_item.send(Arc::new(channel));
    }
    let _output = Skim::run_with(&options, Some(rx_item)).unwrap();
}
