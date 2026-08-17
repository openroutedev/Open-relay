use openrelay_crypto::identity::NodeIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    CashOnHandoff,
    P2PFiat { provider: String, handle: String }, // e.g., Venmo, Cash App, Zelle
    Crypto { network: String, address_or_invoice: String }, // e.g., Lightning, Solana, USDC
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentSpec {
    pub amount_prompt: String,           // e.g., "$15.00" or "0.0003 BTC"
    pub accepted_methods: Vec<PaymentMethod>,
    pub is_settled: bool,                 // Marked during physical handoff
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_persistence() {
        let storage = StorageEngine::in_memory().await.unwrap();
        assert!(storage.save_shipment("0x123", ShipmentState::InTransit, "SEAL-999").await.is_ok());
    }

    #[test]
    fn test_payment_spec_serialization() {
        let spec = PaymentSpec {
            amount_prompt: "$15.00".to_string(),
            accepted_methods: vec![
                PaymentMethod::CashOnHandoff,
                PaymentMethod::P2PFiat {
                    provider: "Venmo".to_string(),
                    handle: "@relay_user".to_string(),
                },
            ],
            is_settled: false,
        };

        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: PaymentSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, deserialized);
    }
}
