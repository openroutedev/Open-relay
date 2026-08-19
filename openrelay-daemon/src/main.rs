use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{sse::Event, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use openrelay_crypto::identity::NodeIdentity;
use openrelay_label::{format::PackageLabelData, pdf::PackingSlipGenerator};
use openrelay_protocol::{
    decrypt_e2ee_message, encrypt_e2ee_message, generate_pseudonym, hash_pin, haversine_km, CourierBid,
    CourierPresence, CourierRequirements, DisputeRecord, DropoffMode, EncryptedMessage, GossipMessage,
    HandoffRecord, NodeRating, PaymentSpec, PeerNode, PickupRequest, RequestStatus, RequestType, ShipmentState,
    StagingHub, StorageEngine, VehicleType,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

#[derive(Clone)]
struct AppState {
    identity: Arc<NodeIdentity>,
    x25519_sk_bytes: Arc<[u8; 32]>,
    x25519_pubkey_hex: String,
    node_pseudonym: String,
    storage: Arc<StorageEngine>,
    event_tx: broadcast::Sender<String>,
    rate_limit_count: Arc<AtomicUsize>,
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
    target_courier_id: Option<String>,
    staging_hub_id: Option<String>,
    hub_fee_num: Option<f64>,
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
struct AcceptBidPayload {
    courier_node_id: String,
}

#[derive(Deserialize)]
struct SendMessagePayload {
    request_id: Option<String>,
    recipient_node_id: String,
    recipient_x25519_pk_hex: String,
    plaintext: String,
}

#[derive(Deserialize)]
struct DecryptMessagePayload {
    ephemeral_pubkey_hex: String,
    nonce_hex: String,
    ciphertext_hex: String,
    mac_tag_hex: String,
}

#[derive(Serialize)]
struct DecryptedMessageResponse {
    id: String,
    request_id: Option<String>,
    sender_node_id: String,
    recipient_node_id: String,
    plaintext: String,
    timestamp: i64,
}

#[derive(Deserialize)]
struct PresenceHeartbeatPayload {
    username: Option<String>,
    x25519_pubkey_hex: Option<String>,
    lat: f64,
    lon: f64,
    is_online: bool,
}

#[derive(Deserialize)]
struct ConfirmSettlementPayload {
    is_requester: bool,
}

#[derive(Deserialize)]
struct RegisterHubPayload {
    name: String,
    address: String,
    lat: f64,
    lon: f64,
    hub_fee_num: f64,
    capacity: i32,
}

#[derive(Deserialize)]
struct MapLinksQuery {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct OnlineCouriersQuery {
    user_lat: f64,
    user_lon: f64,
    max_dist_km: Option<f64>,
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
    ttl_seconds: Option<i64>,
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

async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if state.rate_limit_count.fetch_add(1, Ordering::Relaxed) >= 100 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    Ok(next.run(request).await)
}

#[tokio::main]
async fn main() {
    println!("=== Starting OpenRelay v0.3 Node Daemon ===");

    let node_identity = NodeIdentity::generate();
    println!("[+] Node ID: {}", node_identity.node_id());

    // Generate local static X25519 keypair for E2EE
    let mut x25519_sk_bytes = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut x25519_sk_bytes);
    let x25519_sk = X25519StaticSecret::from(x25519_sk_bytes);
    let x25519_pk = X25519PublicKey::from(&x25519_sk);
    let x25519_pubkey_hex = hex::encode(x25519_pk.as_bytes());
    let node_pseudonym = generate_pseudonym();

    println!("[+] X25519 Public Key: {}", x25519_pubkey_hex);
    println!("[+] Default Session Pseudonym: {}", node_pseudonym);

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://openrelay.db?mode=rwc".to_string());
    let storage = StorageEngine::connect(&db_url).await.expect("Failed to init storage");
    println!("[+] SQLite persistent storage connected: {}", db_url);

    let (event_tx, _) = broadcast::channel::<String>(100);
    let rate_limit_count = Arc::new(AtomicUsize::new(0));

    let counter_clone = rate_limit_count.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            counter_clone.store(0, Ordering::Relaxed);
        }
    });

    let state = AppState {
        identity: Arc::new(node_identity),
        x25519_sk_bytes: Arc::new(x25519_sk_bytes),
        x25519_pubkey_hex,
        node_pseudonym,
        storage: Arc::new(storage),
        event_tx,
        rate_limit_count,
    };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/v1/status", get(get_status))
        .route("/v1/events", get(sse_events_handler))
        .route("/v1/identity/pseudonym", get(get_random_pseudonym))
        .route("/v1/navigation/links", get(get_navigation_links))
        .route("/v1/messages", post(send_encrypted_message).get(fetch_encrypted_messages))
        .route("/v1/messages/decrypted", get(fetch_auto_decrypted_messages))
        .route("/v1/messages/decrypt", post(decrypt_single_message))
        .route("/v1/presence", post(presence_heartbeat))
        .route("/v1/couriers/online", get(get_online_couriers))
        .route("/v1/hubs", post(register_staging_hub).get(get_nearby_hubs))
        .route("/v1/peers", get(get_peers).post(register_peer))
        .route("/v1/gossip/broadcast", post(receive_gossip))
        .route("/v1/pricing/estimate", get(pricing_estimate))
        .route("/v1/nodes/:id/rate", post(submit_rating))
        .route("/v1/requests/:id", get(get_single_request))
        .route("/v1/requests/:id/cancel", post(cancel_pickup_request))
        .route("/v1/requests/:id/confirm", post(confirm_settlement))
        .route("/v1/requests/:id/bids", get(get_request_bids).post(submit_bid))
        .route("/v1/requests/:id/accept_bid", post(accept_bid))
        .route("/v1/requests/:id/dispute", post(file_dispute))
        .route("/v1/label/pdf", post(generate_label_pdf))
        .route("/v1/shipments", post(create_shipment))
        .route("/v1/shipments/:commitment/history", get(get_shipment_history))
        .route("/v1/requests", post(create_pickup_request).get(query_pickup_requests))
        .route("/v1/requests/:id/complete", post(complete_delivery))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(cors)
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
        "node_id": state.identity.node_id(),
        "x25519_pubkey_hex": state.x25519_pubkey_hex,
        "default_pseudonym": state.node_pseudonym
    }))
}

