use openrelay_crypto::identity::NodeIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    CashOnHandoff,
    P2PFiat { provider: String, handle: String },
    Crypto { network: String, address_or_invoice: String },
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSpec {
    pub amount_prompt: String,
    pub accepted_methods: Vec<PaymentMethod>,
    pub is_settled: bool,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupRequest {
    pub id: String,
    pub requester_node_id: String,
    pub pickup_location: String,
    pub item_description: String,
    pub dropoff_location: String,
    pub payment_spec: PaymentSpec,
    pub status: RequestStatus,
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
                pickup_location TEXT NOT NULL,
                item_description TEXT NOT NULL,
                dropoff_location TEXT NOT NULL,
                payment_json TEXT NOT NULL,
                status TEXT NOT NULL,
                updated_at INTEGER NOT NULL
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

    pub async fn create_pickup_request(&self, req: &PickupRequest) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let payment_json = serde_json::to_string(&req.payment_spec).map_err(|e| e.to_string())?;

        sqlx::query(
            "INSERT INTO pickup_requests (id, requester, pickup_location, item_description, dropoff_location, payment_json, status, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?);"
        )
        .bind(&req.id)
        .bind(&req.requester_node_id)
        .bind(&req.pickup_location)
        .bind(&req.item_description)
        .bind(&req.dropoff_location)
        .bind(payment_json)
        .bind(req.status.as_str())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_persistence() {
        let storage = StorageEngine::in_memory().await.unwrap();
        assert!(storage.save_shipment("0x123", ShipmentState::InTransit, "SEAL-999").await.is_ok());

        let req = PickupRequest {
            id: "REQ-001".into(),
            requester_node_id: "OR1:TEST".into(),
            pickup_location: "123 Store St".into(),
            item_description: "Order #42".into(),
            dropoff_location: "456 Home Ave".into(),
            payment_spec: PaymentSpec {
                amount_prompt: "$10.00".into(),
                accepted_methods: vec![PaymentMethod::CashOnHandoff],
                is_settled: false,
            },
            status: RequestStatus::Pending,
        };

        assert!(storage.create_pickup_request(&req).await.is_ok());
    }
}
