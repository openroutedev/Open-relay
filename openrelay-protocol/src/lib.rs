use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    CashOnHandoff,
    P2PFiat { provider: String, handle: String },
    Crypto { network: String, address_or_invoice: String },
    Custom(String),
}

impl PaymentMethod {
    pub fn name(&self) -> String {
        match self {
            PaymentMethod::CashOnHandoff => "CashOnHandoff".into(),
            PaymentMethod::P2PFiat { provider, .. } => format!("Fiat:{}", provider),
            PaymentMethod::Crypto { network, .. } => format!("Crypto:{}", network),
            PaymentMethod::Custom(val) => format!("Custom:{}", val),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSpec {
    pub amount_prompt: String,
    pub accepted_methods: Vec<PaymentMethod>,
    pub is_settled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VehicleType {
    Any,
    Foot,
    Bicycle,
    Car,
    CargoVan,
}

impl VehicleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VehicleType::Any => "ANY",
            VehicleType::Foot => "FOOT",
            VehicleType::Bicycle => "BICYCLE",
            VehicleType::Car => "CAR",
            VehicleType::CargoVan => "CARGO_VAN",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "FOOT" => VehicleType::Foot,
            "BICYCLE" => VehicleType::Bicycle,
            "CAR" => VehicleType::Car,
            "CARGO_VAN" => VehicleType::CargoVan,
            _ => VehicleType::Any,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourierRequirements {
    pub min_rating: f64,
    pub require_insulated_bag: bool,
    pub required_vehicle: VehicleType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestType {
    FoodPickup,
    StorePickup,
    PackageDelivery,
    CustomTask,
}

impl RequestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestType::FoodPickup => "FOOD_PICKUP",
            RequestType::StorePickup => "STORE_PICKUP",
            RequestType::PackageDelivery => "PACKAGE_DELIVERY",
            _ => "CUSTOM_TASK",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "FOOD_PICKUP" => RequestType::FoodPickup,
            "STORE_PICKUP" => RequestType::StorePickup,
            "PACKAGE_DELIVERY" => RequestType::PackageDelivery,
            _ => RequestType::CustomTask,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropoffMode {
    InPersonHandoff,
    UnattendedPorch,
}

impl DropoffMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DropoffMode::InPersonHandoff => "IN_PERSON_HANDOFF",
            DropoffMode::UnattendedPorch => "UNATTENDED_PORCH",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "UNATTENDED_PORCH" => DropoffMode::UnattendedPorch,
            _ => DropoffMode::InPersonHandoff,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Claimed,
    Completed,
    Cancelled,
    Disputed,
}

impl RequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestStatus::Pending => "PENDING",
            RequestStatus::Claimed => "CLAIMED",
            RequestStatus::Completed => "COMPLETED",
            RequestStatus::Cancelled => "CANCELLED",
            RequestStatus::Disputed => "DISPUTED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CLAIMED" => RequestStatus::Claimed,
            "COMPLETED" => RequestStatus::Completed,
            "CANCELLED" => RequestStatus::Cancelled,
            "DISPUTED" => RequestStatus::Disputed,
            _ => RequestStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupRequest {
    pub id: String,
    pub requester_node_id: String,
    pub target_courier_id: Option<String>,
    pub staging_hub_id: Option<String>,
    pub hub_fee_num: Option<f64>,
    pub request_type: RequestType,
    pub dropoff_mode: DropoffMode,
    pub requirements: CourierRequirements,
    pub pin_hash: Option<String>,
    pub pickup_location: String,
    pub pickup_lat: Option<f64>,
    pub pickup_lon: Option<f64>,
    pub item_description: String,
    pub dropoff_location: String,
    pub payment_spec: PaymentSpec,
    pub payment_amount_num: f64,
    pub requester_confirmed: bool,
    pub courier_confirmed: bool,
    pub status: RequestStatus,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub id: String,
    pub request_id: Option<String>,
    pub sender_node_id: String,
    pub recipient_node_id: String,
    pub ephemeral_pubkey_hex: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
    pub mac_tag_hex: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourierPresence {
    pub courier_node_id: String,
    pub username: String,
    pub x25519_pubkey_hex: String,
    pub lat: f64,
    pub lon: f64,
    pub is_online: bool,
    pub last_ping: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingHub {
    pub hub_id: String,
    pub operator_node_id: String,
    pub name: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    pub hub_fee_num: f64,
    pub holding_capacity: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerNode {
    pub node_id: String,
    pub endpoint_url: String,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub msg_id: String,
    pub origin_node_id: String,
    pub payload_json: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffRecord {
    pub commitment: String,
    pub hop_index: i32,
    pub node_pubkey_hash: String,
    pub event_type: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRating {
    pub rater_node_id: String,
    pub subject_node_id: String,
    pub score: f64,
    pub review_notes: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourierBid {
    pub request_id: String,
    pub courier_node_id: String,
    pub bid_amount: f64,
    pub bid_notes: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeRecord {
    pub request_id: String,
    pub filed_by_node_id: String,
    pub reason: String,
    pub evidence_hash: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentState {
    Created,
    InTransit,
    HandedOff,
    Delivered,
    Completed,
}

impl ShipmentState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShipmentState::Created => "CREATED",
            ShipmentState::InTransit => "IN_TRANSIT",
            ShipmentState::HandedOff => "HANDED_OFF",
            ShipmentState::Delivered => "DELIVERED",
            ShipmentState::Completed => "COMPLETED",
        }
    }
}

pub struct PhysicalHandoffEvent {
    pub receiver_sig: [u8; 64],
}

pub struct PhysicalHandoff;

impl PhysicalHandoff {
    pub fn execute_handoff(
        _commitment: &[u8; 32],
        _giver: &openrelay_crypto::identity::NodeIdentity,
        _receiver: &openrelay_crypto::identity::NodeIdentity,
    ) -> Result<(PhysicalHandoffEvent, ShipmentState), String> {
        Ok((
            PhysicalHandoffEvent { receiver_sig: [1u8; 64] },
            ShipmentState::HandedOff,
        ))
    }
}

pub fn generate_pseudonym() -> String {
    use rand::Rng;
    let adjectives = ["Swift", "Neon", "Cipher", "Silent", "Shadow", "Velvet", "Solar", "Lunar", "Amber", "Cosmic", "Vivid", "Sovereign"];
    let nouns = ["Falcon", "Otter", "Courier", "Runner", "Voyager", "Phoenix", "Prowler", "Drifter", "Relay", "Beacon", "Sentry", "Vanguard"];
    
    let mut rng = rand::thread_rng();
    let adj = adjectives[rng.gen_range(0..adjectives.len())];
    let noun = nouns[rng.gen_range(0..nouns.len())];
    let num: u16 = rng.gen_range(10..99);

    format!("{}{}{}", adj, noun, num)
}

pub fn encrypt_e2ee_message(
    recipient_x25519_pk_hex: &str,
    plaintext: &[u8],
) -> Result<(String, String, String, String), String> {
    use rand::RngCore;

    let recip_pk_bytes = hex::decode(recipient_x25519_pk_hex).map_err(|e| e.to_string())?;
    if recip_pk_bytes.len() != 32 {
        return Err("Invalid recipient public key length".into());
    }
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&recip_pk_bytes);
    let recip_pk = X25519PublicKey::from(pk_arr);

    let mut rng = rand::thread_rng();
    let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);
    
    let shared_secret = ephemeral_secret.diffie_hellman(&recip_pk);

    let mut nonce = [0u8; 16];
    rng.fill_bytes(&mut nonce);

    let mut hasher = blake3::Hasher::new_keyed(shared_secret.as_bytes());
    hasher.update(&nonce);
    let mut output_reader = hasher.finalize_xof();

    let mut ciphertext = plaintext.to_vec();
    let mut keystream = vec![0u8; ciphertext.len()];
    output_reader.fill(&mut keystream);

    for i in 0..ciphertext.len() {
        ciphertext[i] ^= keystream[i];
    }

    let mac_tag = blake3::keyed_hash(shared_secret.as_bytes(), &ciphertext);

    Ok((
        hex::encode(ephemeral_public.as_bytes()),
        hex::encode(nonce),
        hex::encode(ciphertext),
        hex::encode(mac_tag.as_bytes()),
    ))
}

pub fn decrypt_e2ee_message(
    my_x25519_sk_bytes: &[u8; 32],
    ephemeral_pubkey_hex: &str,
    nonce_hex: &str,
    ciphertext_hex: &str,
    mac_tag_hex: &str,
) -> Result<Vec<u8>, String> {
    let eph_bytes = hex::decode(ephemeral_pubkey_hex).map_err(|e| e.to_string())?;
    let nonce = hex::decode(nonce_hex).map_err(|e| e.to_string())?;
    let mut ciphertext = hex::decode(ciphertext_hex).map_err(|e| e.to_string())?;
    let mac_tag = hex::decode(mac_tag_hex).map_err(|e| e.to_string())?;

    let mut eph_arr = [0u8; 32];
    eph_arr.copy_from_slice(&eph_bytes);
    let eph_pub = X25519PublicKey::from(eph_arr);

    let my_sk = X25519StaticSecret::from(*my_x25519_sk_bytes);
    let shared_secret = my_sk.diffie_hellman(&eph_pub);

    let expected_mac = blake3::keyed_hash(shared_secret.as_bytes(), &ciphertext);
    if hex::encode(expected_mac.as_bytes()) != hex::encode(&mac_tag) {
        return Err("Cryptographic Integrity Error: Message MAC tag mismatch".into());
    }

    let mut hasher = blake3::Hasher::new_keyed(shared_secret.as_bytes());
    hasher.update(&nonce);
    let mut output_reader = hasher.finalize_xof();

    let mut keystream = vec![0u8; ciphertext.len()];
    output_reader.fill(&mut keystream);

    for i in 0..ciphertext.len() {
        ciphertext[i] ^= keystream[i];
    }

    Ok(ciphertext)
}

pub fn hash_pin(pin: &str) -> String {
    hex::encode(blake3::hash(pin.as_bytes()).as_bytes())
}

pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().atan2((1.0 - a).sqrt())
}

pub struct StorageEngine {
    pool: sqlx::SqlitePool,
}

impl StorageEngine {
    pub async fn connect(db_url: &str) -> Result<Self, String> {
        let pool = sqlx::SqlitePool::connect(db_url)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query("CREATE TABLE IF NOT EXISTS shipments (commitment TEXT PRIMARY KEY, state TEXT NOT NULL, seal_serial TEXT NOT NULL, updated_at INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS pickup_requests (id TEXT PRIMARY KEY, requester TEXT NOT NULL, target_courier TEXT, staging_hub_id TEXT, hub_fee_num REAL, request_type TEXT NOT NULL, dropoff_mode TEXT NOT NULL, requirements_json TEXT NOT NULL, pin_hash TEXT, pickup_location TEXT NOT NULL, pickup_lat REAL, pickup_lon REAL, item_description TEXT NOT NULL, dropoff_location TEXT NOT NULL, payment_json TEXT NOT NULL, payment_amount_num REAL NOT NULL, requester_confirmed INTEGER NOT NULL DEFAULT 0, courier_confirmed INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL, created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS peers (node_id TEXT PRIMARY KEY, endpoint_url TEXT NOT NULL, last_seen INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS seen_gossip (msg_id TEXT PRIMARY KEY, received_at INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS handoff_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, commitment TEXT NOT NULL, hop_index INTEGER NOT NULL, node_pubkey_hash TEXT NOT NULL, event_type TEXT NOT NULL, timestamp INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS node_ratings (id INTEGER PRIMARY KEY AUTOINCREMENT, rater TEXT NOT NULL, subject TEXT NOT NULL, score REAL NOT NULL, review_notes TEXT NOT NULL, timestamp INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS courier_bids (id INTEGER PRIMARY KEY AUTOINCREMENT, request_id TEXT NOT NULL, courier TEXT NOT NULL, amount REAL NOT NULL, notes TEXT NOT NULL, timestamp INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS disputes (id INTEGER PRIMARY KEY AUTOINCREMENT, request_id TEXT NOT NULL, filer TEXT NOT NULL, reason TEXT NOT NULL, evidence_hash TEXT NOT NULL, timestamp INTEGER NOT NULL);").execute(&pool).await.unwrap();

        sqlx::query("CREATE TABLE IF NOT EXISTS chat_messages (id TEXT PRIMARY KEY, request_id TEXT, sender TEXT NOT NULL, recipient TEXT NOT NULL, eph_pubkey TEXT NOT NULL, nonce TEXT NOT NULL, ciphertext TEXT NOT NULL, mac_tag TEXT NOT NULL, timestamp INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS courier_presence (courier TEXT PRIMARY KEY, username TEXT NOT NULL, x25519_pubkey TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, is_online INTEGER NOT NULL, last_ping INTEGER NOT NULL);").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS staging_hubs (hub_id TEXT PRIMARY KEY, operator TEXT NOT NULL, name TEXT NOT NULL, address TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, hub_fee REAL NOT NULL, capacity INTEGER NOT NULL, created_at INTEGER NOT NULL);").execute(&pool).await.unwrap();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_status ON pickup_requests(status);").execute(&pool).await.unwrap();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_requests_target ON pickup_requests(target_courier);").execute(&pool).await.unwrap();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_recipient ON chat_messages(recipient);").execute(&pool).await.unwrap();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_presence_online ON courier_presence(is_online);").execute(&pool).await.unwrap();

        Ok(Self { pool })
    }

    pub async fn in_memory() -> Result<Self, String> {
        Self::connect("sqlite::memory:").await
    }

    pub async fn save_chat_message(&self, msg: &EncryptedMessage) -> Result<(), String> {
        sqlx::query("INSERT INTO chat_messages (id, request_id, sender, recipient, eph_pubkey, nonce, ciphertext, mac_tag, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);")
            .bind(&msg.id).bind(&msg.request_id).bind(&msg.sender_node_id).bind(&msg.recipient_node_id)
            .bind(&msg.ephemeral_pubkey_hex).bind(&msg.nonce_hex).bind(&msg.ciphertext_hex).bind(&msg.mac_tag_hex).bind(msg.timestamp)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_messages(&self, recipient_node_id: &str, request_id: Option<&str>) -> Result<Vec<EncryptedMessage>, String> {
        use sqlx::Row;
        let query_str = if let Some(req_id) = request_id {
            sqlx::query("SELECT id, request_id, sender, recipient, eph_pubkey, nonce, ciphertext, mac_tag, timestamp FROM chat_messages WHERE request_id = ? ORDER BY timestamp ASC;").bind(req_id)
        } else {
            sqlx::query("SELECT id, request_id, sender, recipient, eph_pubkey, nonce, ciphertext, mac_tag, timestamp FROM chat_messages WHERE recipient = ? OR sender = ? ORDER BY timestamp ASC;").bind(recipient_node_id).bind(recipient_node_id)
        };

        let rows = query_str.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut msgs = Vec::new();
        for row in rows {
            msgs.push(EncryptedMessage {
                id: row.get("id"), request_id: row.get("request_id"), sender_node_id: row.get("sender"), recipient_node_id: row.get("recipient"),
                ephemeral_pubkey_hex: row.get("eph_pubkey"), nonce_hex: row.get("nonce"), ciphertext_hex: row.get("ciphertext"), mac_tag_hex: row.get("mac_tag"), timestamp: row.get("timestamp"),
            });
        }
        Ok(msgs)
    }

    pub async fn update_courier_presence(&self, presence: &CourierPresence) -> Result<(), String> {
        sqlx::query("INSERT INTO courier_presence (courier, username, x25519_pubkey, lat, lon, is_online, last_ping) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(courier) DO UPDATE SET username=excluded.username, x25519_pubkey=excluded.x25519_pubkey, lat=excluded.lat, lon=excluded.lon, is_online=excluded.is_online, last_ping=excluded.last_ping;")
            .bind(&presence.courier_node_id).bind(&presence.username).bind(&presence.x25519_pubkey_hex)
            .bind(presence.lat).bind(presence.lon).bind(if presence.is_online { 1 } else { 0 }).bind(presence.last_ping)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_online_couriers(&self, user_lat: f64, user_lon: f64, max_distance_km: f64) -> Result<Vec<CourierPresence>, String> {
        use sqlx::Row;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let threshold = now - 300;

        let rows = sqlx::query("SELECT courier, username, x25519_pubkey, lat, lon, is_online, last_ping FROM courier_presence WHERE is_online = 1 AND last_ping >= ?;")
            .bind(threshold)
            .fetch_all(&self.pool).await.map_err(|e| e.to_string())?;

        let mut couriers = Vec::new();
        for row in rows {
            let c_lat: f64 = row.get("lat");
            let c_lon: f64 = row.get("lon");
            if haversine_km(user_lat, user_lon, c_lat, c_lon) <= max_distance_km {
                couriers.push(CourierPresence {
                    courier_node_id: row.get("courier"), username: row.get("username"), x25519_pubkey_hex: row.get("x25519_pubkey"),
                    lat: c_lat, lon: c_lon, is_online: true, last_ping: row.get("last_ping"),
                });
            }
        }
        Ok(couriers)
    }

    pub async fn confirm_settlement(&self, request_id: &str, is_requester: bool) -> Result<(bool, bool), String> {
        use sqlx::Row;
        if is_requester {
            sqlx::query("UPDATE pickup_requests SET requester_confirmed = 1 WHERE id = ?;").bind(request_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        } else {
            sqlx::query("UPDATE pickup_requests SET courier_confirmed = 1 WHERE id = ?;").bind(request_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        }

        let row = sqlx::query("SELECT requester_confirmed, courier_confirmed FROM pickup_requests WHERE id = ?;").bind(request_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        let req_conf: bool = row.get::<i32, _>("requester_confirmed") == 1;
        let cour_conf: bool = row.get::<i32, _>("courier_confirmed") == 1;

        if req_conf && cour_conf {
            let mut payment_json_val: serde_json::Value = serde_json::from_str(&sqlx::query("SELECT payment_json FROM pickup_requests WHERE id = ?;").bind(request_id).fetch_one(&self.pool).await.unwrap().get::<String, _>("payment_json")).unwrap();
            payment_json_val["is_settled"] = serde_json::json!(true);

            sqlx::query("UPDATE pickup_requests SET status = 'COMPLETED', payment_json = ? WHERE id = ?;")
                .bind(serde_json::to_string(&payment_json_val).unwrap())
                .bind(request_id)
                .execute(&self.pool).await.map_err(|e| e.to_string())?;
        }

        Ok((req_conf, cour_conf))
    }

    pub async fn register_staging_hub(&self, hub: &StagingHub) -> Result<(), String> {
        sqlx::query("INSERT INTO staging_hubs (hub_id, operator, name, address, lat, lon, hub_fee, capacity, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);")
            .bind(&hub.hub_id).bind(&hub.operator_node_id).bind(&hub.name).bind(&hub.address).bind(hub.lat).bind(hub.lon).bind(hub.hub_fee_num).bind(hub.holding_capacity).bind(hub.created_at)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_nearby_hubs(&self, user_lat: f64, user_lon: f64, max_dist_km: f64) -> Result<Vec<StagingHub>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT hub_id, operator, name, address, lat, lon, hub_fee, capacity, created_at FROM staging_hubs;").fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut hubs = Vec::new();
        for row in rows {
            let h_lat: f64 = row.get("lat");
            let h_lon: f64 = row.get("lon");
            if haversine_km(user_lat, user_lon, h_lat, h_lon) <= max_dist_km {
                hubs.push(StagingHub {
                    hub_id: row.get("hub_id"), operator_node_id: row.get("operator"), name: row.get("name"), address: row.get("address"),
                    lat: h_lat, lon: h_lon, hub_fee_num: row.get("hub_fee"), holding_capacity: row.get("capacity"), created_at: row.get("created_at"),
                });
            }
        }
        Ok(hubs)
    }

    pub async fn cancel_request(&self, request_id: &str) -> Result<(), String> {
        use sqlx::Row;
        let row = sqlx::query("SELECT status FROM pickup_requests WHERE id = ?").bind(request_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        if let Some(r) = row {
            if r.get::<String, _>("status") != "PENDING" { return Err("Only PENDING requests can be cancelled".into()); }
            sqlx::query("UPDATE pickup_requests SET status = 'CANCELLED' WHERE id = ?").bind(request_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
            Ok(())
        } else { Err("Request not found".into()) }
    }

    pub async fn register_peer(&self, peer: &PeerNode) -> Result<(), String> {
        sqlx::query("INSERT INTO peers (node_id, endpoint_url, last_seen) VALUES (?, ?, ?) ON CONFLICT(node_id) DO UPDATE SET endpoint_url=excluded.endpoint_url, last_seen=excluded.last_seen;").bind(&peer.node_id).bind(&peer.endpoint_url).bind(peer.last_seen).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_peers(&self) -> Result<Vec<PeerNode>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT node_id, endpoint_url, last_seen FROM peers").fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|row| PeerNode { node_id: row.get("node_id"), endpoint_url: row.get("endpoint_url"), last_seen: row.get("last_seen") }).collect())
    }

    pub async fn record_gossip_seen(&self, msg_id: &str) -> Result<bool, String> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let res = sqlx::query("INSERT OR IGNORE INTO seen_gossip (msg_id, received_at) VALUES (?, ?);").bind(msg_id).bind(now).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn save_shipment(&self, commitment: &str, state: ShipmentState, seal_serial: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        sqlx::query("INSERT INTO shipments (commitment, state, seal_serial, updated_at) VALUES (?, ?, ?, ?) ON CONFLICT(commitment) DO UPDATE SET state=excluded.state, updated_at=excluded.updated_at;").bind(commitment).bind(state.as_str()).bind(seal_serial).bind(now).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn record_handoff_event(&self, record: &HandoffRecord) -> Result<(), String> {
        sqlx::query("INSERT INTO handoff_logs (commitment, hop_index, node_pubkey_hash, event_type, timestamp) VALUES (?, ?, ?, ?, ?);").bind(&record.commitment).bind(record.hop_index).bind(&record.node_pubkey_hash).bind(&record.event_type).bind(record.timestamp).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_handoff_history(&self, commitment: &str) -> Result<Vec<HandoffRecord>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT commitment, hop_index, node_pubkey_hash, event_type, timestamp FROM handoff_logs WHERE commitment = ? ORDER BY hop_index ASC, timestamp ASC;").bind(commitment).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|row| HandoffRecord { commitment: row.get("commitment"), hop_index: row.get("hop_index"), node_pubkey_hash: row.get("node_pubkey_hash"), event_type: row.get("event_type"), timestamp: row.get("timestamp") }).collect())
    }

    pub async fn save_rating(&self, rating: &NodeRating) -> Result<(), String> {
        sqlx::query("INSERT INTO node_ratings (rater, subject, score, review_notes, timestamp) VALUES (?, ?, ?, ?, ?);").bind(&rating.rater_node_id).bind(&rating.subject_node_id).bind(rating.score).bind(&rating.review_notes).bind(rating.timestamp).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn save_bid(&self, bid: &CourierBid) -> Result<(), String> {
        sqlx::query("INSERT INTO courier_bids (request_id, courier, amount, notes, timestamp) VALUES (?, ?, ?, ?, ?);").bind(&bid.request_id).bind(&bid.courier_node_id).bind(bid.bid_amount).bind(&bid.bid_notes).bind(bid.timestamp).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_bids_for_request(&self, request_id: &str) -> Result<Vec<CourierBid>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT request_id, courier, amount, notes, timestamp FROM courier_bids WHERE request_id = ? ORDER BY amount ASC;").bind(request_id).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|row| CourierBid { request_id: row.get("request_id"), courier_node_id: row.get("courier"), bid_amount: row.get("amount"), bid_notes: row.get("notes"), timestamp: row.get("timestamp") }).collect())
    }

    pub async fn accept_bid(&self, request_id: &str, courier_node_id: &str) -> Result<(), String> {
        sqlx::query("UPDATE pickup_requests SET status = 'CLAIMED' WHERE id = ? AND status = 'PENDING';").bind(request_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        self.record_handoff_event(&HandoffRecord { commitment: request_id.to_string(), hop_index: 1, node_pubkey_hash: courier_node_id.to_string(), event_type: "BID_ACCEPTED_ASSIGNED".into(), timestamp: now }).await?;
        Ok(())
    }

    pub async fn file_dispute(&self, dispute: &DisputeRecord) -> Result<(), String> {
        sqlx::query("INSERT INTO disputes (request_id, filer, reason, evidence_hash, timestamp) VALUES (?, ?, ?, ?, ?);").bind(&dispute.request_id).bind(&dispute.filed_by_node_id).bind(&dispute.reason).bind(&dispute.evidence_hash).bind(dispute.timestamp).execute(&self.pool).await.map_err(|e| e.to_string())?;
        sqlx::query("UPDATE pickup_requests SET status = 'DISPUTED' WHERE id = ?;").bind(&dispute.request_id).execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn check_dispute_rate_limit(&self, node_id: &str, time_window_seconds: i64) -> Result<i64, String> {
        use sqlx::Row;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let row = sqlx::query("SELECT COUNT(*) as count FROM disputes WHERE filer = ? AND timestamp > ?").bind(node_id).bind(now - time_window_seconds).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.get("count"))
    }

    pub async fn has_existing_dispute(&self, request_id: &str, node_id: &str) -> Result<bool, String> {
        use sqlx::Row;
        let row = sqlx::query("SELECT COUNT(*) as count FROM disputes WHERE request_id = ? AND filer = ?").bind(request_id).bind(node_id).fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(row.get::<i64, _>("count") > 0)
    }

    pub async fn create_pickup_request(&self, req: &PickupRequest) -> Result<(), String> {
        let payment_json = serde_json::to_string(&req.payment_spec).unwrap();
        let req_json = serde_json::to_string(&req.requirements).unwrap();
        sqlx::query("INSERT INTO pickup_requests (id, requester, target_courier, staging_hub_id, hub_fee_num, request_type, dropoff_mode, requirements_json, pin_hash, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, requester_confirmed, courier_confirmed, status, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);")
            .bind(&req.id).bind(&req.requester_node_id).bind(&req.target_courier_id).bind(&req.staging_hub_id).bind(req.hub_fee_num)
            .bind(req.request_type.as_str()).bind(req.dropoff_mode.as_str()).bind(req_json).bind(&req.pin_hash)
            .bind(&req.pickup_location).bind(req.pickup_lat).bind(req.pickup_lon).bind(&req.item_description).bind(&req.dropoff_location)
            .bind(payment_json).bind(req.payment_amount_num).bind(if req.requester_confirmed { 1 } else { 0 }).bind(if req.courier_confirmed { 1 } else { 0 })
            .bind(req.status.as_str()).bind(req.created_at).bind(req.expires_at)
            .execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn fetch_request_by_id(&self, id: &str) -> Result<Option<PickupRequest>, String> {
        use sqlx::Row;
        let row_opt = sqlx::query("SELECT id, requester, target_courier, staging_hub_id, hub_fee_num, request_type, dropoff_mode, requirements_json, pin_hash, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, requester_confirmed, courier_confirmed, status, created_at, expires_at FROM pickup_requests WHERE id = ?").bind(id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        if let Some(row) = row_opt {
            let payment_spec: PaymentSpec = serde_json::from_str(&row.get::<String, _>("payment_json")).unwrap();
            let requirements: CourierRequirements = serde_json::from_str(&row.get::<String, _>("requirements_json")).unwrap();
            Ok(Some(PickupRequest {
                id: row.get("id"), requester_node_id: row.get("requester"), target_courier_id: row.get("target_courier"), staging_hub_id: row.get("staging_hub_id"), hub_fee_num: row.get("hub_fee_num"),
                request_type: RequestType::from_str(&row.get::<String, _>("request_type")), dropoff_mode: DropoffMode::from_str(&row.get::<String, _>("dropoff_mode")), requirements,
                pin_hash: row.get("pin_hash"), pickup_location: row.get("pickup_location"), pickup_lat: row.get("pickup_lat"), pickup_lon: row.get("pickup_lon"),
                item_description: row.get("item_description"), dropoff_location: row.get("dropoff_location"), payment_spec, payment_amount_num: row.get("payment_amount_num"),
                requester_confirmed: row.get::<i32, _>("requester_confirmed") == 1, courier_confirmed: row.get::<i32, _>("courier_confirmed") == 1,
                status: RequestStatus::from_str(&row.get::<String, _>("status")), created_at: row.get("created_at"), expires_at: row.get("expires_at"),
            }))
        } else { Ok(None) }
    }

    pub async fn verify_and_complete_request(&self, request_id: &str, provided_pin: Option<&str>) -> Result<bool, String> {
        use sqlx::Row;
        let row = sqlx::query("SELECT dropoff_mode, pin_hash FROM pickup_requests WHERE id = ?").bind(request_id).fetch_optional(&self.pool).await.map_err(|e| e.to_string())?;
        if let Some(r) = row {
            let mode = DropoffMode::from_str(&r.get::<String, _>("dropoff_mode"));
            let stored_pin_hash: Option<String> = r.get("pin_hash");
            if mode == DropoffMode::InPersonHandoff {
                match (provided_pin, stored_pin_hash) {
                    (Some(pin), Some(expected_hash)) => if hash_pin(pin) != expected_hash { return Ok(false); },
                    _ => return Ok(false),
                }
            }
            Ok(true)
        } else { Err("Request not found".into()) }
    }

    pub async fn fetch_pending_requests(&self) -> Result<Vec<PickupRequest>, String> {
        use sqlx::Row;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        sqlx::query("UPDATE pickup_requests SET status = 'CANCELLED' WHERE status = 'PENDING' AND expires_at < ?").bind(now).execute(&self.pool).await.unwrap();
        let rows = sqlx::query("SELECT id, requester, target_courier, staging_hub_id, hub_fee_num, request_type, dropoff_mode, requirements_json, pin_hash, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, requester_confirmed, courier_confirmed, status, created_at, expires_at FROM pickup_requests WHERE status = 'PENDING'").fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for row in rows {
            let payment_spec: PaymentSpec = serde_json::from_str(&row.get::<String, _>("payment_json")).unwrap();
            let requirements: CourierRequirements = serde_json::from_str(&row.get::<String, _>("requirements_json")).unwrap();
            results.push(PickupRequest {
                id: row.get("id"), requester_node_id: row.get("requester"), target_courier_id: row.get("target_courier"), staging_hub_id: row.get("staging_hub_id"), hub_fee_num: row.get("hub_fee_num"),
                request_type: RequestType::from_str(&row.get::<String, _>("request_type")), dropoff_mode: DropoffMode::from_str(&row.get::<String, _>("dropoff_mode")), requirements,
                pin_hash: row.get("pin_hash"), pickup_location: row.get("pickup_location"), pickup_lat: row.get("pickup_lat"), pickup_lon: row.get("pickup_lon"),
                item_description: row.get("item_description"), dropoff_location: row.get("dropoff_location"), payment_spec, payment_amount_num: row.get("payment_amount_num"),
                requester_confirmed: row.get::<i32, _>("requester_confirmed") == 1, courier_confirmed: row.get::<i32, _>("courier_confirmed") == 1,
                status: RequestStatus::from_str(&row.get::<String, _>("status")), created_at: row.get("created_at"), expires_at: row.get("expires_at"),
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2ee_encryption_roundtrip() {
        let sk_bytes = [7u8; 32];
        let static_sk = X25519StaticSecret::from(sk_bytes);
        let static_pk = X25519PublicKey::from(&static_sk);
        let pk_hex = hex::encode(static_pk.as_bytes());

        let plaintext = b"Secret offer: $25 to pickup camera";
        let (eph_pk_hex, nonce_hex, cipher_hex, tag_hex) = encrypt_e2ee_message(&pk_hex, plaintext).unwrap();

        let decrypted = decrypt_e2ee_message(&sk_bytes, &eph_pk_hex, &nonce_hex, &cipher_hex, &tag_hex).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());
    }
}
