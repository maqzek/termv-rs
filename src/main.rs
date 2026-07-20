use args::Args;
use clap::Parser;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use types::Channel;
use utils::open_mpv;

mod args;
mod downloader;
mod m3u;
mod selector;
mod source;
mod types;
mod utils;

fn main() {
    let args = Args::parse();

    let source = match args.m3u_source {
        Some(s) => {
            source::write_saved_source(&s);
            s
        }
        None => match source::read_saved_source() {
            Some(s) => s,
            None => {
                eprintln!("No playlist specified. Usage: termv-rs <path-or-url>");
                return;
            }
        },
    };

    let channels = source::fetch(&source, args.update);

    let running = Arc::new(AtomicBool::new(true));

    let running_clone = Arc::clone(&running);
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    while running.load(Ordering::SeqCst) {
        let channel: Channel =
            match selector::get_user_selection("".to_string(), channels.clone()) {
                Ok(c) => c,
                Err(_e) => break,
            };

        if let Some(stream_url) = channel.stream.url.as_ref() {
            open_mpv(stream_url.to_string(), args.mpv_flags.clone());
        } else {
            eprintln!("Error: No stream found for selected channel.");
        }

        thread::sleep(Duration::from_millis(100));
    }
}
