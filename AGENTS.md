# AGENTS.md

Single-crate Rust CLI (`termv-rs`) — an IPTV channel browser that shells out to `mpv`. No workspace, no tests, no lint config.

## Commands

- `cargo build` — dev build. **Do NOT run locally on this machine** (see "No local toolchain" below); push to CI instead.
- `cargo run -- <path-or-url>` — load an m3u playlist (local path or http(s) URL), persist it beside the exe for future runs
- `cargo run` — re-use the persisted playlist source
- `cargo run -- -u` — force-refresh the playlist (ignore ETag/mtime) then browse
- `cargo test` — runs but there are **no tests** in the repo; CI still calls it
- `cargo build --release --locked` — release build (CI's release artifacts use this). Release profile already sets `strip = true` and `lto = true`.

No `rustfmt.toml` / `clippy.toml` exists; do not invent lint commands.

## No local toolchain

This machine has only the default `stable-x86_64-pc-windows-msvc` rustup toolchain and **no C/C++ linker** (`link.exe` missing, `gcc.exe` missing, `clang.exe` missing). `cargo build` and `cargo check` both fail because proc-macro build scripts require a linker. **Do not attempt local compilation and do not install MSVC Build Tools / MinGW / extra rustup toolchains** — the user keeps this system clean of dev dependencies and compiles exclusively via GitHub Actions. Verify changes by code review only. (Prior sessions attempted `winget install Microsoft.VisualStudio.2022.BuildTools` and `rustup toolchain install stable-x86_64-pc-windows-gnu` — both were uninstalled at the user's request; do not repeat.)

## Runtime dependencies (non-obvious)

- `mpv` must be on `PATH` on every platform (`utils::open_mpv` shells out to it).
- `fzf` must be on `PATH` on **Windows only**. On non-Windows the `skim` crate is used instead (it is a target-gated dependency in `Cargo.toml`).
- `utils::has_dependencies()` checks these but is **never called** from `main.rs` — missing deps surface as a panic at `mpv`/`fzf` spawn time, not a clean error.

## Playlist / m3u flow (current)

`main.rs` no longer touches iptv-org at all. The flow is:

1. Resolve source: positional `m3u_source` arg → overwrite `source.txt` beside the exe and use it. No arg → read `source.txt`. Neither → `eprintln!("No playlist specified...")` + `return`.
2. `source::fetch(source, force)`:
   - URL (`http://`/`https://` prefix): GET via `ureq` with `If-None-Match` from `playlist.etag`; on 304 → load `playlist.json` cache. Else parse body, save `playlist.json` + new etag.
   - Local path: compare file mtime (in seconds since UNIX_EPOCH) with `playlist.mtime`; if unchanged and cache exists → load cache. Else parse, save cache + mtime.
   - `force=true` (set by `-u`/`--update`) skips the unchanged check.
3. `m3u::parse(content) -> Vec<Channel>`: walks lines; on `#EXTINF` extracts `tvg-id` → `channel.id`, `group-title` → `categories: vec![...]`, name after last comma → `name`. Next non-`#` line → `stream.url`. **Missing `tvg-id` synthesizes an id** `_synthetic_<N>` so the channel remains selectable (skim/fzf both key on `channel.id`). Does not skip any channels.
4. Selector returns a `Channel`; `main.rs` reads `channel.stream.url` directly. No `stream_map` indirection anymore.

All persisted files live beside `current_exe()`: `source.txt`, `playlist.json`, `playlist.etag`, `playlist.mtime`. This is intentional for portability (no `AppDirs` involvement in the new flow).

## Dead iptv-org code (preserved intentionally)

`src/downloader.rs`, `src/types/channel.rs`'s `DownloadTrait` impl, `src/types/stream.rs`, and `src/types/config.rs` still reference the old iptv-org `channels.json`/`streams.json` flow. **Nothing in `main.rs` calls them.** The user kept them in case another JSON source is wired up later. Do not delete without explicit instruction. Args `channels_url`, `streams_url`, `auto_update`, `fullscreen`, `env_fullscreen` in `args.rs` are likewise parsed-but-unused.

## Platform-conditional code

`src/selector.rs` and `src/utils.rs` use `#[cfg(target_os = "windows")]` to switch implementations. Both `selector::get_user_selection` variants now share the signature `fn(query: String, channels: Vec<Channel>) -> Result<Channel, UserSelectionResult>` and both return a `Channel` directly. Windows builds fzf input as `<id>\t<display>` with `--delimiter=\t --with-nth=2..`, then looks up the `Channel` by id from the passed-in `Vec<Channel>`. The Windows target should now compile (was previously broken by a signature mismatch).

## CLI args vs. what is actually wired up

`src/args.rs` declares env-overridable args via clap (`TERMV_AUTO_UPDATE`, `TERMV_FULL_SCREEN`, `TERMV_DEFAULT_MPV_FLAGS`, `TERMV_CHANNELS_URL`, `TERMV_STREAMS_URL`) plus `m3u_source`, `fullscreen`, `update`. **`main.rs` only consumes `args.m3u_source`, `args.update`, and `args.mpv_flags`.** The other fields — including `channels_url`, `streams_url`, `fullscreen`, `auto_update` — are parsed but never read (see "Dead iptv-org code" above). Do not assume an env var or flag takes effect without tracing it into `main.rs`.

The README is stale on this point: it advertises `TERMV_API_URL` and a single API URL. Real env vars are `TERMV_CHANNELS_URL` and `TERMV_STREAMS_URL` (and even those are currently unused at runtime).

## CI

- `.github/workflows/rust.yml` — ubuntu: `cargo build --verbose` + `cargo test --verbose` on push/PR to `main`.
- `.github/workflows/windows.yml` — windows: `cargo build --release --locked`, zips the exe + README, uploads as a workflow artifact on every run, and attaches the zip to a release. Triggers: push/PR to `main`, `release: created`, and `workflow_dispatch`.
  - On push to `main` or `workflow_dispatch`: attaches to the rolling `nightly` pre-release (tag `nightly`, `prerelease: true`, `make_latest: false`). The `nightly` tag is auto-created on first run and force-moved to the latest commit on each subsequent run; assets with the same filename are overwritten by `softprops/action-gh-release@v2`.
  - On `release: created`: attaches to that specific release using `github.ref_name` as `tag_name`, `prerelease: false`, `make_latest: true` — so manually-cut versioned releases become the "Latest" release.
  - On `pull_request`: build-only (no release write).
  - Requires the repo setting **Settings → Actions → General → Workflow permissions** to be "Read and write permissions", plus the in-workflow `permissions: contents: write` block (defense in depth). Without these, `softprops/action-gh-release` fails with `Resource not accessible by integration`.

The previous `.github/workflows/release.yml` (linux musl via `rust-build.action`) was deleted: its bundled Rust (1.76 in v1.4.5, the latest tag) cannot parse `Cargo.lock` v4 (needs 1.78+). Windows-only releases for now.

Push to CI to verify any change to `selector.rs`, `main.rs`, `m3u.rs`, or `source.rs` — local `cargo build`/`cargo check` is not possible on this machine.
