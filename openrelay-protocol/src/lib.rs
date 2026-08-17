use openrelay_crypto::identity::NodeIdentity;
use serde::{Deserialize, Serialize};

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
            RequestType::CustomTask => "CUSTOM_TASK",
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
}

impl RequestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestStatus::Pending => "PENDING",
            RequestStatus::Claimed => "CLAIMED",
            RequestStatus::Completed => "COMPLETED",
            RequestStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CLAIMED" => RequestStatus::Claimed,
            "COMPLETED" => RequestStatus::Completed,
            "CANCELLED" => RequestStatus::Cancelled,
            _ => RequestStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupRequest {
    pub id: String,
    pub requester_node_id: String,
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
    pub status: RequestStatus,
    pub created_at: i64,
    pub expires_at: i64,
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
        _giver: &NodeIdentity,
        _receiver: &NodeIdentity,
    ) -> Result<(PhysicalHandoffEvent, ShipmentState), String> {
        Ok((
            PhysicalHandoffEvent { receiver_sig: [1u8; 64] },
            ShipmentState::HandedOff,
        ))
    }
}

pub fn hash_pin(pin: &str) -> String {
    hex::encode(blake3::hash(pin.as_bytes()).as_bytes())
}

pub struct StorageEngine {
    pool: sqlx::SqlitePool,
}

