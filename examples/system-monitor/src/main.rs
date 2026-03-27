#[path = "generated.rs"]
#[allow(dead_code)]
mod generated;

use generated::*;
use vexil_runtime::BitWriter;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Html,
    routing::get,
    Router,
};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tokio::time::{interval, Duration};

static INDEX_HTML: &str = include_str!("../static/index.html");
static BUNDLE_JS: &str = include_str!("../static/bundle.js");

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/bundle.js", get(bundle_js))
        .route("/ws", get(ws_handler));

    let addr = "127.0.0.1:3000";
    println!("System Monitor running at http://{addr}");
    println!("Press Ctrl+C to stop.\n");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn bundle_js() -> ([(axum::http::header::HeaderName, &'static str); 1], &'static str) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        BUNDLE_JS,
    )
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    let mut sys = System::new_all();
    // Initial refresh to get baseline CPU readings
    sys.refresh_all();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut encoder = SystemSnapshotEncoder::new();
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tick.tick().await;
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage() as u8;
        let cpus = sys.cpus();
        let per_core: Vec<u8> = cpus.iter().map(|c| c.cpu_usage() as u8).collect();
        let cpu_count = cpus.len() as u8;

        let memory_used_mb = (sys.used_memory() / 1_048_576) as u32;
        let memory_total_mb = (sys.total_memory() / 1_048_576) as u32;

        let status = if cpu_usage >= 90 {
            CpuStatus::Critical
        } else if cpu_usage >= 70 {
            CpuStatus::Degraded
        } else {
            CpuStatus::Normal
        };

        let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let snapshot = SystemSnapshot {
            timestamp_ms,
            hostname,
            cpu_usage,
            cpu_count,
            per_core_usage: per_core,
            memory_used_mb,
            memory_total_mb,
            cpu_status: status,
        };

        let mut w = BitWriter::new();
        if encoder.pack(&snapshot, &mut w).is_err() {
            continue;
        }
        let bytes = w.finish();

        if socket.send(Message::Binary(bytes.into())).await.is_err() {
            break;
        }
    }
}