async fn get_random_pseudonym() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "pseudonym": generate_pseudonym() }))
}

async fn get_navigation_links(Query(query): Query<MapLinksQuery>) -> Json<serde_json::Value> {
    let google = format!("https://www.google.com/maps/dir/?api=1&destination={},{}", query.lat, query.lon);
    let apple = format!("https://maps.apple.com/?daddr={},{}", query.lat, query.lon);
    let waze = format!("https://waze.com/ul?ll={},{}&navigate=yes", query.lat, query.lon);
    let osm = format!("https://www.openstreetmap.org/directions?engine=fossgis_osrm_car&route=;{},{}", query.lat, query.lon);

    Json(serde_json::json!({
        "coordinates": { "lat": query.lat, "lon": query.lon },
        "links": { "google_maps": google, "apple_maps": apple, "waze": waze, "openstreetmap": osm }
    }))
}

async fn send_encrypted_message(
    State(state): State<AppState>,
    Json(payload): Json<SendMessagePayload>,
) -> Result<Json<serde_json::Value>, String> {
    let (eph_pk_hex, nonce_hex, cipher_hex, mac_tag_hex) = encrypt_e2ee_message(&payload.recipient_x25519_pk_hex, payload.plaintext.as_bytes())?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let msg_id = format!("MSG-{}", &hex::encode(blake3::hash(cipher_hex.as_bytes()).as_bytes())[..8]);

    let message = EncryptedMessage {
        id: msg_id.clone(), request_id: payload.request_id, sender_node_id: state.identity.node_id(),
        recipient_node_id: payload.recipient_node_id.clone(), ephemeral_pubkey_hex: eph_pk_hex,
        nonce_hex, ciphertext_hex: cipher_hex, mac_tag_hex, timestamp: now,
    };

    state.storage.save_chat_message(&message).await?;
    let _ = state.event_tx.send(format!("NEW_MESSAGE|{}", payload.recipient_node_id));
    Ok(Json(serde_json::json!({ "status": "MESSAGE_SENT", "msg_id": msg_id })))
}

