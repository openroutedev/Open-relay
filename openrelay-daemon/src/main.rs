use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use openrelay_crypto::identity::NodeIdentity;
use openrelay_label::{format::PackageLabelData, pdf::PackingSlipGenerator};
use openrelay_protocol::{
    hash_pin, haversine_km, CourierBid, CourierRequirements, DisputeRecord, DropoffMode,
    GossipMessage, HandoffRecord, NodeRating, PaymentSpec, PeerNode, PickupRequest, RequestStatus,
    RequestType, ShipmentState, StorageEngine, VehicleType,
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
    dropoff_mode: Option<String>,
    verification_pin: Option<String>,
    pickup_location: String,
    pickup_lat: Option<f64>,
    pickup_lon: Option<f64>,
    item_description: String,
    dropoff_location: String,
    payment_spec: PaymentSpec,
    payment_amount_num: f64,
    ttl_seconds: Option<i64>,
    min_rating: Option<f64>,
    require_insulated_bag: Option<bool>,
    required_vehicle: Option<String>,
}

#[derive(Deserialize)]
struct CompleteDeliveryPayload {
    verification_pin: Option<String>,
    dropoff_notes: Option<String>,
    photo_hash: Option<String>,
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

#[derive(Deserialize)]
struct RegisterPeerPayload {
    node_id: String,
    endpoint_url: String,
}

#[derive(Deserialize)]
struct PriceEstimateQuery {
    pickup_lat: f64,
    pickup_lon: f64,
    dropoff_lat: f64,
    dropoff_lon: f64,
}

#[derive(Deserialize)]
struct SubmitRatingPayload {
    score: f64,
    review_notes: String,
}

#[derive(Deserialize)]
struct SubmitBidPayload {
    bid_amount: f64,
    bid_notes: String,
}

#[derive(Deserialize)]
struct FileDisputePayload {
    reason: String,
    evidence_hash: String,
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
        .route("/v1/peers", get(get_peers).post(register_peer))
        .route("/v1/gossip/broadcast", post(receive_gossip))
        .route("/v1/pricing/estimate", get(pricing_estimate))
        .route("/v1/nodes/:id/rate", post(submit_rating))
        .route("/v1/requests/:id/bids", post(submit_bid))
        .route("/v1/requests/:id/dispute", post(file_dispute))
        .route("/v1/label/pdf", post(generate_label_pdf))
        .route("/v1/shipments", post(create_shipment))
        .route("/v1/shipments/:commitment/history", get(get_shipment_history))
        .route("/v1/requests", post(create_pickup_request).get(query_pickup_requests))
        .route("/v1/requests/:id/complete", post(complete_delivery))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("[+] Daemon listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ACTIVE_ONLINE", "version": "0.3.0", "node_id": state.identity.node_id() }))
}

async fn register_peer(State(state): State<AppState>, Json(payload): Json<RegisterPeerPayload>) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    state.storage.register_peer(&PeerNode { node_id: payload.node_id, endpoint_url: payload.endpoint_url, last_seen: now }).await?;
    Ok(Json(serde_json::json!({ "status": "PEER_REGISTERED" })))
}

async fn get_peers(State(state): State<AppState>) -> Result<Json<Vec<PeerNode>>, String> {
    Ok(Json(state.storage.fetch_peers().await?))
}

async fn receive_gossip(State(state): State<AppState>, Json(msg): Json<GossipMessage>) -> Result<Json<serde_json::Value>, String> {
    if state.storage.record_gossip_seen(&msg.msg_id).await? {
        Ok(Json(serde_json::json!({ "status": "GOSSIP_ACCEPTED" })))
    } else {
        Ok(Json(serde_json::json!({ "status": "GOSSIP_DUPLICATE_IGNORED" })))
    }
}

async fn pricing_estimate(Query(query): Query<PriceEstimateQuery>) -> Json<serde_json::Value> {
    let dist_km = haversine_km(query.pickup_lat, query.pickup_lon, query.dropoff_lat, query.dropoff_lon);
    
    // Dynamic Algorithm: Base Rate ($5) + Distance ($1.20/km)
    let suggested_base = 5.0 + (dist_km * 1.20);
    let anti_gouge_cap = suggested_base * 1.5;

    Json(serde_json::json!({
        "distance_km": dist_km,
        "suggested_amount": format!("{:.2}", suggested_base),
        "maximum_bid_cap": format!("{:.2}", anti_gouge_cap)
    }))
}

async fn submit_rating(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    Json(payload): Json<SubmitRatingPayload>,
) -> Result<Json<serde_json::Value>, String> {
    if payload.score < 1.0 || payload.score > 5.0 { return Err("Score must be between 1.0 and 5.0".into()); }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;

    let rating = NodeRating {
        rater_node_id: state.identity.node_id(),
        subject_node_id: subject_id.clone(),
        score: payload.score,
        review_notes: payload.review_notes,
        timestamp: now,
    };

    state.storage.save_rating(&rating).await?;
    Ok(Json(serde_json::json!({ "status": "RATING_SUBMITTED", "subject": subject_id })))
}

async fn submit_bid(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Json(payload): Json<SubmitBidPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let req = state.storage.fetch_request_by_id(&request_id).await?
        .ok_or("Pickup Request not found")?;

    if req.status != RequestStatus::Pending {
        return Err("Cannot bid on a task that is not pending".into());
    }

    let limit = req.payment_amount_num * 1.5;
    if payload.bid_amount > limit {
        return Err(format!("Anti-Gouging Error: Bid {:.2} exceeds 50% cap of {:.2}", payload.bid_amount, limit));
    }

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let bid = CourierBid {
        request_id: request_id.clone(),
        courier_node_id: state.identity.node_id(),
        bid_amount: payload.bid_amount,
        bid_notes: payload.bid_notes,
        timestamp: now,
    };

    state.storage.save_bid(&bid).await?;
    Ok(Json(serde_json::json!({ "status": "BID_SUBMITTED", "request_id": request_id })))
}

