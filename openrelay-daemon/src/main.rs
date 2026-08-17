use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use openrelay_crypto::identity::NodeIdentity;
use openrelay_label::{format::PackageLabelData, pdf::PackingSlipGenerator};
use openrelay_protocol::{
    haversine_km, HandoffRecord, PaymentSpec, PickupRequest, RequestStatus, RequestType,
    ShipmentState, StorageEngine,
};
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
    request_type: Option<String>,
    pickup_location: String,
    pickup_lat: Option<f64>,
    pickup_lon: Option<f64>,
    item_description: String,
    dropoff_location: String,
    payment_spec: PaymentSpec,
    payment_amount_num: f64,
}

#[derive(Deserialize)]
struct RequestQueryFilter {
    sort_by: Option<String>,
    order: Option<String>,
    request_type: Option<String>,
    payment_method: Option<String>,
    min_amount: Option<f64>,
    user_lat: Option<f64>,
    user_lon: Option<f64>,
    max_distance_km: Option<f64>,
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
        .route("/v1/shipments/:commitment/history", get(get_shipment_history))
        .route("/v1/requests", post(create_pickup_request).get(query_pickup_requests))
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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    state
        .storage
        .save_shipment(&payload.commitment_hex, ShipmentState::Created, &payload.seal_serial)
        .await?;

    let initial_log = HandoffRecord {
        commitment: payload.commitment_hex.clone(),
        hop_index: 0,
        node_pubkey_hash: state.identity.node_id(),
        event_type: "PACKAGE_CREATED".into(),
        timestamp: now,
    };

    state.storage.record_handoff_event(&initial_log).await?;

    Ok(Json(serde_json::json!({
        "status": "SUCCESS",
        "commitment": payload.commitment_hex,
        "state": "CREATED"
    })))
}

async fn get_shipment_history(
    State(state): State<AppState>,
    Path(commitment): Path<String>,
) -> Result<Json<Vec<HandoffRecord>>, String> {
    let history = state.storage.fetch_handoff_history(&commitment).await?;
    Ok(Json(history))
}

async fn create_pickup_request(
    State(state): State<AppState>,
    Json(payload): Json<CreatePickupOrderPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let req_id = format!("REQ-{}", &hex::encode(blake3::hash(payload.item_description.as_bytes()).as_bytes())[..8]);
    let req_type = RequestType::from_str(payload.request_type.as_deref().unwrap_or("CUSTOM_TASK"));

    let request = PickupRequest {
        id: req_id.clone(),
        requester_node_id: state.identity.node_id(),
        request_type: req_type,
        pickup_location: payload.pickup_location,
        pickup_lat: payload.pickup_lat,
        pickup_lon: payload.pickup_lon,
        item_description: payload.item_description,
        dropoff_location: payload.dropoff_location,
        payment_spec: payload.payment_spec,
        payment_amount_num: payload.payment_amount_num,
        status: RequestStatus::Pending,
        created_at: now,
    };

    state.storage.create_pickup_request(&request).await?;

    Ok(Json(serde_json::json!({
        "status": "REQUEST_CREATED",
        "request_id": req_id,
        "state": "PENDING"
    })))
}

async fn query_pickup_requests(
    State(state): State<AppState>,
    Query(filter): Query<RequestQueryFilter>,
) -> Result<Json<Vec<serde_json::Value>>, String> {
    let mut requests = state.storage.fetch_pending_requests().await?;

    if let Some(min_amt) = filter.min_amount {
        requests.retain(|r| r.payment_amount_num >= min_amt);
    }

    if let Some(ref req_type_filter) = filter.request_type {
        requests.retain(|r| r.request_type.as_str() == req_type_filter.to_uppercase());
    }

    if let Some(ref method_filter) = filter.payment_method {
        requests.retain(|r| {
            r.payment_spec
                .accepted_methods
                .iter()
                .any(|m| m.name().to_lowercase().contains(&method_filter.to_lowercase()))
        });
    }

    if let (Some(u_lat), Some(u_lon), Some(max_dist)) = (filter.user_lat, filter.user_lon, filter.max_distance_km) {
        requests.retain(|r| {
            if let (Some(p_lat), Some(p_lon)) = (r.pickup_lat, r.pickup_lon) {
                haversine_km(u_lat, u_lon, p_lat, p_lon) <= max_dist
            } else {
                false
            }
        });
    }

    let sort_mode = filter.sort_by.as_deref().unwrap_or("date");
    let is_desc = filter.order.as_deref().unwrap_or("desc") == "desc";

    requests.sort_by(|a, b| {
        let cmp = match sort_mode {
            "amount" => a.payment_amount_num.partial_cmp(&b.payment_amount_num).unwrap_or(std::cmp::Ordering::Equal),
            "distance" => {
                if let (Some(u_lat), Some(u_lon)) = (filter.user_lat, filter.user_lon) {
                    let dist_a = a.pickup_lat.zip(a.pickup_lon).map(|(lat, lon)| haversine_km(u_lat, u_lon, lat, lon)).unwrap_or(f64::MAX);
                    let dist_b = b.pickup_lat.zip(b.pickup_lon).map(|(lat, lon)| haversine_km(u_lat, u_lon, lat, lon)).unwrap_or(f64::MAX);
                    dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    a.created_at.cmp(&b.created_at)
                }
            }
            _ => a.created_at.cmp(&b.created_at),
        };

        if is_desc { cmp.reverse() } else { cmp }
    });

    let json_list = requests
        .into_iter()
        .map(|r| {
            let mut val = serde_json::to_value(&r).unwrap();
            if let (Some(u_lat), Some(u_lon)) = (filter.user_lat, filter.user_lon) {
                if let (Some(p_lat), Some(p_lon)) = (r.pickup_lat, r.pickup_lon) {
                    let dist = haversine_km(u_lat, u_lon, p_lat, p_lon);
                    val.as_object_mut().unwrap().insert("distance_km".to_string(), serde_json::json!(dist));
                }
            }
            val
        })
        .collect();

    Ok(Json(json_list))
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