impl StorageEngine {
    pub async fn in_memory() -> Result<Self, String> {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS shipments (
                commitment TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                seal_serial TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );"
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS pickup_requests (
                id TEXT PRIMARY KEY,
                requester TEXT NOT NULL,
                request_type TEXT NOT NULL,
                dropoff_mode TEXT NOT NULL,
                requirements_json TEXT NOT NULL,
                pin_hash TEXT,
                pickup_location TEXT NOT NULL,
                pickup_lat REAL,
                pickup_lon REAL,
                item_description TEXT NOT NULL,
                dropoff_location TEXT NOT NULL,
                payment_json TEXT NOT NULL,
                payment_amount_num REAL NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );"
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS peers (
                node_id TEXT PRIMARY KEY,
                endpoint_url TEXT NOT NULL,
                last_seen INTEGER NOT NULL
            );"
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS seen_gossip (
                msg_id TEXT PRIMARY KEY,
                received_at INTEGER NOT NULL
            );"
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS handoff_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                commitment TEXT NOT NULL,
                hop_index INTEGER NOT NULL,
                node_pubkey_hash TEXT NOT NULL,
                event_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );"
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(Self { pool })
    }

    pub async fn register_peer(&self, peer: &PeerNode) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO peers (node_id, endpoint_url, last_seen)
             VALUES (?, ?, ?)
             ON CONFLICT(node_id) DO UPDATE SET endpoint_url=excluded.endpoint_url, last_seen=excluded.last_seen;"
        )
        .bind(&peer.node_id)
        .bind(&peer.endpoint_url)
        .bind(peer.last_seen)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn fetch_peers(&self) -> Result<Vec<PeerNode>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT node_id, endpoint_url, last_seen FROM peers")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let mut peers = Vec::new();
        for row in rows {
            peers.push(PeerNode {
                node_id: row.get("node_id"),
                endpoint_url: row.get("endpoint_url"),
                last_seen: row.get("last_seen"),
            });
        }
        Ok(peers)
    }

    pub async fn record_gossip_seen(&self, msg_id: &str) -> Result<bool, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let res = sqlx::query("INSERT OR IGNORE INTO seen_gossip (msg_id, received_at) VALUES (?, ?);")
            .bind(msg_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn save_shipment(&self, commitment: &str, state: ShipmentState, seal_serial: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query(
            "INSERT INTO shipments (commitment, state, seal_serial, updated_at) 
             VALUES (?, ?, ?, ?)
             ON CONFLICT(commitment) DO UPDATE SET state=excluded.state, updated_at=excluded.updated_at;"
        )
        .bind(commitment)
        .bind(state.as_str())
        .bind(seal_serial)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn record_handoff_event(&self, record: &HandoffRecord) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO handoff_logs (commitment, hop_index, node_pubkey_hash, event_type, timestamp)
             VALUES (?, ?, ?, ?, ?);"
        )
        .bind(&record.commitment)
        .bind(record.hop_index)
        .bind(&record.node_pubkey_hash)
        .bind(&record.event_type)
        .bind(record.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn fetch_handoff_history(&self, commitment: &str) -> Result<Vec<HandoffRecord>, String> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT commitment, hop_index, node_pubkey_hash, event_type, timestamp 
             FROM handoff_logs WHERE commitment = ? ORDER BY hop_index ASC, timestamp ASC;"
        )
        .bind(commitment)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let mut history = Vec::new();
        for row in rows {
            history.push(HandoffRecord {
                commitment: row.get("commitment"),
                hop_index: row.get("hop_index"),
                node_pubkey_hash: row.get("node_pubkey_hash"),
                event_type: row.get("event_type"),
                timestamp: row.get("timestamp"),
            });
        }

        Ok(history)
    }

    pub async fn create_pickup_request(&self, req: &PickupRequest) -> Result<(), String> {
        let payment_json = serde_json::to_string(&req.payment_spec).map_err(|e| e.to_string())?;
        let req_json = serde_json::to_string(&req.requirements).map_err(|e| e.to_string())?;

        sqlx::query(
            "INSERT INTO pickup_requests (id, requester, request_type, dropoff_mode, requirements_json, pin_hash, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, status, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"
        )
        .bind(&req.id)
        .bind(&req.requester_node_id)
        .bind(req.request_type.as_str())
        .bind(req.dropoff_mode.as_str())
        .bind(req_json)
        .bind(&req.pin_hash)
        .bind(&req.pickup_location)
        .bind(req.pickup_lat)
        .bind(req.pickup_lon)
        .bind(&req.item_description)
        .bind(&req.dropoff_location)
        .bind(payment_json)
        .bind(req.payment_amount_num)
        .bind(req.status.as_str())
        .bind(req.created_at)
        .bind(req.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn verify_and_complete_request(&self, request_id: &str, provided_pin: Option<&str>) -> Result<bool, String> {
        use sqlx::Row;
        let row = sqlx::query("SELECT dropoff_mode, pin_hash FROM pickup_requests WHERE id = ?")
            .bind(request_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(r) = row {
            let dropoff_mode_str: String = r.get("dropoff_mode");
            let mode = DropoffMode::from_str(&dropoff_mode_str);
            let stored_pin_hash: Option<String> = r.get("pin_hash");

            if mode == DropoffMode::InPersonHandoff {
                match (provided_pin, stored_pin_hash) {
                    (Some(pin), Some(expected_hash)) => {
                        if hash_pin(pin) != expected_hash {
                            return Ok(false);
                        }
                    }
                    _ => return Ok(false),
                }
            }

            sqlx::query("UPDATE pickup_requests SET status = 'COMPLETED' WHERE id = ?")
                .bind(request_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            Ok(true)
        } else {
            Err("Request not found".into())
        }
    }

    pub async fn fetch_pending_requests(&self) -> Result<Vec<PickupRequest>, String> {
        use sqlx::Row;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query("UPDATE pickup_requests SET status = 'CANCELLED' WHERE status = 'PENDING' AND expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let rows = sqlx::query("SELECT id, requester, request_type, dropoff_mode, requirements_json, pin_hash, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, status, created_at, expires_at FROM pickup_requests WHERE status = 'PENDING'")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            let payment_json: String = row.get("payment_json");
            let req_json: String = row.get("requirements_json");
            let payment_spec: PaymentSpec = serde_json::from_str(&payment_json).map_err(|e| e.to_string())?;
            let requirements: CourierRequirements = serde_json::from_str(&req_json).unwrap_or(CourierRequirements {
                min_rating: 0.0,
                require_insulated_bag: false,
                required_vehicle: VehicleType::Any,
            });

            let request_type_str: String = row.get("request_type");
            let dropoff_mode_str: String = row.get("dropoff_mode");
            let status_str: String = row.get("status");

            results.push(PickupRequest {
                id: row.get("id"),
                requester_node_id: row.get("requester"),
                request_type: RequestType::from_str(&request_type_str),
                dropoff_mode: DropoffMode::from_str(&dropoff_mode_str),
                requirements,
                pin_hash: row.get("pin_hash"),
                pickup_location: row.get("pickup_location"),
                pickup_lat: row.get("pickup_lat"),
                pickup_lon: row.get("pickup_lon"),
                item_description: row.get("item_description"),
                dropoff_location: row.get("dropoff_location"),
                payment_spec,
                payment_amount_num: row.get("payment_amount_num"),
                status: RequestStatus::from_str(&status_str),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
            });
        }

        Ok(results)
    }
}

pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_peer_registration() {
        let storage = StorageEngine::in_memory().await.unwrap();

        let peer = PeerNode {
            node_id: "OR1:PEER_NODE_ABC".into(),
            endpoint_url: "http://192.168.1.50:8080".into(),
            last_seen: 10000,
        };

        storage.register_peer(&peer).await.unwrap();
        let peers = storage.fetch_peers().await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "OR1:PEER_NODE_ABC");
    }
}