async fn file_dispute(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Json(payload): Json<FileDisputePayload>,
) -> Result<Json<serde_json::Value>, String> {
    if payload.evidence_hash.trim().is_empty() {
        return Err("Mandatory Abuse Protection: Evidence photo hash required to file dispute".into());
    }

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    
    let dispute = DisputeRecord {
        request_id: request_id.clone(),
        filed_by_node_id: state.identity.node_id(),
        reason: payload.reason,
        evidence_hash: payload.evidence_hash,
        timestamp: now,
    };

    state.storage.file_dispute(&dispute).await?;
    Ok(Json(serde_json::json!({ "status": "DISPUTE_FILED", "request_id": request_id })))
}

async fn create_shipment(State(state): State<AppState>, Json(payload): Json<CreateShipmentRequest>) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    state.storage.save_shipment(&payload.commitment_hex, ShipmentState::Created, &payload.seal_serial).await?;
    state.storage.record_handoff_event(&HandoffRecord { commitment: payload.commitment_hex.clone(), hop_index: 0, node_pubkey_hash: state.identity.node_id(), event_type: "PACKAGE_CREATED".into(), timestamp: now }).await?;
    Ok(Json(serde_json::json!({ "status": "SUCCESS", "commitment": payload.commitment_hex, "state": "CREATED" })))
}

async fn get_shipment_history(State(state): State<AppState>, Path(commitment): Path<String>) -> Result<Json<Vec<HandoffRecord>>, String> {
    Ok(Json(state.storage.fetch_handoff_history(&commitment).await?))
}

async fn create_pickup_request(State(state): State<AppState>, Json(payload): Json<CreatePickupOrderPayload>) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let expires_at = now + payload.ttl_seconds.unwrap_or(86400);
    let req_id = format!("REQ-{}", &hex::encode(blake3::hash(payload.item_description.as_bytes()).as_bytes())[..8]);
    
    let request = PickupRequest {
        id: req_id.clone(), requester_node_id: state.identity.node_id(),
        request_type: RequestType::from_str(payload.request_type.as_deref().unwrap_or("CUSTOM_TASK")),
        dropoff_mode: DropoffMode::from_str(payload.dropoff_mode.as_deref().unwrap_or("IN_PERSON_HANDOFF")),
        requirements: CourierRequirements {
            min_rating: payload.min_rating.unwrap_or(0.0),
            require_insulated_bag: payload.require_insulated_bag.unwrap_or(false),
            required_vehicle: VehicleType::from_str(payload.required_vehicle.as_deref().unwrap_or("ANY")),
        },
        pin_hash: payload.verification_pin.as_deref().map(hash_pin),
        pickup_location: payload.pickup_location, pickup_lat: payload.pickup_lat, pickup_lon: payload.pickup_lon,
        item_description: payload.item_description, dropoff_location: payload.dropoff_location,
        payment_spec: payload.payment_spec, payment_amount_num: payload.payment_amount_num,
        status: RequestStatus::Pending, created_at: now, expires_at,
    };

    state.storage.create_pickup_request(&request).await?;
    Ok(Json(serde_json::json!({ "status": "REQUEST_CREATED", "request_id": req_id, "state": "PENDING" })))
}

async fn complete_delivery(State(state): State<AppState>, Path(id): Path<String>, Json(payload): Json<CompleteDeliveryPayload>) -> Result<Json<serde_json::Value>, String> {
    if !state.storage.verify_and_complete_request(&id, payload.verification_pin.as_deref()).await? {
        return Err("Verification failed".into());
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    state.storage.record_handoff_event(&HandoffRecord {
        commitment: id.clone(), hop_index: 99, node_pubkey_hash: state.identity.node_id(),
        event_type: format!("DELIVERY_COMPLETED | Notes: {} | PhotoHash: {}", payload.dropoff_notes.unwrap_or_else(|| "None".into()), payload.photo_hash.unwrap_or_else(|| "None".into())),
        timestamp: now,
    }).await?;
    Ok(Json(serde_json::json!({ "status": "SUCCESS", "request_id": id, "state": "COMPLETED" })))
}

async fn query_pickup_requests(State(state): State<AppState>, Query(filter): Query<RequestQueryFilter>) -> Result<Json<Vec<serde_json::Value>>, String> {
    let mut requests = state.storage.fetch_pending_requests().await?;
    
    // Restored fully active filtering logic
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
        requests.retain(|r| r.pickup_lat.zip(r.pickup_lon).map_or(false, |(lat, lon)| haversine_km(u_lat, u_lon, lat, lon) <= max_dist));
    }

    // Restored fully active sorting logic
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

async fn generate_label_pdf(Json(payload): Json<CreatePackingSlipRequest>) -> Result<Json<CreatePackingSlipResponse>, String> {
    let pdf_bytes = PackingSlipGenerator::generate_pdf(&PackageLabelData { commitment_hex: payload.commitment_hex, seal_serial: payload.seal_serial, next_staging_point: payload.next_staging_point, deadline_timestamp: payload.deadline })?;
    Ok(Json(CreatePackingSlipResponse { pdf_base64: data_encoding::BASE64.encode(&pdf_bytes) }))
}
