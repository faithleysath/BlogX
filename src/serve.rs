use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use tiny_http::{Header, Response, Server, StatusCode};

use crate::{
    build::{BuildOptions, build_site},
    config::Config,
};

#[derive(Debug, Clone, Default)]
pub struct ServeOverrides {
    pub host: Option<String>,
    pub port: Option<u16>,
}

pub fn serve_site(overrides: ServeOverrides, jobs: Option<usize>) -> Result<()> {
    let config = load_config_with_overrides(&overrides)?;
    build_site(
        &config,
        BuildOptions {
            clean: false,
            no_cache: false,
            jobs,
            profile: false,
        },
    )?;

    let host = config.serve.host.clone();
    let port = config.serve.port;
    let address = format!("{host}:{port}");

    let shutdown = Arc::new(AtomicBool::new(false));
    let server_shutdown = Arc::clone(&shutdown);
    let output = config.paths.output.clone();
    let server = Server::http(&address)
        .map_err(|err| anyhow!("failed to bind static server at {address}: {err}"))?;
    let server_handle = thread::spawn(move || {
        while !server_shutdown.load(Ordering::Relaxed) {
            match server.recv_timeout(Duration::from_millis(250)) {
                Ok(Some(request)) => {
                    if let Err(err) = respond_static(request, &output) {
                        eprintln!("serve error: {err:?}");
                    }
                }
                Ok(None) => {}
                Err(err) => eprintln!("serve error: {err}"),
            }
        }
    });

    println!(
        "serving {} at http://{address}/",
        config.paths.output.display()
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(250), move |result| {
        let _ = tx.send(result);
    })
    .context("failed to create file watcher")?;

    debouncer
        .watcher()
        .watch(&config.paths.content, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch {}", config.paths.content.display()))?;
    debouncer
        .watcher()
        .watch(&config.paths.theme, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch {}", config.paths.theme.display()))?;
    debouncer
        .watcher()
        .watch(std::path::Path::new("."), RecursiveMode::NonRecursive)
        .context("failed to watch project root for blogx.toml")?;

    loop {
        match rx.recv() {
            Ok(DebounceEventResult::Ok(events)) => {
                if events.is_empty() {
                    continue;
                }
                println!(
                    "detected {} change(s): {}, rebuilding",
                    events.len(),
                    summarize_events(&events)
                );
                let config = match load_config_with_overrides(&overrides) {
                    Ok(config) => config,
                    Err(err) => {
                        eprintln!("rebuild failed: {err:?}");
                        continue;
                    }
                };
                if let Err(err) = build_site(
                    &config,
                    BuildOptions {
                        clean: false,
                        no_cache: false,
                        jobs,
                        profile: false,
                    },
                ) {
                    eprintln!("rebuild failed: {err:?}");
                }
            }
            Ok(DebounceEventResult::Err(errors)) => {
                eprintln!("watch error: {errors}");
            }
            Err(_) => break,
        }
    }

    shutdown.store(true, Ordering::Relaxed);
    let _ = server_handle.join();
    Ok(())
}

fn load_config_with_overrides(overrides: &ServeOverrides) -> Result<Config> {
    let mut config = Config::load_from_current_dir()?;
    if let Some(host) = &overrides.host {
        config.serve.host = host.clone();
    }
    if let Some(port) = overrides.port {
        config.serve.port = port;
    }
    Ok(config)
}

fn summarize_events(events: &[notify_debouncer_mini::DebouncedEvent]) -> String {
    const LIMIT: usize = 5;
    let mut paths = events
        .iter()
        .map(|event| event.path.display().to_string())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let mut summary = paths
        .iter()
        .take(LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if paths.len() > LIMIT {
        summary.push_str(&format!(", ... +{} more", paths.len() - LIMIT));
    }
    summary
}

fn respond_static(request: tiny_http::Request, root: &Path) -> Result<()> {
    let method = request.method().as_str().to_string();
    if method != "GET" && method != "HEAD" {
        let response = Response::from_string("method not allowed")
            .with_status_code(StatusCode(405))
            .with_header(Header::from_bytes("Allow", "GET, HEAD").unwrap());
        request.respond(response)?;
        return Ok(());
    }

    let url = request.url().to_string();
    let path = resolve_request_path(root, &url);
    let Some(path) = path else {
        request.respond(Response::from_string("not found").with_status_code(StatusCode(404)))?;
        return Ok(());
    };

    let mime = mime_for_path(&path);
    let mut file = fs::File::open(&path)
        .with_context(|| format!("failed to open static file {}", path.display()))?;
    let mut body = Vec::new();
    file.read_to_end(&mut body)
        .with_context(|| format!("failed to read static file {}", path.display()))?;
    let response = if method == "HEAD" {
        Response::from_data(Vec::new())
    } else {
        Response::from_data(body)
    }
    .with_header(Header::from_bytes("Content-Type", mime).unwrap());
    request.respond(response)?;
    Ok(())
}

fn resolve_request_path(root: &Path, url: &str) -> Option<PathBuf> {
    let path_part = url.split(['?', '#']).next().unwrap_or("/");
    let mut relative = PathBuf::new();
    for segment in path_part.trim_start_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == ".." || segment.contains('\\') {
            return None;
        }
        relative.push(segment);
    }

    let mut candidate = root.join(&relative);
    if candidate.is_dir() {
        candidate = candidate.join("index.html");
    }
    if candidate.exists() && candidate.is_file() {
        return Some(candidate);
    }

    let html_fallback = root.join(relative.with_extension("html"));
    if html_fallback.exists() && html_fallback.is_file() {
        return Some(html_fallback);
    }

    None
}

fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}
