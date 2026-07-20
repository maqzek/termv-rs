use args::Args;
use clap::Parser;
use types::Channel;

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

    let filtered_channels: Vec<Channel> = channels
        .into_iter()
        .filter(|c| c.stream.url.is_some())
        .collect();

    selector::get_user_selection(filtered_channels, args.mpv_flags.clone());
}