async fn decrypt_single_message(
    State(state): State<AppState>,
    Json(payload): Json<DecryptMessagePayload>,
) -> Result<Json<serde_json::Value>, String> {
    let decrypted_bytes = decrypt_e2ee_message(
        &state.x25519_sk_bytes,
        &payload.ephemeral_pubkey_hex,
        &payload.nonce_hex,
        &payload.ciphertext_hex,
        &payload.mac_tag_hex,
    )?;
    let plaintext = String::from_utf8(decrypted_bytes).map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({ "plaintext": plaintext })))
}

async fn fetch_encrypted_messages(
    State(state): State<AppState>,
    Query(filter): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<EncryptedMessage>>, String> {
    let req_id = filter.get("request_id").map(|s| s.as_str());
    let msgs = state.storage.fetch_messages(&state.identity.node_id(), req_id).await?;
    Ok(Json(msgs))
}

async fn fetch_auto_decrypted_messages(
    State(state): State<AppState>,
    Query(filter): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<DecryptedMessageResponse>>, String> {
    let req_id = filter.get("request_id").map(|s| s.as_str());
    let raw_msgs = state.storage.fetch_messages(&state.identity.node_id(), req_id).await?;
    
    let mut decrypted_list = Vec::new();
    for msg in raw_msgs {
        if let Ok(decrypted_bytes) = decrypt_e2ee_message(
            &state.x25519_sk_bytes,
            &msg.ephemeral_pubkey_hex,
            &msg.nonce_hex,
            &msg.ciphertext_hex,
            &msg.mac_tag_hex,
        ) {
            if let Ok(plaintext) = String::from_utf8(decrypted_bytes) {
                decrypted_list.push(DecryptedMessageResponse {
                    id: msg.id,
                    request_id: msg.request_id,
                    sender_node_id: msg.sender_node_id,
                    recipient_node_id: msg.recipient_node_id,
                    plaintext,
                    timestamp: msg.timestamp,
                });
            }
        }
    }
    Ok(Json(decrypted_list))
}

async fn presence_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<PresenceHeartbeatPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let username = payload.username.unwrap_or_else(|| state.node_pseudonym.clone());
    let pubkey = payload.x25519_pubkey_hex.unwrap_or_else(|| state.x25519_pubkey_hex.clone());

    let presence = CourierPresence {
        courier_node_id: state.identity.node_id(), username, x25519_pubkey_hex: pubkey,
        lat: payload.lat, lon: payload.lon, is_online: payload.is_online, last_ping: now,
    };

    state.storage.update_courier_presence(&presence).await?;
    Ok(Json(serde_json::json!({ "status": "PRESENCE_UPDATED", "is_online": payload.is_online })))
}

async fn get_online_couriers(
    State(state): State<AppState>,
    Query(query): Query<OnlineCouriersQuery>,
) -> Result<Json<Vec<CourierPresence>>, String> {
    let max_dist = query.max_dist_km.unwrap_or(25.0);
    let couriers = state.storage.fetch_online_couriers(query.user_lat, query.user_lon, max_dist).await?;
    Ok(Json(couriers))
}

async fn confirm_settlement(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ConfirmSettlementPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let (req_conf, cour_conf) = state.storage.confirm_settlement(&id, payload.is_requester).await?;
    let fully_settled = req_conf && cour_conf;
    let _ = state.event_tx.send(format!("SETTLEMENT_UPDATE|{}|{}", id, fully_settled));

    Ok(Json(serde_json::json!({
        "status": if fully_settled { "FULLY_SETTLED_COMPLETED" } else { "CONFIRMATION_LOGGED_WAITING_OTHER_PARTY" },
        "requester_confirmed": req_conf,
        "courier_confirmed": cour_conf
    })))
}

async fn register_staging_hub(
    State(state): State<AppState>,
    Json(payload): Json<RegisterHubPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let hub_id = format!("HUB-{}", &hex::encode(blake3::hash(payload.name.as_bytes()).as_bytes())[..8]);

    let hub = StagingHub {
        hub_id: hub_id.clone(), operator_node_id: state.identity.node_id(), name: payload.name, address: payload.address,
        lat: payload.lat, lon: payload.lon, hub_fee_num: payload.hub_fee_num, holding_capacity: payload.capacity, created_at: now,
    };

    state.storage.register_staging_hub(&hub).await?;
    Ok(Json(serde_json::json!({ "status": "HUB_REGISTERED", "hub_id": hub_id })))
}

async fn get_nearby_hubs(
    State(state): State<AppState>,
    Query(query): Query<OnlineCouriersQuery>,
) -> Result<Json<Vec<StagingHub>>, String> {
    let max_dist = query.max_dist_km.unwrap_or(50.0);
    let hubs = state.storage.fetch_nearby_hubs(query.user_lat, query.user_lon, max_dist).await?;
    Ok(Json(hubs))
}

async fn sse_events_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(msg) => Some(Ok(Event::default().data(msg))),
        Err(_) => None,
    });
    Sse::new(stream)
}

