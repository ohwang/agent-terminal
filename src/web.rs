use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

const INDEX_HTML: &str = include_str!("web/index.html");
const PLAYER_HTML: &str = include_str!("web/player.html");
const STYLE_CSS: &str = include_str!("web/style.css");
const PLAYER_JS: &str = include_str!("web/player.js");

const DEFAULT_RECORDINGS_DIR: &str = ".agent-terminal/recordings";

fn default_recordings_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home.join(DEFAULT_RECORDINGS_DIR))
}

fn recordings_dir(dir: Option<&str>) -> Result<PathBuf, String> {
    match dir {
        Some(d) => Ok(PathBuf::from(d)),
        None => default_recordings_dir(),
    }
}

#[derive(Deserialize, serde::Serialize)]
struct RecordingMeta {
    session: String,
    group: String,
    label: String,
    started_at: String,
    stopped_at: Option<String>,
    cols: u16,
    rows: u16,
    frame_count: u64,
    duration_ms: u64,
    // Added by the API, not stored on disk
    #[serde(skip_deserializing, default)]
    _dir_name: String,
    #[serde(skip_deserializing, default)]
    _group_dir: String,
}

/// Scan the recordings directory and return metadata for all recordings.
fn list_recordings(base: &PathBuf) -> Vec<RecordingMeta> {
    let mut recordings = Vec::new();
    if !base.exists() {
        return recordings;
    }
    let Ok(entries) = fs::read_dir(base) else {
        return recordings;
    };
    for group_entry in entries.flatten() {
        if !group_entry.path().is_dir() {
            continue;
        }
        let group_name = group_entry.file_name().to_string_lossy().to_string();
        let Ok(rec_entries) = fs::read_dir(group_entry.path()) else {
            continue;
        };
        for rec_entry in rec_entries.flatten() {
            let meta_path = rec_entry.path().join("meta.json");
            if let Ok(meta_str) = fs::read_to_string(&meta_path) {
                if let Ok(mut meta) = serde_json::from_str::<RecordingMeta>(&meta_str) {
                    meta._dir_name = rec_entry.file_name().to_string_lossy().to_string();
                    meta._group_dir = group_name.clone();
                    recordings.push(meta);
                }
            }
        }
    }
    recordings.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    recordings
}

/// Serve the web viewer.
pub fn serve(dir: Option<&str>, port: u16) -> Result<(), String> {
    let base = recordings_dir(dir)?;
    let addr = format!("0.0.0.0:{}", port);

    let server = tiny_http::Server::http(&addr)
        .map_err(|e| format!("Failed to start server on {}: {}", addr, e))?;

    println!("Web viewer running at http://localhost:{}", port);
    println!("Recordings dir: {}", base.display());
    println!("Press Ctrl+C to stop");

    loop {
        let request = match server.recv() {
            Ok(req) => req,
            Err(_) => break,
        };

        let url = request.url().to_string();

        let response = route(&url, &base);
        let _ = request.respond(response);
    }

    Ok(())
}

fn route(url: &str, base: &PathBuf) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    // Strip query string for path matching
    let path = url.split('?').next().unwrap_or(url);

    match path {
        "/" => serve_html(INDEX_HTML),
        "/player" => serve_html(PLAYER_HTML),
        "/style.css" => serve_content(STYLE_CSS, "text/css"),
        "/player.js" => serve_content(PLAYER_JS, "application/javascript"),
        "/api/recordings" => {
            let recordings = list_recordings(base);
            let json = serde_json::to_string(&recordings).unwrap_or_else(|_| "[]".to_string());
            serve_content(&json, "application/json")
        }
        _ if path.starts_with("/api/recording/") => serve_recording_file(path, base),
        _ => serve_not_found(),
    }
}

/// Serve a recording sub-file: /api/recording/{group}/{name}/{file}
fn serve_recording_file(
    path: &str,
    base: &std::path::Path,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    // Parse: /api/recording/{group}/{name}/{file_type}
    let parts: Vec<&str> = path
        .trim_start_matches("/api/recording/")
        .splitn(3, '/')
        .collect();
    if parts.len() < 3 {
        return serve_not_found();
    }

    let group = parts[0];
    let name = parts[1];
    let file_type = parts[2];

    let filename = match file_type {
        "cast" => "recording.cast",
        "frames" => "frames.jsonl",
        "actions" => "actions.jsonl",
        "meta" => "meta.json",
        _ => return serve_not_found(),
    };

    let file_path = base.join(group).join(name).join(filename);

    // Security: ensure the resolved path is under base
    match file_path.canonicalize() {
        Ok(canonical) => {
            if let Ok(base_canonical) = base.canonicalize() {
                if !canonical.starts_with(&base_canonical) {
                    return serve_not_found();
                }
            }
        }
        Err(_) => {
            // File doesn't exist — serve empty for jsonl files, 404 otherwise
            if filename.ends_with(".jsonl") {
                return serve_content("", "application/x-ndjson");
            }
            return serve_not_found();
        }
    }

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            let content_type = match file_type {
                "cast" => "application/x-asciicast",
                "frames" | "actions" => "application/x-ndjson",
                "meta" => "application/json",
                _ => "text/plain",
            };
            serve_content(&content, content_type)
        }
        Err(_) => {
            if filename.ends_with(".jsonl") {
                serve_content("", "application/x-ndjson")
            } else {
                serve_not_found()
            }
        }
    }
}

fn serve_html(content: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    serve_content(content, "text/html; charset=utf-8")
}

fn serve_content(
    content: &str,
    content_type: &str,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let data = content.as_bytes().to_vec();
    let len = data.len();
    tiny_http::Response::new(
        tiny_http::StatusCode(200),
        vec![
            tiny_http::Header::from_bytes("Content-Type", content_type).unwrap(),
            tiny_http::Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
        ],
        std::io::Cursor::new(data),
        Some(len),
        None,
    )
}

fn serve_not_found() -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let body = b"Not Found".to_vec();
    let len = body.len();
    tiny_http::Response::new(
        tiny_http::StatusCode(404),
        vec![tiny_http::Header::from_bytes("Content-Type", "text/plain").unwrap()],
        std::io::Cursor::new(body),
        Some(len),
        None,
    )
}
