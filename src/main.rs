use args::Args;
use clap::Parser;
use types::{set_col_widths, Channel};

mod args;
mod downloader;
mod epg;
mod m3u;
mod selector;
mod source;
mod types;
mod utils;

fn main() {
    let args = Args::parse();

    let (c1, c2) = {
        let mut parts = args.cols.split(',');
        (
            parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(40),
            parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(22),
        )
    };
    set_col_widths(c1, c2);

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

    let epg_source = match args.epg_source {
        Some(s) => {
            epg::write_saved_source(&s);
            Some(s)
        }
        None => epg::read_saved_source(),
    };

    let epg_data = match epg_source {
        Some(s) => epg::fetch(&s, args.update),
        None => None,
    };

    let mut filtered_channels: Vec<Channel> = channels
        .into_iter()
        .filter(|c| c.stream.url.is_some())
        .collect();

    if let Some(ref epg) = epg_data {
        for channel in &mut filtered_channels {
            if let Some(ref name) = channel.name {
                if let Some(title) = epg.current_programme_for_name(name) {
                    channel.current_programme = Some(title);
                }
            }
        }
    }

    selector::get_user_selection(filtered_channels, args.mpv_flags.clone());
}
