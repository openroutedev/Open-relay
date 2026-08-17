use std::collections::HashMap;

pub struct EscrowAdapter {
    pub collateral_ratio: f64,
    pub deposits: HashMap<String, u64>,
    pub locked: HashMap<String, u64>,
}

impl EscrowAdapter {
    pub fn new(collateral_ratio: f64) -> Self {
        Self {
            collateral_ratio,
            deposits: HashMap::new(),
            locked: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, node_id: &str, amount: u64) {
        *self.deposits.entry(node_id.to_string()).or_insert(0) += amount;
    }

    pub fn lock_collateral(&mut self, node_id: &str, declared_value: u64) -> Result<(), String> {
        let required = (declared_value as f64 * self.collateral_ratio) as u64;
        let available = self.deposits.get(node_id).copied().unwrap_or(0);
        if available >= required {
            self.locked.insert(node_id.to_string(), required);
            Ok(())
        } else {
            Err("Insufficient bond deposit".to_string())
        }
    }
}
