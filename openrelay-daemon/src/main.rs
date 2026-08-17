use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use openrelay_crypto::identity::NodeIdentity;
use openrelay_label::{format::PackageLabelData, pdf::PackingSlipGenerator};
use openrelay_protocol::{PaymentSpec, PickupRequest, RequestStatus, ShipmentState, StorageEngine};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    identity: Arc<NodeIdentity>,
    storage: Arc<StorageEngine>,
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

#[derive(Deserialize)]
struct CreateShipmentRequest {
    commitment_hex: String,
    seal_serial: String,
}

#[derive(Deserialize)]
struct CreatePickupOrderPayload {
    pickup_location: String,
    item_description: String,
    dropoff_location: String,
    payment_spec: PaymentSpec,
}

#[tokio::main]
async fn main() {
    println!("=== Starting OpenRelay v0.3 Node Daemon ===");

    let node_identity = NodeIdentity::generate();
    println!("[+] Node ID: {}", node_identity.node_id());

    let storage = StorageEngine::in_memory().await.expect("Failed to init storage");
    println!("[+] SQLite storage initialized");

    let state = AppState {
        identity: Arc::new(node_identity),
        storage: Arc::new(storage),
    };

    let app = Router::new()
        .route("/v1/status", get(get_status))
        .route("/v1/label/pdf", post(generate_label_pdf))
        .route("/v1/shipments", post(create_shipment))
        .route("/v1/requests", post(create_pickup_request))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("[+] Daemon listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ACTIVE_ONLINE",
        "version": "0.3.0",
        "node_id": state.identity.node_id()
    }))
}

async fn create_shipment(
    State(state): State<AppState>,
    Json(payload): Json<CreateShipmentRequest>,
) -> Result<Json<serde_json::Value>, String> {
    state
        .storage
        .save_shipment(&payload.commitment_hex, ShipmentState::Created, &payload.seal_serial)
        .await?;

    Ok(Json(serde_json::json!({
        "status": "SUCCESS",
        "commitment": payload.commitment_hex,
        "state": "CREATED"
    })))
}

async fn create_pickup_request(
    State(state): State<AppState>,
    Json(payload): Json<CreatePickupOrderPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let req_id = format!("REQ-{}", &hex::encode(blake3::hash(payload.item_description.as_bytes()).as_bytes())[..8]);

    let request = PickupRequest {
        id: req_id.clone(),
        requester_node_id: state.identity.node_id(),
        pickup_location: payload.pickup_location,
        item_description: payload.item_description,
        dropoff_location: payload.dropoff_location,
        payment_spec: payload.payment_spec,
        status: RequestStatus::Pending,
    };

    state.storage.create_pickup_request(&request).await?;

    Ok(Json(serde_json::json!({
        "status": "REQUEST_CREATED",
        "request_id": req_id,
        "state": "PENDING"
    })))
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
