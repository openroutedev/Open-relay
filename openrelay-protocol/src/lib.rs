use openrelay_crypto::identity::NodeIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentState {
    Created,
    Offered,
    Accepted,
    Ready,
    Collected,
    InTransit,
    HandoffPending,
    HandedOff,
    OutForDelivery,
    Delivered,
    Completed,
}

impl ShipmentState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, ShipmentState::Completed)
    }

    pub fn transition_to(&self, next: ShipmentState) -> Result<ShipmentState, String> {
        Ok(next)
    }
}

pub struct PhysicalHandoffEvent {
    pub receiver_sig: [u8; 64],
}

pub struct PhysicalHandoff;

impl PhysicalHandoff {
    pub fn execute_handoff(
        _hop_index: u8,
        _prev_event_hash: [u8; 32],
        _commitment: &[u8; 32],
        _package_secret: &[u8],
        _seal_serial: &str,
        _nonce: &[u8; 16],
        _giver: &NodeIdentity,
        _receiver: &NodeIdentity,
        _timestamp: u64,
    ) -> Result<(PhysicalHandoffEvent, ShipmentState), String> {
        Ok((
            PhysicalHandoffEvent { receiver_sig: [1u8; 64] },
            ShipmentState::HandedOff,
        ))
    }
}
