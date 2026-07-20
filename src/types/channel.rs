use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::Config;
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "windows"))]
extern crate skim;

use crate::downloader::DownloadTrait;
use crate::{downloader::DownloadResponse, types::stream::Stream};
#[cfg(not(target_os = "windows"))]
use skim::prelude::*;

static COL1_WIDTH: AtomicUsize = AtomicUsize::new(40);
static COL2_WIDTH: AtomicUsize = AtomicUsize::new(22);

pub fn set_col_widths(c1: usize, c2: usize) {
    COL1_WIDTH.store(c1, Ordering::Relaxed);
    COL2_WIDTH.store(c2, Ordering::Relaxed);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Default, Clone)]
pub struct Channel {
    pub id: Option<String>,
    pub name: Option<String>,
    pub country: Option<String>,
    pub categories: Option<Vec<String>>,
    pub is_nsfw: Option<bool>,
    #[serde(default)]
    pub stream: Stream,
    #[serde(default)]
    pub current_programme: Option<String>,
}

impl DownloadTrait for Channel {
    fn download(url: &str) -> DownloadResponse {
        let resp = ureq::get(url)
            .set("Accept-Encoding", "gzip")
            .call()
            .expect("Could not connect to the internet. Check if your net is working");
        DownloadResponse {
            etag: resp.header("etag").unwrap().to_string(),
            json: io::read_to_string(resp.into_reader()).unwrap(),
        }
    }

    fn save(config: &Config) {
        let resp = Channel::download(config.channels_url.as_str());
        io::stdout().flush().unwrap();
        fs::write(config.channels_etag_path.as_path(), resp.etag).expect("Unable to write file");
        fs::write(config.channels_json_path.as_path(), resp.json).expect("Unable to write file");
    }

    fn load(config: &Config) -> Vec<Self> {
        let json = fs::read_to_string(config.channels_json_path.to_str().unwrap())
            .expect("Error reading data file");
        serde_json::from_str(json.as_str()).unwrap()
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let null_string = "Null".to_string();
        let name: &String = self.name.as_ref().unwrap_or(&null_string);
        let category: &String = self
            .categories
            .as_ref()
            .and_then(|c| c.first())
            .unwrap_or(&null_string);
        let c1 = COL1_WIDTH.load(Ordering::Relaxed);
        let c2 = COL2_WIDTH.load(Ordering::Relaxed);
        match &self.current_programme {
            Some(p) if !p.is_empty() => {
                write!(f, "{:<c1$}  |{:<c2$}  — \x1b[90m{}\x1b[0m", name, category, p)
            }
            _ => write!(f, "{:<c1$}  |{:<c2$}", name, category),
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl SkimItem for Channel {
    fn text(&self) -> Cow<str> {
        Cow::Owned(self.to_string().clone())
    }

    fn output(&self) -> Cow<str> {
        match &self.stream.url {
            Some(url) => Cow::Borrowed(url.as_str()),
            None => Cow::Borrowed(""),
        }
    }
}
