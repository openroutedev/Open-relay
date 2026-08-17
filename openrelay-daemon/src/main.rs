use axum::{routing::{get, post}, Json, Router};
use openrelay_crypto::identity::NodeIdentity;
use openrelay_label::{format::PackageLabelData, pdf::PackingSlipGenerator};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    identity: Arc<NodeIdentity>,
}

#[derive(Deserialize)]
struct CreatePackingSlipRequest {
    commitment_hex: String,
    seal_serial: String,
    next_staging_point: String,
    deadline: u64,
}

#[derive(Serialize)]
struct CreatePackingSlipResponse {
    pdf_base64: String,
}

#[tokio::main]
async fn main() {
    println!("=== Starting OpenRelay v0.3 Node Daemon ===");

    let node_identity = NodeIdentity::generate();
    println!("[+] Node ID: {}", node_identity.node_id());

    let state = AppState { identity: Arc::new(node_identity) };

    let app = Router::new()
        .route("/v1/status", get(get_status))
        .route("/v1/label/pdf", post(generate_label_pdf))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("[+] Daemon listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ACTIVE_ONLINE",
        "version": "0.3.0"
    }))
}

async fn generate_label_pdf(
    Json(payload): Json<CreatePackingSlipRequest>,
) -> Result<Json<CreatePackingSlipResponse>, String> {
    let label_data = PackageLabelData {
        commitment_hex: payload.commitment_hex,
        seal_serial: payload.seal_serial,
        next_staging_point: payload.next_staging_point,
        deadline_timestamp: payload.deadline,
    };

    let pdf_bytes = PackingSlipGenerator::generate_pdf(&label_data)?;

    Ok(Json(CreatePackingSlipResponse {
        pdf_base64: data_encoding::BASE64.encode(&pdf_bytes),
    }))
}