async fn get_single_request(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PickupRequest>, String> {
    let request = state.storage.fetch_request_by_id(&id).await?.ok_or_else(|| "Request not found".to_string())?;
    Ok(Json(request))
}

async fn cancel_pickup_request(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, String> {
    state.storage.cancel_request(&id).await?;
    let _ = state.event_tx.send(format!("REQUEST_CANCELLED|{}", id));
    Ok(Json(serde_json::json!({ "status": "REQUEST_CANCELLED", "request_id": id })))
}

async fn get_request_bids(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CourierBid>>, String> {
    let bids = state.storage.fetch_bids_for_request(&id).await?;
    Ok(Json(bids))
}

async fn accept_bid(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AcceptBidPayload>,
) -> Result<Json<serde_json::Value>, String> {
    state.storage.accept_bid(&id, &payload.courier_node_id).await?;
    let _ = state.event_tx.send(format!("BID_ACCEPTED|{}|{}", id, payload.courier_node_id));
    Ok(Json(serde_json::json!({ "status": "BID_ACCEPTED", "request_id": id, "courier": payload.courier_node_id })))
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
        let msg_clone = msg.clone();
        let storage_clone = state.storage.clone();
        tokio::spawn(async move {
            if let Ok(peers) = storage_clone.fetch_peers().await {
                let client = reqwest::Client::new();
                for peer in peers {
                    if peer.node_id != msg_clone.origin_node_id {
                        let url = format!("{}/v1/gossip/broadcast", peer.endpoint_url);
                        let _ = client.post(&url).json(&msg_clone).send().await;
                    }
                }
            }
        });
        Ok(Json(serde_json::json!({ "status": "GOSSIP_ACCEPTED_AND_FORWARDED" })))
    } else {
        Ok(Json(serde_json::json!({ "status": "GOSSIP_DUPLICATE_IGNORED" })))
    }
}

async fn pricing_estimate(Query(query): Query<PriceEstimateQuery>) -> Json<serde_json::Value> {
    let dist_km = haversine_km(query.pickup_lat, query.pickup_lon, query.dropoff_lat, query.dropoff_lon);
    let urgency_multiplier = match query.ttl_seconds {
        Some(ttl) if ttl <= 3600 => 1.5,
        Some(ttl) if ttl <= 10800 => 1.25,
        _ => 1.0,
    };
    let suggested_base = (5.0 + (dist_km * 1.20)) * urgency_multiplier;
    let anti_gouge_cap = suggested_base * 1.5;

    Json(serde_json::json!({
        "distance_km": dist_km,
        "urgency_multiplier": urgency_multiplier,
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
        rater_node_id: state.identity.node_id(), subject_node_id: subject_id.clone(),
        score: payload.score, review_notes: payload.review_notes, timestamp: now,
    };

    state.storage.save_rating(&rating).await?;
    Ok(Json(serde_json::json!({ "status": "RATING_SUBMITTED", "subject": subject_id })))
}

async fn submit_bid(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Json(payload): Json<SubmitBidPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let req = state.storage.fetch_request_by_id(&request_id).await?.ok_or("Pickup Request not found")?;

    if req.status != RequestStatus::Pending {
        return Err("Cannot bid on a task that is not pending".into());
    }

    let limit = req.payment_amount_num * 1.5;
    if payload.bid_amount > limit {
        return Err(format!("Anti-Gouging Error: Bid {:.2} exceeds 50% cap of {:.2}", payload.bid_amount, limit));
    }

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let bid = CourierBid {
        request_id: request_id.clone(), courier_node_id: state.identity.node_id(),
        bid_amount: payload.bid_amount, bid_notes: payload.bid_notes, timestamp: now,
    };

    state.storage.save_bid(&bid).await?;
    let _ = state.event_tx.send(format!("NEW_BID|{}|{}", request_id, payload.bid_amount));
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

    let filer_id = state.identity.node_id();

    if state.storage.has_existing_dispute(&request_id, &filer_id).await? {
        return Err("You have already filed a dispute for this request.".into());
    }

    if state.storage.check_dispute_rate_limit(&filer_id, 900).await? >= 3 {
        return Err("Rate Limit Exceeded: Cooldown active due to excessive dispute filings.".into());
    }

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    
    let dispute = DisputeRecord {
        request_id: request_id.clone(), filed_by_node_id: filer_id, reason: payload.reason,
        evidence_hash: payload.evidence_hash, timestamp: now,
    };

    state.storage.file_dispute(&dispute).await?;
    let _ = state.event_tx.send(format!("DISPUTE_FILED|{}", request_id));
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
        target_courier_id: payload.target_courier_id, staging_hub_id: payload.staging_hub_id, hub_fee_num: payload.hub_fee_num,
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
        requester_confirmed: false, courier_confirmed: false,
        status: RequestStatus::Pending, created_at: now, expires_at,
    };

    state.storage.create_pickup_request(&request).await?;
    let _ = state.event_tx.send(format!("NEW_REQUEST|{}", req_id));
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
    let _ = state.event_tx.send(format!("DELIVERY_COMPLETED|{}", id));
    Ok(Json(serde_json::json!({ "status": "SUCCESS", "request_id": id, "state": "COMPLETED" })))
}

async fn query_pickup_requests(State(state): State<AppState>, Query(filter): Query<RequestQueryFilter>) -> Result<Json<Vec<serde_json::Value>>, String> {
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
        requests.retain(|r| r.pickup_lat.zip(r.pickup_lon).map_or(false, |(lat, lon)| haversine_km(u_lat, u_lon, lat, lon) <= max_dist));
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

async fn generate_label_pdf(Json(payload): Json<CreatePackingSlipRequest>) -> Result<Json<CreatePackingSlipResponse>, String> {
    let pdf_bytes = PackingSlipGenerator::generate_pdf(&PackageLabelData { commitment_hex: payload.commitment_hex, seal_serial: payload.seal_serial, next_staging_point: payload.next_staging_point, deadline_timestamp: payload.deadline })?;
    Ok(Json(CreatePackingSlipResponse { pdf_base64: data_encoding::BASE64.encode(&pdf_bytes) }))
}
