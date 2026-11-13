//! `qi-cloud` 二进制入口：启动官方极简零健康库控制面与 ZDR 网关。

use std::path::PathBuf;
use std::sync::Arc;

use qi_cloud::db::ControlPlaneDb;
use qi_cloud::{AppState, ZdrUpstreamConfig, build_router};

#[tokio::main]
async fn main() {
    let bind_addr =
        std::env::var("QI_CLOUD_BIND").unwrap_or_else(|_| "127.0.0.1:8788".to_owned());
    let db_path =
        std::env::var("QI_CLOUD_DB").unwrap_or_else(|_| "./qi_control_plane.sqlite3".to_owned());

    let db = ControlPlaneDb::open_file(&PathBuf::from(&db_path))
        .expect("failed to open qi-cloud control plane database");

    let zdr_upstream = std::env::var("QI_ZDR_UPSTREAM_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|base_url| ZdrUpstreamConfig {
            base_url,
            api_key: std::env::var("QI_ZDR_UPSTREAM_KEY").unwrap_or_default(),
            model: std::env::var("QI_ZDR_UPSTREAM_MODEL")
                .unwrap_or_else(|_| "qwen3:latest".to_owned()),
        });

    let app = build_router(AppState {
        db: Arc::new(db),
        zdr_upstream,
    });

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind TCP listener");
    println!("qi-cloud listening on http://{bind_addr} (db={db_path}, zero-health-plaintext)");
    axum::serve(listener, app).await.expect("server error");
}
