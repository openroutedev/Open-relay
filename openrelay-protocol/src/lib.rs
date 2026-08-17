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
    pub pickup_location: String,
    pub pickup_lat: Option<f64>,
    pub pickup_lon: Option<f64>,
    pub item_description: String,
    pub dropoff_location: String,
    pub payment_spec: PaymentSpec,
    pub payment_amount_num: f64,
    pub status: RequestStatus,
    pub created_at: i64,
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
                pickup_location TEXT NOT NULL,
                pickup_lat REAL,
                pickup_lon REAL,
                item_description TEXT NOT NULL,
                dropoff_location TEXT NOT NULL,
                payment_json TEXT NOT NULL,
                payment_amount_num REAL NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL
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

        sqlx::query(
            "INSERT INTO pickup_requests (id, requester, request_type, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);"
        )
        .bind(&req.id)
        .bind(&req.requester_node_id)
        .bind(req.request_type.as_str())
        .bind(&req.pickup_location)
        .bind(req.pickup_lat)
        .bind(req.pickup_lon)
        .bind(&req.item_description)
        .bind(&req.dropoff_location)
        .bind(payment_json)
        .bind(req.payment_amount_num)
        .bind(req.status.as_str())
        .bind(req.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn fetch_pending_requests(&self) -> Result<Vec<PickupRequest>, String> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT id, requester, request_type, pickup_location, pickup_lat, pickup_lon, item_description, dropoff_location, payment_json, payment_amount_num, status, created_at FROM pickup_requests WHERE status = 'PENDING'")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            let payment_json: String = row.get("payment_json");
            let payment_spec: PaymentSpec = serde_json::from_str(&payment_json).map_err(|e| e.to_string())?;
            let request_type_str: String = row.get("request_type");
            let status_str: String = row.get("status");

            results.push(PickupRequest {
                id: row.get("id"),
                requester_node_id: row.get("requester"),
                request_type: RequestType::from_str(&request_type_str),
                pickup_location: row.get("pickup_location"),
                pickup_lat: row.get("pickup_lat"),
                pickup_lon: row.get("pickup_lon"),
                item_description: row.get("item_description"),
                dropoff_location: row.get("dropoff_location"),
                payment_spec,
                payment_amount_num: row.get("payment_amount_num"),
                status: RequestStatus::from_str(&status_str),
                created_at: row.get("created_at"),
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
    async fn test_handoff_audit_trail() {
        let storage = StorageEngine::in_memory().await.unwrap();

        let record1 = HandoffRecord {
            commitment: "0xa1b2".into(),
            hop_index: 0,
            node_pubkey_hash: "OR1:HASH_ORIGIN".into(),
            event_type: "PACKAGE_CREATED".into(),
            timestamp: 1000,
        };

        let record2 = HandoffRecord {
            commitment: "0xa1b2".into(),
            hop_index: 1,
            node_pubkey_hash: "OR1:HASH_RELAY1".into(),
            event_type: "HANDOFF_EXECUTED".into(),
            timestamp: 1050,
        };

        storage.record_handoff_event(&record1).await.unwrap();
        storage.record_handoff_event(&record2).await.unwrap();

        let history = storage.fetch_handoff_history("0xa1b2").await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].event_type, "PACKAGE_CREATED");
        assert_eq!(history[1].event_type, "HANDOFF_EXECUTED");
    }
}
