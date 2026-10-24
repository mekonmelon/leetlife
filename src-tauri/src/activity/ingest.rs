//! Loopback HTTP ingest: `POST /attempt` with an `ExternalAttempt` JSON body.
//! Intended for a browser extension that detects "Accepted" on leetcode.com.
//! Deliberately tiny (no HTTP framework): one request per connection, bounded
//! body, bound to 127.0.0.1 only.
use super::ExternalAttempt;
use crate::engine::Engine;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

const MAX_HEADER: usize = 16 * 1024;
const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, PartialEq)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

/// Parses one HTTP/1.1 request from `buf`. `None` means "need more bytes";
/// `Some(Err(status))` means reject.
pub fn parse_request(buf: &[u8]) -> Option<std::result::Result<Request, u16>> {
    let head_end = find(buf, b"\r\n\r\n")?;
    let head = std::str::from_utf8(&buf[..head_end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut req_line = lines.next()?.split_whitespace();
    let method = req_line.next()?.to_string();
    let path = req_line.next()?.split('?').next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }
    if content_length > MAX_BODY {
        return Some(Err(413));
    }
    let body_start = head_end + 4;
    if buf.len() < body_start + content_length {
        return None;
    }
    Some(Ok(Request { method, path, body: buf[body_start..body_start + content_length].to_vec() }))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn response(status: u16, body: &str) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

pub async fn route(engine: &Engine, req: Request) -> Vec<u8> {
    match (req.method.as_str(), req.path.as_str()) {
        ("OPTIONS", _) => response(204, ""),
        ("GET", "/health") => response(200, r#"{"ok":true,"app":"leetlife"}"#),
        ("GET", "/snapshot") => match engine.snapshot().await {
            Ok(s) => response(200, &serde_json::to_string(&*s).unwrap_or_else(|_| "{}".into())),
            Err(e) => response(500, &serde_json::json!({ "error": e.to_string() }).to_string()),
        },
        ("POST", "/attempt") => match serde_json::from_slice::<ExternalAttempt>(&req.body) {
            Ok(a) => match engine.ingest_external(vec![a]).await {
                Ok(n) => response(200, &serde_json::json!({ "imported": n }).to_string()),
                Err(e) => response(400, &serde_json::json!({ "error": e.to_string() }).to_string()),
            },
            Err(e) => response(400, &serde_json::json!({ "error": format!("bad body: {e}") }).to_string()),
        },
        _ => response(404, r#"{"error":"not found"}"#),
    }
}

async fn handle(engine: Arc<Engine>, mut stream: TcpStream) {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let parsed = loop {
        match stream.read(&mut chunk).await {
            Ok(0) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(_) => return,
        }
        if let Some(p) = parse_request(&buf) {
            break p;
        }
        if buf.len() > MAX_HEADER + MAX_BODY {
            let _ = stream.write_all(&response(413, r#"{"error":"too large"}"#)).await;
            return;
        }
    };
    let out = match parsed {
        Ok(req) => route(&engine, req).await,
        Err(status) => response(status, r#"{"error":"rejected"}"#),
    };
    let _ = stream.write_all(&out).await;
    let _ = stream.shutdown().await;
}

/// Serves until cancelled. Binding failure is logged, not fatal: the app works
/// without the extension endpoint.
pub async fn serve(engine: Arc<Engine>, port: u16, cancel: CancellationToken) {
    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(port, error = %e, "ingest endpoint unavailable");
            return;
        }
    };
    tracing::info!(port, "ingest endpoint listening");
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => { tokio::spawn(handle(engine.clone(), stream)); }
                Err(e) => tracing::warn!(error = %e, "accept failed"),
            }
        }
    }
    tracing::info!("ingest endpoint stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_post_with_body() {
        let raw = b"POST /attempt HTTP/1.1\r\nHost: x\r\nContent-Length: 2\r\n\r\n{}";
        let r = parse_request(raw).unwrap().unwrap();
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/attempt");
        assert_eq!(r.body, b"{}");
    }

    #[test]
    fn waits_for_full_body_and_rejects_huge() {
        assert!(parse_request(b"POST /attempt HTTP/1.1\r\nContent-Length: 5\r\n\r\n{}").is_none());
        assert!(parse_request(b"GET /x?y=1 HTTP/1.1\r\n").is_none());
        assert_eq!(parse_request(b"POST /a HTTP/1.1\r\nContent-Length: 999999\r\n\r\n").unwrap().unwrap_err(), 413);
        assert_eq!(parse_request(b"GET /health?x=1 HTTP/1.1\r\n\r\n").unwrap().unwrap().path, "/health");
    }

    #[test]
    fn response_has_cors_and_length() {
        let r = String::from_utf8(response(200, "{}")).unwrap();
        assert!(r.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(r.contains("Content-Length: 2\r\n"));
        assert!(r.contains("Access-Control-Allow-Origin: *"));
        assert!(r.ends_with("\r\n\r\n{}"));
    }
}
